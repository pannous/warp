//! `use cargo regex` / `use crate "regex"`: a crate from crates.io as a WebAssembly component (notes/cargo_crates.md).
//! A crate has no stable ABI and no WIT of its own, so warp generates a wrapper crate: rustdoc's JSON (RUSTC_BOOTSTRAP
//! on any toolchain) lists the crate's public functions and the structs it constructs; every one whose types map to
//! WIT is exported (a struct as a resource), built for wasm32-wasip2 with wit-bindgen, and what the compiler refuses
//! is dropped by the lines its errors name. The component lands in ~/.cache/warp/crates/<crate>@<version>.wasm (with
//! its .wit), once per machine; `use wasm` loads it from there (src/components.rs). Common crates are prebuilt by CI
//! and downloaded instead (release crates-<generation>), so most users need no cargo.
//! The wrappers are members of one cargo workspace (~/.cache/warp/crates/Cargo.toml), so a configured target-dir is
//! honoured and a user without one gets that workspace's single target/.
use crate::package_tools::{download_bytes, write_atomically};
use crate::versions::Version;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

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
const MISSING_TARGET_SIGN: &str = "target may not be installed";
/// one wrapper build at a time within a process; cargo locks the target directory across processes
static BUILDING: Mutex<()> = Mutex::new(());

/// A cargo profile of the wrappers and the file name ending of its components
struct Variant {
	profile: &'static str,
	ending: &'static str,
}
const RELEASE: Variant = Variant {
	profile: "release",
	ending: ".wasm",
};
const DEBUG: Variant = Variant {
	profile: "debugging",
	ending: ".debug.wasm",
};

fn variant() -> &'static Variant {
	match std::env::var_os(DEBUG_BUILDS) {
		Some(_) => &DEBUG,
		None => &RELEASE,
	}
}

/// The version in a component file name `<crate><separator><version><ending>` of the variant
fn version_in<'a>(file: &'a str, name: &str, separator: char) -> Option<&'a str> {
	let version = file
		.strip_prefix(name)?
		.strip_prefix(separator)?
		.strip_suffix(variant().ending)?;
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
	let expected = asset["digest"]
		.as_str()
		.and_then(|digest| digest.strip_prefix(SHA256_PREFIX));
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
	sha2::Sha256::digest(bytes)
		.iter()
		.map(|byte| format!("{byte:02x}"))
		.collect()
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

fn build(name: &str) -> Result<PathBuf, String> {
	let cache = cache();
	let wrapper = cache.join(WRAPPERS).join(name);
	std::fs::create_dir_all(wrapper.join("src")).map_err(|e| format!("{}: {e}", wrapper.display()))?;
	write(&cache.join("Cargo.toml"), WORKSPACE_MANIFEST)?;
	write_manifest(&wrapper, name, "*")?;
	write(&wrapper.join("src/lib.rs"), "")?;
	let metadata = cargo_json(&wrapper, &["metadata", "--format-version", "1"])?;
	let version = resolved_version(&metadata, name)?;
	let target = PathBuf::from(metadata["target_directory"].as_str().unwrap_or("target"));
	write_manifest(&wrapper, name, &format!("={version}"))?;
	eprintln!(
		"use cargo {name}: building crate {name} {version} as a component, once (~/{CACHE}/components-{GENERATION})"
	);
	let package = format!("{name}@{version}");
	cargo(
		&wrapper,
		&[
			"rustdoc",
			"-p",
			&package,
			"--lib",
			"--",
			"-Z",
			"unstable-options",
			"--output-format",
			"json",
		],
		true,
	)?;
	let doc = target.join("doc").join(format!("{}.json", rust_name(name)));
	let doc = std::fs::read_to_string(&doc).map_err(|e| format!("{}: {e}", doc.display()))?;
	let doc: Value = serde_json::from_str(&doc).map_err(|e| format!("rustdoc JSON of {name}: {e}"))?;
	let mut api = Api::of(&doc, &rust_name(name));
	let wasm = compile_dropping_refused(&wrapper, name, &mut api, &target)?;
	let built = components().join(format!("{name}@{version}{}", variant().ending));
	write_atomically(
		&built,
		&std::fs::read(&wasm).map_err(|e| format!("{}: {e}", wasm.display()))?,
	)?;
	write(&built.with_extension("wit"), &api.wit(name))?;
	Ok(built)
}

