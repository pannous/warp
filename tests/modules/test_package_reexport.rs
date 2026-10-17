// card module-own, decision package-reexport: what a module itself uses reaches the programs that use it, its classes
// too (draw's Picture, which canvas() and save make)
use warp::wasm_emitter::eval;

fn run_beside_shapes(code: &str) -> String {
	let folder = crate::common::scratch_directory("package_reexport");
	std::fs::create_dir_all(&folder).expect("scratch folder");
	std::fs::write(folder.join("shapes.warp"), "use draw\nshape(n) := n * 2\n").expect("the module file");
	warp::modules::with_program_file(&folder.join("app.warp"), || eval(code)).serialize().trim().to_string()
}

#[test]
fn a_modules_own_use_reaches_its_users() {
	assert_eq!(run_beside_shapes("use shapes\nshape(3)"), "6");
	assert_eq!(run_beside_shapes("use shapes\ncanvas(200, 100)\ncanvas.width"), "200");
	assert_eq!(run_beside_shapes("use shapes\nc = canvas()\nc is Picture"), "yes");
}
