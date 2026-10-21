// card rust-lane-use: a use cycle loads each file once: a.warp uses b.warp, which uses a.warp back
use warp::wasm_emitter::eval;

#[test]
fn a_module_using_the_program_back() {
	let folder = crate::common::scratch_directory("use_cycle");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	let program = "use b.warp\nfa() := 1\nfb() + 1";
	std::fs::write(folder.join("a.warp"), program).expect("the program file");
	std::fs::write(folder.join("b.warp"), "use a.warp\nfb() := 2\n").expect("the module file");
	let result = warp::modules::with_program_file(&folder.join("a.warp"), || eval(program));
	assert_eq!(result.serialize().trim(), "3");
}