fn write(path: &Path, text: &str) -> Result<(), String> {
	if std::fs::read_to_string(path).is_ok_and(|old| old == text) {
		return Ok(());
	}
	std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_manifest(wrapper: &Path, name: &str, requirement: &str) -> Result<(), String> {
	write(
		&wrapper.join("Cargo.toml"),
		&format!(
			"[package]\nname = \"{WRAPPER_PREFIX}{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\n{name} = \"{requirement}\"\nwit-bindgen = \"{WIT_BINDGEN}\"\n"
		),
	)
}

/// `cargo <arguments>` in the wrapper, offline first (the local registry), then with the network
fn cargo_output(wrapper: &Path, arguments: &[&str], bootstrap: bool) -> Result<std::process::Output, String> {
	let run = |offline: bool| {
		let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
		command.args(arguments).current_dir(wrapper);
		if offline {
			command.arg("--offline");
		}
		if bootstrap {
			command.env("RUSTC_BOOTSTRAP", "1");
		}
		command
			.output()
			.map_err(|e| format!("use cargo needs cargo (https://rustup.rs): {e}"))
	};
	let offline = run(true)?;
	match offline.status.success() {
		true => Ok(offline),
		false => run(false),
	}
}

fn cargo(wrapper: &Path, arguments: &[&str], bootstrap: bool) -> Result<String, String> {
	let output = cargo_output(wrapper, arguments, bootstrap)?;
	let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
	match output.status.success() {
		true => Ok(stdout),
		false => Err(cargo_failure(arguments, &String::from_utf8_lossy(&output.stderr))),
	}
}

fn cargo_failure(arguments: &[&str], stderr: &str) -> String {
	let advice = match stderr.contains(MISSING_TARGET_SIGN) {
		true => format!("; install the target with: rustup target add {TARGET}"),
		false => String::new(),
	};
	let problems: Vec<&str> = stderr.lines().filter(|line| line.starts_with("error")).collect();
	let problems = match problems.is_empty() {
		true => stderr.trim().to_string(),
		false => problems.join("\n"),
	};
	format!("cargo {} failed: {problems}{advice}", arguments.join(" "))
}

fn cargo_json(wrapper: &Path, arguments: &[&str]) -> Result<Value, String> {
	serde_json::from_str(&cargo(wrapper, arguments, false)?).map_err(|e| format!("cargo {}: {e}", arguments[0]))
}

/// The version of the crate `name` cargo resolved for the wrapper
fn resolved_version(metadata: &Value, name: &str) -> Result<String, String> {
	let wrapper_name = format!("{WRAPPER_PREFIX}{name}");
	let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
	let wrapper_id = packages
		.iter()
		.find(|package| package["name"] == wrapper_name.as_str())
		.and_then(|package| package["id"].as_str())
		.ok_or_else(|| format!("cargo metadata lists no {wrapper_name}"))?;
	let nodes = metadata["resolve"]["nodes"].as_array().cloned().unwrap_or_default();
	let dependencies = nodes
		.iter()
		.find(|node| node["id"] == wrapper_id)
		.and_then(|node| node["dependencies"].as_array().cloned())
		.unwrap_or_default();
	packages
		.iter()
		.filter(|package| package["name"] == name && dependencies.iter().any(|id| *id == package["id"]))
		.find_map(|package| package["version"].as_str().map(String::from))
		.ok_or_else(|| format!("cargo resolved no crate {name}"))
}

/// Build the wrapper; a function the compiler refuses is dropped and the wrapper built again
fn compile_dropping_refused(wrapper: &Path, name: &str, api: &mut Api, target: &Path) -> Result<PathBuf, String> {
	let package = format!("{WRAPPER_PREFIX}{name}");
	let arguments = [
		"build",
		"--profile",
		variant().profile,
		"--target",
		TARGET,
		"-p",
		&package,
		"--message-format",
		"json",
	];
	for _ in 0..BUILD_ROUNDS {
		if api.functions.is_empty() && api.resources.is_empty() {
			return Err(format!(
				"crate {name} has no public function warp can call: none takes and gives only texts, numbers, bools, bytes, lists, options, results or structs the crate constructs (generic and trait-based APIs need a wrapper written by hand)"
			));
		}
		std::fs::create_dir_all(wrapper.join("wit")).map_err(|e| e.to_string())?;
		write(&wrapper.join("wit/wrapper.wit"), &api.wit(name))?;
		let (code, lines) = api.rust(name);
		write(&wrapper.join("src/lib.rs"), &code)?;
		let output = cargo_output(wrapper, &arguments, false)?;
		if output.status.success() {
			return Ok(target
				.join(TARGET)
				.join(variant().profile)
				.join(format!("{}.wasm", rust_name(&package))));
		}
		let messages = String::from_utf8_lossy(&output.stdout).into_owned();
		let (refused_lines, rendered) = refused(&messages);
		let refused_items: HashSet<String> = refused_lines
			.iter()
			.filter_map(|line| {
				lines
					.iter()
					.find(|(range, _)| range.contains(line))
					.map(|(_, item)| item.clone())
			})
			.collect();
		if refused_items.is_empty() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(match rendered.is_empty() {
				true => cargo_failure(&arguments, &stderr),
				false => format!("the wrapper of crate {name} does not build:\n{}", rendered.join("\n")),
			});
		}
		api.drop_items(&refused_items);
	}
	Err(format!(
		"the wrapper of crate {name} still does not build after {BUILD_ROUNDS} rounds"
	))
}

