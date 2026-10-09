// card std-file: a program folder's own copy of a standard module (collections.warp) works like the embedded one:
// the other languages' names of its classes and methods (HashSet, deque, appendleft) are its words too
use warp::wasm_emitter::eval;

fn with_collections_copy(test: &str, code: &str) -> String {
	let folder = crate::common::scratch_directory(test);
	std::fs::create_dir_all(&folder).expect("scratch folder");
	std::fs::write(folder.join("collections.warp"), include_str!("../../lib/collections.warp")).expect("the copy");
	let result = warp::modules::with_program_file(&folder.join("app.warp"), || eval(code));
	std::fs::remove_dir_all(&folder).ok();
	result.serialize().trim().to_string()
}

#[test]
fn a_copy_of_a_std_module_knows_the_foreign_names() {
	assert_eq!(with_collections_copy("std_file_set", "use collections; s = HashSet([1 2 2]); s.size()"), "2");
	assert_eq!(with_collections_copy("std_file_deque", "use collections; d = deque(); d.append(1); d.appendleft(0); d.popleft() * 10 + d.pop()"), "1");
}
