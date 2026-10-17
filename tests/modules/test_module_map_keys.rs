//! card map-key: a module function's word renamed apart from the program's (`lib·total`) keeps the keys of the maps it
//! builds, `{total: …}` stays `total`, not `lib·total` (lib/shell.warp's wc had to quote its keys)
use warp::wasm_emitter::eval;

fn run_beside(folder_name: &str, module: &str, program: &str) -> String {
	let folder = crate::common::scratch_directory(folder_name);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	std::fs::write(folder.join("stats.warp"), module).expect("the module file");
	warp::modules::with_program_file(&folder.join("app.warp"), || eval(program)).serialize()
}

#[test]
fn a_renamed_local_keeps_its_map_key() {
	let module = "summary(xs) := { total = sum(xs); {total: total, size: count(xs)} }";
	let result = run_beside("module_map_keys", module, "use stats\ntotal = 5\nsummary([1 2])");
	assert_eq!(result, "{total:3 size:2}");
}