/// The lines of src/lib.rs cargo's JSON errors name, and the errors as rustc printed them
fn refused(messages: &str) -> (Vec<usize>, Vec<String>) {
	let mut lines = vec![];
	let mut rendered = vec![];
	for message in messages
		.lines()
		.filter_map(|line| serde_json::from_str::<Value>(line).ok())
	{
		let message = &message["message"];
		if message["level"] != "error" {
			continue;
		}
		rendered.extend(message["rendered"].as_str().map(String::from));
		for span in message["spans"].as_array().into_iter().flatten() {
			if span["file_name"]
				.as_str()
				.is_some_and(|file| file.ends_with("src/lib.rs"))
			{
				lines.extend(span["line_start"].as_u64().map(|line| line as usize));
			}
		}
	}
	(lines, rendered)
}

/// A crate's value as WIT and the Rust converting it: `expression` has `{}` where the value goes
#[derive(Clone)]
struct Conversion {
	wit: String,
	rust: String,
	expression: String,
}

impl Conversion {
	fn new(wit: &str, rust: &str, expression: &str) -> Conversion {
		Conversion {
			wit: wit.into(),
			rust: rust.into(),
			expression: expression.into(),
		}
	}
	fn same(wit: &str, rust: &str) -> Conversion {
		Conversion::new(wit, rust, "{}")
	}
	fn of(&self, value: &str) -> String {
		self.expression.replace("{}", value)
	}
	fn is_identity(&self) -> bool {
		self.expression == "{}"
	}
}

/// The receiver of a method: `&self`, `&mut self`, `self`
#[derive(Clone, Copy, PartialEq)]
enum Receiver {
	Shared,
	Mutable,
	Owned,
	/// a public field read: `self.major`
	Field,
}

#[derive(Clone)]
struct Function {
	/// rust name as written in the crate
	name: String,
	parameters: Vec<(String, Conversion)>,
	result: Option<Conversion>,
	receiver: Option<Receiver>,
}

struct Resource {
	/// the name at the crate's root: `Regex`
	name: String,
	functions: Vec<Function>,
}

/// What the wrapper exports: the crate's free functions and its structs as resources
struct Api {
	crate_name: String,
	functions: Vec<Function>,
	resources: Vec<Resource>,
}

/// Rust's primitive types and their WIT names
const PRIMITIVES: [(&str, &str); 12] = [
	("bool", "bool"),
	("char", "char"),
	("i8", "s8"),
	("i16", "s16"),
	("i32", "s32"),
	("i64", "s64"),
	("u8", "u8"),
	("u16", "u16"),
	("u32", "u32"),
	("u64", "u64"),
	("f32", "f32"),
	("f64", "f64"),
];
/// a struct implementing Display gets the method to_string
const DISPLAY_TRAIT: &str = "Display";
const TO_STRING: &str = "to_string";
const STRING_TRAITS: [&str; 5] = ["AsRef", "Borrow", "Into", "ToString", "Display"];

fn last_segment(path: &str) -> &str {
	path.rsplit("::").next().unwrap_or(path)
}

fn type_arguments(path: &Value) -> Vec<&Value> {
	path["args"]["angle_bracketed"]["args"]
		.as_array()
		.into_iter()
		.flatten()
		.filter_map(|arg| arg.get("type"))
		.collect()
}

fn kebab(rust: &str) -> Option<String> {
	let mut kebab = String::new();
	let mut previous_lower = false;
	for c in rust.chars() {
		match c {
			'_' => {
				kebab.push('-');
				previous_lower = false;
			}
			c if c.is_ascii_uppercase() => {
				if previous_lower {
					kebab.push('-');
				}
				kebab.push(c.to_ascii_lowercase());
				previous_lower = false;
			}
			c => {
				kebab.push(c);
				previous_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
			}
		}
	}
	let valid = kebab.split('-').all(|segment| {
		segment.starts_with(|c: char| c.is_ascii_lowercase()) && segment.chars().all(|c| c.is_ascii_alphanumeric())
	});
	valid.then_some(kebab)
}

/// `regex-set` → `RegexSet`, the name wit-bindgen gives a resource
fn camel(kebab: &str) -> String {
	kebab
		.split('-')
		.map(|segment| {
			let mut chars = segment.chars();
			chars
				.next()
				.map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
				.unwrap_or_default()
		})
		.collect()
}

/// Reads rustdoc JSON: the items at the crate's root (re-exports followed) and how their types convert
struct Reader<'a> {
	index: &'a Value,
	crate_name: String,
	/// the structs a function constructs, by rustdoc id: their name at the root
	resources: HashMap<String, String>,
}

