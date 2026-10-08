//! card task-sample: Safari 27 throws "WebAssembly.Module.imports unable to produce import descriptors" (and the same
//! for exports) for any module whose imported functions take or give a GC reference (anyref, eqref), so every task
//! program failed in the playground before it started. The page reads import names from the module's bytes
//! (host.js importDescriptors) and export names from the instance.
const PLAYGROUND: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/web/playground");
const REFLECTION_CALLS: [&str; 2] = ["WebAssembly.Module.imports(", "WebAssembly.Module.exports("];

#[test]
fn the_page_never_asks_the_engine_for_import_or_export_descriptors() {
	let scripts = std::fs::read_dir(PLAYGROUND).unwrap().map(|entry| entry.unwrap().path()).filter(|path| path.extension().is_some_and(|extension| extension == "js"));
	let callers: Vec<String> = scripts.filter(|path| {
		let source = std::fs::read_to_string(path).unwrap();
		REFLECTION_CALLS.iter().any(|call| source.contains(call))
	}).map(|path| path.display().to_string()).collect();
	assert!(callers.is_empty(), "Safari 27 cannot describe GC-typed imports/exports: {callers:?}");
}

// Safari's own engine, where macOS has its shell: the page's import reader on a compiled task program
#[cfg(all(feature = "native", target_os = "macos"))]
#[test]
fn safaris_engine_reads_the_imports_of_a_task_program() {
	use std::path::PathBuf;
	const SAFARI_SHELL: &str = "/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/Helpers/jsc";
	if !std::path::Path::new(SAFARI_SHELL).exists() {
		return;
	}
	let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	let source = directory.join("safari_threads.wasp");
	std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/threads.wasp"), &source).unwrap();
	let compiled = crate::common::warp_command().args(["compile", "--wasm"]).arg(&source).output().unwrap();
	assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));

	let host = std::fs::read_to_string(format!("{PLAYGROUND}/host.js")).unwrap();
	let start = host.find("const WASM_HEADER_BYTES").unwrap();
	let end = start + host[start..].find("\n}\n").unwrap() + 2;
	let script = directory.join("safari_imports.js");
	let reader = &host[start..end];
	std::fs::write(&script, format!("const decode = bytes => String.fromCharCode(...bytes);\n{reader}\n\
		const bytes = read(arguments[0], 'binary');\n\
		new WebAssembly.Module(bytes);\n\
		print(importDescriptors(bytes).map(entry => entry.name).join(' '));\n")).unwrap();
	let shown = std::process::Command::new(SAFARI_SHELL).arg(&script).arg("--").arg(source.with_extension("wasm")).output().unwrap();
	let names = String::from_utf8_lossy(&shown.stdout);
	assert!(shown.status.success() && names.contains("task_spawn"), "{names}{}", String::from_utf8_lossy(&shown.stderr));
}
