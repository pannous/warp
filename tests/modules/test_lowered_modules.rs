// card prelude-cache-runs: a standard module's lowered source is kept on disk (src/modules/lowered_modules.rs), so the
// next program using it skips parsing and lowering it; the module it compiles to stays byte for byte the same
#![cfg(feature = "native")]
use std::path::{Path, PathBuf};

/// sound's panned() subtracts `sample - sample_offset` with a spaced minus, a parser mark (SpacedMinus) the kept module
/// must keep, else the warm compile calls sample(-sample_offset)
const PROGRAM: &str = "use sound\npanned([1, 2])";

/// A folder for one test's program and its own cache
fn folder(name: &str) -> PathBuf {
	let folder = crate::common::scratch_directory(name);
	std::fs::create_dir_all(folder.join("cache")).unwrap();
	std::fs::write(folder.join("program.warp"), PROGRAM).unwrap();
	folder
}

/// The module `warp compile --wasm program.warp` writes with the cache setting
fn compiled(folder: &Path, cache: &str) -> Vec<u8> {
	let run = crate::common::warp_command()
		.args(["--no-ask", "compile", "--wasm", "program.warp"])
		.current_dir(folder)
		.env(warp::run::module_cache::CACHE_VARIABLE, cache)
		.output()
		.unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	std::fs::read(folder.join("program.wasm")).unwrap()
}

fn kept_modules(folder: &Path) -> usize {
	let lowered = Some(std::ffi::OsStr::new("lowered"));
	std::fs::read_dir(folder.join("cache"))
		.unwrap()
		.filter(|entry| entry.as_ref().unwrap().path().extension() == lowered)
		.count()
}

#[test]
fn a_kept_lowered_module_compiles_to_the_same_module() {
	let folder = folder("lowered_modules_same");
	let cache = folder.join("cache").display().to_string();
	let uncached = compiled(&folder, "off");
	let cold = compiled(&folder, &cache);
	assert!(kept_modules(&folder) >= 1, "use sound keeps lib/sound.warp lowered");
	let warm = compiled(&folder, &cache);
	assert!(cold == uncached, "the cold compile differs from the uncached one");
	assert!(warm == cold, "the compile from the kept module differs from the cold one");
}