impl<'a> Reader<'a> {
	fn item(&self, id: &Value) -> Option<&'a Value> {
		self.index.get(id.to_string().trim_matches('"'))
	}

	/// The root's public items by the name they have there: `use` re-exports followed, glob re-exports expanded
	fn root_items(&self, module: &'a Value, items: &mut Vec<(String, &'a Value)>, seen: &mut HashSet<String>) {
		for id in module["inner"]["module"]["items"].as_array().into_iter().flatten() {
			let Some(item) = self.item(id) else { continue };
			if let Some(reexport) = item["inner"].get("use") {
				let Some(target) = reexport.get("id").and_then(|id| self.item(id)) else {
					continue;
				};
				let key = reexport["id"].to_string();
				if reexport["is_glob"] == true {
					if seen.insert(key) && target["inner"].get("module").is_some() {
						self.root_items(target, items, seen);
					}
				} else if let Some(name) = reexport["name"].as_str() {
					items.push((name.to_string(), target));
				}
			} else if item["visibility"] == "public" {
				if let Some(name) = item["name"].as_str() {
					items.push((name.to_string(), item));
				}
			}
		}
	}

	/// The trait bounds of a generic parameter, from the function's generics
	fn bounds(generics: &'a Value, name: &str) -> Vec<&'a Value> {
		let declared = generics["params"]
			.as_array()
			.into_iter()
			.flatten()
			.filter(|param| param["name"] == name)
			.flat_map(|param| param["kind"]["type"]["bounds"].as_array().into_iter().flatten());
		let predicates = generics["where_predicates"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|predicate| predicate.get("bound_predicate"))
			.filter(|predicate| predicate["type"]["generic"] == name)
			.flat_map(|predicate| predicate["bounds"].as_array().into_iter().flatten());
		declared.chain(predicates).collect()
	}

	/// A parameter given as a generic with these bounds: a text for `AsRef<str>`, `Into<String>`…, bytes for
	/// `AsRef<[u8]>`; any other trait is tried as a text too and dropped if the compiler refuses
	fn bounded(&self, bounds: &[&Value]) -> Option<Conversion> {
		let traits: Vec<&Value> = bounds
			.iter()
			.filter_map(|bound| bound.get("trait_bound"))
			.map(|bound| &bound["trait"])
			.collect();
		let bytes = traits.iter().any(|t| {
			type_arguments(t)
				.first()
				.is_some_and(|argument| argument["slice"]["primitive"] == "u8")
		});
		match traits.as_slice() {
			[] => None,
			_ if bytes => Some(Conversion::same("list<u8>", "Vec<u8>")),
			_ if traits
				.iter()
				.all(|t| STRING_TRAITS.contains(&last_segment(t["path"].as_str().unwrap_or("")))) =>
			{
				Some(Conversion::same("string", "String"))
			}
			_ => Some(Conversion::new("string", "String", "{}.as_str()")),
		}
	}

	/// The type a plain type alias names: `HammingResult` is `Result<usize, StrSimError>`
	fn aliased(&self, ty: &Value) -> Option<&'a Value> {
		let alias = self.item(&ty["resolved_path"]["id"])?["inner"].get("type_alias")?;
		alias["generics"]["params"]
			.as_array()
			.is_some_and(|params| params.is_empty())
			.then(|| &alias["type"])
	}

	fn resource_of(&self, ty: &Value, self_type: Option<&str>) -> Option<String> {
		if ty["generic"] == "Self" {
			return self_type.map(String::from);
		}
		let path = ty.get("resolved_path")?;
		if !type_arguments(path).is_empty() {
			return None;
		}
		self.resources.get(&path["id"].to_string()).cloned()
	}

	fn parameter(&self, ty: &Value, generics: &Value, self_type: Option<&str>) -> Option<Conversion> {
		if let Some(aliased) = self.aliased(ty) {
			return self.parameter(aliased, generics, self_type);
		}
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, true);
		}
		if let Some(name) = ty["generic"].as_str().filter(|name| *name != "Self") {
			return self.bounded(&Reader::bounds(generics, name));
		}
		if let Some(bounds) = ty["impl_trait"].as_array() {
			return self.bounded(&bounds.iter().collect::<Vec<_>>());
		}
		if let Some(reference) = ty.get("borrowed_ref") {
			let inner = &reference["type"];
			let mutable = reference["is_mutable"] == true;
			if let Some(resource) = self.resource_of(inner, self_type) {
				let kebab = kebab(&resource)?;
				let borrow = if mutable { "borrow_mut" } else { "borrow" };
				return Some(Conversion::new(
					&format!("borrow<{kebab}>"),
					&format!("{}Borrow<'_>", camel(&kebab)),
					&format!(
						"&{}*{{}}.get::<{resource}Resource>().0.{borrow}()",
						if mutable { "mut " } else { "" }
					),
				));
			}
			if mutable {
				return None;
			}
			if inner["primitive"] == "str" {
				return Some(Conversion::new("string", "String", "&{}"));
			}
			if let Some(element) = inner.get("slice") {
				let element = self.element(element)?;
				return Some(match element.is_identity() {
					true => Conversion::new(
						&format!("list<{}>", element.wit),
						&format!("Vec<{}>", element.rust),
						"&{}",
					),
					false => Conversion::new(
						&format!("list<{}>", element.wit),
						&format!("Vec<{}>", element.rust),
						&format!("&{{}}.iter().map(|x| {}).collect::<Vec<_>>()", element.of("x")),
					),
				});
			}
			if let Some(name) = inner["generic"].as_str().filter(|name| *name != "Self") {
				return self.bounded(&Reader::bounds(generics, name)).map(|text| Conversion {
					expression: "&{}".into(),
					..text
				});
			}
			return None;
		}
		if let Some(resource) = self.resource_of(ty, self_type) {
			let kebab = kebab(&resource)?;
			return Some(Conversion::new(
				&format!("borrow<{kebab}>"),
				&format!("{}Borrow<'_>", camel(&kebab)),
				&format!("{{}}.get::<{resource}Resource>().0.borrow().clone()"),
			));
		}
		let path = ty.get("resolved_path")?;
		let arguments = type_arguments(path);
		match (last_segment(path["path"].as_str()?), arguments.as_slice()) {
			("String", []) => Some(Conversion::same("string", "String")),
			("PathBuf", []) => Some(Conversion::new("string", "String", "{}.into()")),
			("Vec", [element]) => {
				let element = self
					.parameter(element, generics, self_type)
					.filter(Conversion::is_identity)?;
				Some(Conversion::same(
					&format!("list<{}>", element.wit),
					&format!("Vec<{}>", element.rust),
				))
			}
			("Option", [inner]) => {
				let inner = self.parameter(inner, generics, self_type)?;
				let expression = match inner.expression.as_str() {
					"{}" => "{}",
					"&{}" => "{}.as_deref()",
					_ => return None,
				};
				Some(Conversion::new(
					&format!("option<{}>", inner.wit),
					&format!("Option<{}>", inner.rust),
					expression,
				))
			}
			_ => None,
		}
	}

	/// An element of a slice parameter, borrowed from the wrapper's Vec: `&[&str]` from `Vec<String>`
	fn element(&self, ty: &Value) -> Option<Conversion> {
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, true).filter(Conversion::is_identity);
		}
		if ty["borrowed_ref"]["type"]["primitive"] == "str" {
			return Some(Conversion::new("string", "String", "{}.as_str()"));
		}
		let path = ty.get("resolved_path")?;
		(last_segment(path["path"].as_str()?) == "String").then(|| Conversion::new("string", "String", "{}.clone()"))
	}

	fn result(&self, ty: &Value, self_type: Option<&str>) -> Option<Conversion> {
		if let Some(aliased) = self.aliased(ty) {
			return self.result(aliased, self_type);
		}
		if let Some(primitive) = ty["primitive"].as_str() {
			return primitive_conversion(primitive, false);
		}
		if let Some(resource) = self.resource_of(ty, self_type) {
			let kebab = kebab(&resource)?;
			let camel = camel(&kebab);
			return Some(Conversion::new(
				&kebab,
				&camel,
				&format!("{camel}::new({resource}Resource(std::cell::RefCell::new({{}})))"),
			));
		}
		if let Some(reference) = ty.get("borrowed_ref") {
			let inner = &reference["type"];
			if inner["primitive"] == "str" {
				return Some(Conversion::new("string", "String", "{}.to_string()"));
			}
			let element = self
				.result(inner.get("slice")?, self_type)
				.filter(Conversion::is_identity)?;
			return Some(Conversion::new(
				&format!("list<{}>", element.wit),
				&format!("Vec<{}>", element.rust),
				"{}.to_vec()",
			));
		}
		if let Some(items) = ty["tuple"].as_array() {
			let items: Vec<Conversion> = items
				.iter()
				.map(|item| self.result(item, self_type).filter(Conversion::is_identity))
				.collect::<Option<_>>()?;
			let wit: Vec<&str> = items.iter().map(|item| item.wit.as_str()).collect();
			let rust: Vec<&str> = items.iter().map(|item| item.rust.as_str()).collect();
			return Some(Conversion::same(
				&format!("tuple<{}>", wit.join(", ")),
				&format!("({})", rust.join(", ")),
			));
		}
		let path = ty.get("resolved_path")?;
		let arguments = type_arguments(path);
		match (last_segment(path["path"].as_str()?), arguments.as_slice()) {
			("String", []) => Some(Conversion::same("string", "String")),
			("PathBuf", []) => Some(Conversion::new("string", "String", "{}.to_string_lossy().into_owned()")),
			("Cow", [inner]) if inner["primitive"] == "str" => {
				Some(Conversion::new("string", "String", "{}.into_owned()"))
			}
			("Cow", [inner]) => {
				let element = self
					.result(inner.get("slice")?, self_type)
					.filter(Conversion::is_identity)?;
				Some(Conversion::new(
					&format!("list<{}>", element.wit),
					&format!("Vec<{}>", element.rust),
					"{}.into_owned()",
				))
			}
			("Vec", [element]) => {
				let element = self.result(element, self_type)?;
				let list = |expression: &str| {
					Conversion::new(
						&format!("list<{}>", element.wit),
						&format!("Vec<{}>", element.rust),
						expression,
					)
				};
				Some(match element.is_identity() {
					true => list("{}"),
					false => list(&format!("{{}}.into_iter().map(|x| {}).collect()", element.of("x"))),
				})
			}
			("Option", [inner]) => {
				let inner = self.result(inner, self_type)?;
				let option = |expression: &str| {
					Conversion::new(
						&format!("option<{}>", inner.wit),
						&format!("Option<{}>", inner.rust),
						expression,
					)
				};
				Some(match inner.is_identity() {
					true => option("{}"),
					false => option(&format!("{{}}.map(|x| {})", inner.of("x"))),
				})
			}
			("Result", [ok, ..]) => {
				let unit = ok["tuple"].as_array().is_some_and(|items| items.is_empty());
				let (wit, rust, mapped) = match unit {
					true => ("_".to_string(), "()".to_string(), "{}".to_string()),
					false => {
						let ok = self.result(ok, self_type)?;
						let mapped = match ok.is_identity() {
							true => "{}".to_string(),
							false => format!("{{}}.map(|x| {})", ok.of("x")),
						};
						(ok.wit, ok.rust, mapped)
					}
				};
				Some(Conversion::new(
					&format!("result<{wit}, string>"),
					&format!("Result<{rust}, String>"),
					&format!("{mapped}.map_err(|e| e.to_string())"),
				))
			}
			_ => None,
		}
	}

	fn function(&self, name: &str, item: &Value, self_type: Option<&str>) -> Option<Function> {
		let function = item["inner"].get("function")?;
		let header = &function["header"];
		if header["is_unsafe"] == true || header["is_async"] == true {
			return None;
		}
		kebab(name)?;
		let generics = &function["generics"];
		let inputs = function["sig"]["inputs"].as_array()?;
		let mut receiver = None;
		let mut parameters = vec![];
		for (position, input) in inputs.iter().enumerate() {
			let (parameter, ty) = (input[0].as_str().unwrap_or(""), &input[1]);
			if position == 0 && parameter == "self" {
				receiver = Some(match ty.get("borrowed_ref") {
					Some(reference) if reference["is_mutable"] == true => Receiver::Mutable,
					Some(_) => Receiver::Shared,
					None => Receiver::Owned,
				});
				continue;
			}
			let parameter = kebab(parameter).unwrap_or_else(|| format!("arg{position}"));
			parameters.push((parameter, self.parameter(ty, generics, self_type)?));
		}
		let output = &function["sig"]["output"];
		let result = match output.is_null() {
			true => None,
			false => Some(self.result(output, self_type)?),
		};
		Some(Function {
			name: name.to_string(),
			parameters,
			result,
			receiver,
		})
	}

	/// The public functions of a struct's inherent impls
	fn methods(&self, name: &str, item: &Value) -> Vec<Function> {
		let impls = item["inner"]["struct"]["impls"].as_array().into_iter().flatten();
		let mut methods: Vec<Function> = impls
			.filter_map(|id| self.item(id))
			.filter(|implementation| implementation["inner"]["impl"]["trait"].is_null())
			.flat_map(|implementation| {
				implementation["inner"]["impl"]["items"]
					.as_array()
					.into_iter()
					.flatten()
			})
			.filter_map(|id| self.item(id))
			.filter(|method| method["visibility"] == "public")
			.filter_map(|method| self.function(method["name"].as_str()?, method, Some(name)))
			.collect();
		for (field, result) in self.fields(item, name) {
			if !methods.iter().any(|method| method.name == field) {
				methods.push(Function {
					name: field,
					parameters: vec![],
					result: Some(result),
					receiver: Some(Receiver::Field),
				});
			}
		}
		if self.implements(item, DISPLAY_TRAIT) && !methods.iter().any(|method| method.name == TO_STRING) {
			methods.push(Function {
				name: TO_STRING.into(),
				parameters: vec![],
				result: Some(Conversion::same("string", "String")),
				receiver: Some(Receiver::Shared),
			});
		}
		methods
	}

	/// The public fields of a struct whose values cross: each read by a method of its name (`v.major()`)
	fn fields(&self, item: &Value, name: &str) -> Vec<(String, Conversion)> {
		item["inner"]["struct"]["kind"]["plain"]["fields"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|id| self.item(id))
			.filter(|field| field["visibility"] == "public")
			.filter_map(|field| {
				let result = self.result(field["inner"].get("struct_field")?, Some(name))?;
				Some((
					kebab(field["name"].as_str()?)
						.map(|_| field["name"].as_str())??
						.to_string(),
					result,
				))
			})
			.collect()
	}

	fn implements(&self, item: &Value, trait_name: &str) -> bool {
		item["inner"]["struct"]["impls"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|id| self.item(id))
			.any(|implementation| {
				implementation["inner"]["impl"]["trait"]["path"]
					.as_str()
					.is_some_and(|path| last_segment(path) == trait_name)
			})
	}

	/// Does a function of the struct make one (`new` returning `Self`, `Result<Self, E>`, `Option<Self>`)?
	fn constructs(&self, item: &Value) -> bool {
		let impls = item["inner"]["struct"]["impls"].as_array().into_iter().flatten();
		impls
			.filter_map(|id| self.item(id))
			.filter(|implementation| implementation["inner"]["impl"]["trait"].is_null())
			.flat_map(|implementation| {
				implementation["inner"]["impl"]["items"]
					.as_array()
					.into_iter()
					.flatten()
			})
			.filter_map(|id| self.item(id))
			.filter_map(|method| method["inner"].get("function"))
			.any(|function| {
				let output = &function["sig"]["output"];
				let is_self = |ty: &Value| ty["generic"] == "Self" || ty["resolved_path"]["id"] == item["id"];
				is_self(output)
					|| type_arguments(&output["resolved_path"])
						.first()
						.is_some_and(|ty| is_self(ty))
			})
	}
}

