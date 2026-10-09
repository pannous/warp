//! `warp deploy app.warp` (card cloud-deploy): the program as a Cloudflare Worker. Workers run V8, which runs WASM GC,
//! so the Worker is the playground's host scripts the program needs (reader.js, host.js, its parts, as a built site
//! ships them: src/site.rs) with web/playground/cloud-worker.js, which answers each request by the program's routes
//! (`get "/" {…}`, lowering/serve.rs page·submitted), and the program's module. A Worker compiles no module from bytes,
//! so app.wasm comes in compiled and app.bin as bytes for the host parts that read its sections.
//! `warp deploy app.warp` writes app-worker/ next to the program and runs `wrangler deploy` there (its browser login
//! the first time); `warp deploy --dev app.warp` runs `wrangler dev`, the Worker on this machine; `--dry-run` only
//! writes the directory. A host uploading the Worker itself (Cloudflare's API) sends worker.js as the main module,
//! app.wasm as a compiled module and app.bin as data, the parts wrangler.toml names.

use crate::site::{host_scripts_of, write_files, BuiltSite, SiteFile};
use std::path::{Path, PathBuf};

const WORKER_SCRIPT: &str = include_str!("../web/playground/cloud-worker.js");
const WORKER_FILE: &str = "worker.js";
const MODULE_FILE: &str = "app.wasm";
const MODULE_BYTES_FILE: &str = "app.bin";
const CONFIG_FILE: &str = "wrangler.toml";
/// the Workers runtime version the Worker is written for (wrangler's compatibility_date)
const COMPATIBILITY_DATE: &str = "2026-09-01";
pub const WORKER_SUFFIX: &str = "-worker";
/// the wrangler binary (`npx wrangler` works too, as WARP_WRANGLER="npx wrangler")
const WRANGLER_VARIABLE: &str = "WARP_WRANGLER";
const WRANGLER: &str = "wrangler";
/// What `warp deploy` does after building the Worker
#[derive(Clone, Copy, PartialEq)]
pub enum Deployment {
	/// `wrangler deploy`: to Cloudflare
	Cloud,
	/// `--dev`: `wrangler dev`, on this machine
	Local,
	/// `--dry-run`: only the directory
	DryRun,
}
pub const DEV_FLAG: &str = "--dev";
pub const DRY_RUN_FLAG: &str = "--dry-run";
/// a Worker's name: lower-case letters, digits and dashes
const NAME_LENGTH: usize = 63;

/// The files of the Worker of `code`, named `name`
pub fn worker_files(code: &str, name: &str) -> Result<Vec<SiteFile>, String> {
	// the reflection getters the JavaScript host reads values with
	let module = crate::pipeline::for_any_host(|| crate::pipeline::compile(code)).map_err(|failure| format!("nothing to compile: {}", crate::site::with_excerpt(code, crate::site::message_of(&failure))))?;
	if !crate::site::exports(&module.bytes, crate::serve::PAGE_SUBMITTED) {
		return Err("the program answers no request: give it a route, `get \"/\" { \"hello\" }`".into());
	}
	let mut worker: String = host_scripts_of(&module.bytes)?.iter().map(|(_, script)| *script).collect();
	worker.push_str(WORKER_SCRIPT);
	let config = format!("name = \"{}\"\nmain = \"{WORKER_FILE}\"\ncompatibility_date = \"{COMPATIBILITY_DATE}\"\n\n[[rules]]\ntype = \"Data\"\nglobs = [\"**/*.bin\"]\n", worker_name(name));
	Ok(vec![
		(WORKER_FILE.to_string(), worker.into_bytes()),
		(MODULE_FILE.to_string(), module.bytes.clone()),
		(MODULE_BYTES_FILE.to_string(), module.bytes),
		(CONFIG_FILE.to_string(), config.into_bytes()),
	])
}

/// The Worker of the program file written next to it: app.warp → app-worker/
pub fn build(program: &Path) -> Result<BuiltSite, String> {
	let code = std::fs::read_to_string(program).map_err(|failure| format!("cannot read {}: {failure}", program.display()))?;
	let name = program.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	write_files(&worker_files(&code, &name)?, &worker_directory(program))
}

pub fn worker_directory(program: &Path) -> PathBuf {
	let stem = program.file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
	program.with_file_name(format!("{stem}{WORKER_SUFFIX}"))
}

/// `warp deploy [--dev|--dry-run] app.warp`: the Worker built, then deployed by wrangler (or run on this machine)
pub fn deploy(program: &Path, deployment: Deployment) -> Result<(), String> {
	let built = build(program)?;
	println!("built the Worker {} ({})", built.directory.display(), built.files.join(", "));
	let command = match deployment {
		Deployment::Cloud => "deploy",
		Deployment::Local => "dev",
		Deployment::DryRun => return Ok(()),
	};
	let wrangler = std::env::var(WRANGLER_VARIABLE).unwrap_or_else(|_| WRANGLER.to_string());
	let mut words = wrangler.split_whitespace();
	let binary = words.next().unwrap_or(WRANGLER);
	let status = std::process::Command::new(binary).args(words).arg(command).current_dir(&built.directory).status()
		.map_err(|failure| format!("{binary} does not run ({failure}): install it with `npm install -g wrangler`, or name it in {WRANGLER_VARIABLE}"))?;
	status.success().then_some(()).ok_or_else(|| format!("{wrangler} {command} failed ({status})"))
}

/// `My App.warp` → my-app
fn worker_name(name: &str) -> String {
	let dashed: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
	let trimmed = dashed.trim_matches('-');
	let name = if trimmed.is_empty() { "warp-app" } else { trimmed };
	name.chars().take(NAME_LENGTH).collect()
}
