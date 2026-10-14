//! `use cargo regex` / `use crate "regex"`: a crate from crates.io as a WebAssembly component (notes/cargo_crates.md).
//! A crate has no stable ABI and no WIT of its own, so warp generates a wrapper crate: rustdoc's JSON (RUSTC_BOOTSTRAP
//! on any toolchain) lists the crate's public functions and the structs it constructs; every one whose types map to
//! WIT is exported (a struct as a resource), built for wasm32-wasip2 with wit-bindgen, and what the compiler refuses
//! is dropped by the lines its errors name. The component lands in ~/.cache/warp/crates/<crate>@<version>.wasm (with
//! its .wit), once per machine; `use wasm` loads it from there (src/modules/components.rs). Common crates are prebuilt by CI
//! and downloaded instead (release crates-<generation>), so most users need no cargo.
//! The wrappers are members of one cargo workspace (~/.cache/warp/crates/Cargo.toml), so a configured target-dir is
//! honoured and a user without one gets that workspace's single target/.
use crate::modules::package_tools::MISSING_TARGET_SIGN;
use crate::modules::package_tools::{download_bytes, write_atomically};
use crate::versions::Version;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

mod compile;
mod rustdoc;
mod wrapper;
use compile::*;
use rustdoc::*;

const CACHE: &str = ".cache/warp/crates";
const WRAPPERS: &str = "wrappers";
/// outside the workspace members: the wrappers that did not build
const FAILED: &str = "failed";
/// raise when the generated wrappers change: the components of older generators stay unused (components-<generation>,
/// and the release crates-<generation> that hosts prebuilt ones)
const GENERATION: &str = "v4";
/// prebuilt components: the assets `<crate>.<version>.wasm` (and `.debug.wasm`) of this release, built by .github/workflows/crates.yml
const HOSTED_RELEASES: &str = "https://api.github.com/repos/pannous/warp/releases/tags/crates-";
/// set: build every crate locally, never download (the workflow that builds the hosted ones)
const BUILD_LOCALLY: &str = "WARP_BUILD_CRATES";
/// set: the debug build of each crate component, with DWARF (file and line in traps; 10 to 45 times the size)
const DEBUG_BUILDS: &str = "WARP_DEBUG_CRATES";
const SHA256_PREFIX: &str = "sha256:";
/// release keeps the name section (`strip = "debuginfo"`): traps name the Rust functions, for about 20% more bytes
const WORKSPACE_MANIFEST: &str = "[workspace]\nmembers = [\"wrappers/*\"]\nresolver = \"2\"\n\n[profile.release]\nopt-level = \"s\"\nstrip = \"debuginfo\"\n\n[profile.debugging]\ninherits = \"release\"\ndebug = true\nstrip = false\n";
const WIT_BINDGEN: &str = "0.46";
const TARGET: &str = "wasm32-wasip2";
const WRAPPER_PREFIX: &str = "warp-crate-";
/// rounds of dropping the functions the compiler refused
const BUILD_ROUNDS: usize = 12;
/// one wrapper build at a time within a process; cargo locks the target directory across processes
static BUILDING: Mutex<()> = Mutex::new(());

/// A cargo profile of the wrappers and the file name ending of its components
struct Variant {
	profile: &'static str,
	ending: &'static str,
}
const RELEASE: Variant = Variant { profile: "release", ending: ".wasm" };
const DEBUG: Variant = Variant { profile: "debugging", ending: ".debug.wasm" };

fn variant() -> &'static Variant {
	match std::env::var_os(DEBUG_BUILDS) {
		Some(_) => &DEBUG,
		None => &RELEASE,
	}
}

/// The version in a component file name `<crate><separator><version><ending>` of the variant
fn version_in<'a>(file: &'a str, name: &str, separator: char) -> Option<&'a str> {
	let version = file.strip_prefix(name)?.strip_prefix(separator)?.strip_suffix(variant().ending)?;
	(!version.ends_with(".debug")).then_some(version)
}

/// The component file of the crate `name`: the newest cached build, else the newest hosted one, else generated and
/// built now
pub fn component(name: &str) -> Result<String, String> {
	if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
		return Err(format!("{name:?} is no crate name"));
	}
	let _building = BUILDING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let path = match newest_cached(name) {
		Some(cached) => cached,
		None => downloaded_or_built(name)?,
	};
	Ok(path.to_string_lossy().into_owned())
}

fn downloaded_or_built(name: &str) -> Result<PathBuf, String> {
	if std::env::var_os(BUILD_LOCALLY).is_none() {
		match download(name, &components()) {
			Ok(path) => return Ok(path),
			Err(reason) => eprintln!("use cargo {name}: no prebuilt component ({reason}), so it is built here"),
		}
	}
	build(name).inspect_err(|_| set_aside(name))
}

/// The newest prebuilt component of the crate `name`, checked against its SHA-256, saved into `directory` as
/// `<crate>@<version>.wasm` (`.debug.wasm` for WARP_DEBUG_CRATES)
pub fn download(name: &str, directory: &Path) -> Result<PathBuf, String> {
	let releases = format!("{HOSTED_RELEASES}{GENERATION}");
	let release: Value = serde_json::from_slice(&download_bytes(&releases)?).map_err(|e| format!("{releases}: {e}"))?;
	let assets = release["assets"].as_array().cloned().unwrap_or_default();
	let (version, asset) = assets
		.iter()
		.filter_map(|asset| {
			let version = version_in(asset["name"].as_str()?, name, '.')?;
			Some((Version::parse(version)?, version.to_string(), asset))
		})
		.max_by(|a, b| a.0.cmp(&b.0))
		.map(|(_, version, asset)| (version, asset))
		.ok_or_else(|| format!("{name} is not hosted"))?;
	let url = asset["browser_download_url"].as_str().unwrap_or_default();
	let expected = asset["digest"].as_str().and_then(|digest| digest.strip_prefix(SHA256_PREFIX));
	let bytes = download_bytes(url)?;
	let actual = hex_digest(&bytes);
	if expected != Some(actual.as_str()) {
		return Err(format!("{url}: SHA-256 {actual} is not the published {expected:?}"));
	}
	let path = directory.join(format!("{name}@{version}{}", variant().ending));
	write_atomically(&path, &bytes)?;
	eprintln!("use cargo {name}: downloaded the prebuilt component of {name} {version}");
	Ok(path)
}

fn hex_digest(bytes: &[u8]) -> String {
	use sha2::Digest;
	sha2::Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A wrapper that failed leaves the workspace (an unresolvable dependency would break every other wrapper's build),
/// kept in failed/ to look at
fn set_aside(name: &str) {
	let failed = cache().join(FAILED);
	let _ = std::fs::create_dir_all(&failed);
	let _ = std::fs::remove_dir_all(failed.join(name));
	let _ = std::fs::rename(cache().join(WRAPPERS).join(name), failed.join(name));
}

/// The built components; a new generation of the wrapper generator builds every crate again
fn components() -> PathBuf {
	cache().join(format!("components-{GENERATION}"))
}

fn cache() -> PathBuf {
	PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(CACHE)
}

fn newest_cached(name: &str) -> Option<PathBuf> {
	std::fs::read_dir(components())
		.ok()?
		.flatten()
		.filter_map(|entry| {
			let file = entry.file_name().to_string_lossy().into_owned();
			Some((Version::parse(version_in(&file, name, '@')?)?, entry.path()))
		})
		.max_by(|a, b| a.0.cmp(&b.0))
		.map(|(_, path)| path)
}

fn rust_name(name: &str) -> String {
	name.replace('-', "_")
}