/// A primitive as a parameter (`usize` from a u64) or a result (`usize` as a u64)
fn primitive_conversion(primitive: &str, parameter: bool) -> Option<Conversion> {
	let sized = |wit: &str, rust: &str, native: &str| match parameter {
		true => Conversion::new(wit, rust, &format!("{{}} as {native}")),
		false => Conversion::new(wit, rust, &format!("{{}} as {rust}")),
	};
	match primitive {
		"usize" => Some(sized("u64", "u64", "usize")),
		"isize" => Some(sized("s64", "i64", "isize")),
		_ => PRIMITIVES
			.iter()
			.find(|(rust, _)| *rust == primitive)
			.map(|(rust, wit)| Conversion::same(wit, rust)),
	}
}

impl Api {
	fn of(doc: &Value, crate_name: &str) -> Api {
		let index = &doc["index"];
		let mut reader = Reader {
			index,
			crate_name: crate_name.to_string(),
			resources: HashMap::new(),
		};
		let root = reader.item(&doc["root"]).cloned().unwrap_or(Value::Null);
		let mut items = vec![];
		reader.root_items(&root, &mut items, &mut HashSet::new());
		let structs: Vec<(String, &Value)> = items
			.iter()
			.filter(|(name, item)| {
				let plain = item["inner"]["struct"]["generics"]["params"]
					.as_array()
					.is_some_and(|params| params.is_empty());
				plain && reader.constructs(item) && kebab(name).is_some()
			})
			.map(|(name, item)| (name.clone(), *item))
			.collect();
		for (name, item) in &structs {
			let id = item["id"].to_string();
			reader.resources.insert(id, name.clone());
		}
		let mut names = HashSet::new();
		let resources: Vec<Resource> = structs
			.iter()
			.filter(|(name, _)| names.insert(kebab(name).unwrap_or_default()))
			.map(|(name, item)| Resource {
				name: name.clone(),
				functions: reader.methods(name, item),
			})
			.collect();
		let functions = items
			.iter()
			.filter_map(|(name, item)| reader.function(name, item, None))
			.filter(|function| names.insert(kebab(&function.name).unwrap_or_default()))
			.collect();
		Api {
			crate_name: reader.crate_name,
			functions,
			resources,
		}
	}

