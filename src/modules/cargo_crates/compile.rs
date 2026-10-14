//! Building the wrapper crate with cargo, dropping what the compiler refuses
use super::*;

pub(super) fn build(name: &str) -> Result<PathBuf, String> {
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
		&["rustdoc", "-p", &package, "--lib", "--", "-Z", "unstable-options", "--output-format", "json"],
		true,
	)?;
	let doc = target.join("doc").join(format!("{}.json", rust_name(name)));
	let doc = std::fs::read_to_string(&doc).map_err(|e| format!("{}: {e}", doc.display()))?;
	let doc: Value = serde_json::from_str(&doc).map_err(|e| format!("rustdoc JSON of {name}: {e}"))?;
	let mut api = Api::of(&doc, &rust_name(name));
	let wasm = compile_dropping_refused(&wrapper, name, &mut api, &target)?;
	let built = components().join(format!("{name}@{version}{}", variant().ending));
	write_atomically(&built, &std::fs::read(&wasm).map_err(|e| format!("{}: {e}", wasm.display()))?)?;
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
		command.output().map_err(|e| format!("use cargo needs cargo (https://rustup.rs): {e}"))
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
	let arguments =
		["build", "--profile", variant().profile, "--target", TARGET, "-p", &package, "--message-format", "json"];
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
			return Ok(target.join(TARGET).join(variant().profile).join(format!("{}.wasm", rust_name(&package))));
		}
		let messages = String::from_utf8_lossy(&output.stdout).into_owned();
		let (refused_lines, rendered) = refused(&messages);
		let refused_items: HashSet<String> = refused_lines
			.iter()
			.filter_map(|line| lines.iter().find(|(range, _)| range.contains(line)).map(|(_, item)| item.clone()))
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
	Err(format!("the wrapper of crate {name} still does not build after {BUILD_ROUNDS} rounds"))
}

/// The lines of src/lib.rs cargo's JSON errors name, and the errors as rustc printed them
fn refused(messages: &str) -> (Vec<usize>, Vec<String>) {
	let mut lines = vec![];
	let mut rendered = vec![];
	for message in messages.lines().filter_map(|line| serde_json::from_str::<Value>(line).ok()) {
		let message = &message["message"];
		if message["level"] != "error" {
			continue;
		}
		rendered.extend(message["rendered"].as_str().map(String::from));
		for span in message["spans"].as_array().into_iter().flatten() {
			if span["file_name"].as_str().is_some_and(|file| file.ends_with("src/lib.rs")) {
				lines.extend(span["line_start"].as_u64().map(|line| line as usize));
			}
		}
	}
	(lines, rendered)
}