	/// Item keys: `escape`, `Regex`, `Regex.is_match`
	fn drop_items(&mut self, refused: &HashSet<String>) {
		self.functions.retain(|function| !refused.contains(&function.name));
		self.resources.retain(|resource| !refused.contains(&resource.name));
		for resource in &mut self.resources {
			let name = resource.name.clone();
			resource
				.functions
				.retain(|function| !refused.contains(&format!("{name}.{}", function.name)));
		}
	}

	fn wit(&self, crate_name: &str) -> String {
		let mut wit = format!(
			"package warp:{WRAPPER_PREFIX}{};\n\ninterface api {{\n",
			crate_name.replace('_', "-")
		);
		for resource in &self.resources {
			let kebab = kebab(&resource.name).unwrap_or_default();
			wit += &format!("  resource %{kebab} {{\n");
			for function in &resource.functions {
				wit += &format!("    {}\n", function_wit(function, true));
			}
			wit += "  }\n";
		}
		for function in &self.functions {
			wit += &format!("  {}\n", function_wit(function, false));
		}
		wit + "}\n\nworld wrapper {\n  export api;\n}\n"
	}

	/// The wrapper's Rust and which item each range of its lines belongs to
	fn rust(&self, crate_name: &str) -> (String, Vec<(std::ops::Range<usize>, String)>) {
		let module = format!(
			"exports::warp::{}::api",
			rust_name(&format!("{WRAPPER_PREFIX}{crate_name}"))
		);
		let mut code = Code::default();
		code.line("#![allow(warnings, clippy::all)]", None);
		code.line("wit_bindgen::generate!({ world: \"wrapper\", path: \"wit\" });", None);
		code.line(&format!("use {module}::*;"), None);
		code.line("struct Wrapper;", None);
		code.line("export!(Wrapper);", None);
		for resource in &self.resources {
			let name = &resource.name;
			let key = Some(name.as_str());
			code.line(
				&format!(
					"pub struct {name}Resource(std::cell::RefCell<::{}::{name}>);",
					self.crate_name
				),
				key,
			);
			code.line(
				&format!(
					"impl Guest{} for {name}Resource {{",
					camel(&kebab(name).unwrap_or_default())
				),
				key,
			);
			for function in &resource.functions {
				let body = self.call(function, Some(name));
				code.line(&body, Some(&format!("{name}.{}", function.name)));
			}
			code.line("}", key);
		}
		code.line("impl Guest for Wrapper {", None);
		for resource in &self.resources {
			let name = &resource.name;
			let camel = camel(&kebab(name).unwrap_or_default());
			code.line(&format!("type {camel} = {name}Resource;"), Some(name));
		}
		for function in &self.functions {
			code.line(&self.call(function, None), Some(&function.name));
		}
		code.line("}", None);
		(code.text, code.ranges)
	}

	/// One wrapper function: its signature as wit-bindgen's trait has it and the call of the crate's function
	fn call(&self, function: &Function, resource: Option<&str>) -> String {
		let snake = kebab(&function.name).unwrap_or_default().replace('-', "_");
		let mut parameters: Vec<String> = vec![];
		if function.receiver.is_some() {
			parameters.push("&self".into());
		}
		let mut arguments = vec![];
		for (position, (_, conversion)) in function.parameters.iter().enumerate() {
			parameters.push(format!("a{position}: {}", conversion.rust));
			arguments.push(conversion.of(&format!("a{position}")));
		}
		let callee = match (function.receiver, resource) {
			(Some(Receiver::Mutable), _) => format!("self.0.borrow_mut().{}", function.name),
			(Some(Receiver::Shared), _) => format!("self.0.borrow().{}", function.name),
			(Some(Receiver::Owned), _) => format!("self.0.borrow().clone().{}", function.name),
			(Some(Receiver::Field), _) => format!("self.0.borrow().{}.clone()", function.name),
			(None, Some(resource)) => format!("::{}::{resource}::{}", self.crate_name, function.name),
			(None, None) => format!("::{}::{}", self.crate_name, function.name),
		};
		let call = match function.receiver {
			Some(Receiver::Field) => callee,
			_ => format!("{callee}({})", arguments.join(", ")),
		};
		let (returns, body) = match &function.result {
			Some(result) => (format!(" -> {}", result.rust), result.of(&call)),
			None => (String::new(), format!("{call};")),
		};
		format!("fn {snake}({}){returns} {{ {body} }}", parameters.join(", "))
	}
}

fn function_wit(function: &Function, in_resource: bool) -> String {
	let parameters: Vec<String> = function
		.parameters
		.iter()
		.map(|(name, conversion)| format!("%{name}: {}", conversion.wit))
		.collect();
	let returns = match &function.result {
		Some(result) => format!(" -> {}", result.wit),
		None => String::new(),
	};
	let kind = match in_resource && function.receiver.is_none() {
		true => "static func",
		false => "func",
	};
	format!(
		"%{}: {kind}({}){returns};",
		kebab(&function.name).unwrap_or_default(),
		parameters.join(", ")
	)
}

/// Generated source, remembering which item each line belongs to
#[derive(Default)]
struct Code {
	text: String,
	ranges: Vec<(std::ops::Range<usize>, String)>,
	lines: usize,
}

impl Code {
	fn line(&mut self, line: &str, item: Option<&str>) {
		self.lines += 1;
		if let Some(item) = item {
			self.ranges.push((self.lines..self.lines + 1, item.to_string()));
		}
		self.text += line;
		self.text += "\n";
	}
}
