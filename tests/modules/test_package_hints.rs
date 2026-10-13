// card hints-fire: style hints are for the program's own files; a fetched package's source (uniscript.wasp writes
// `fewest[0]`) is not the user's to change, so it says nothing there
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

#[test]
fn a_package_s_style_gets_no_hints_the_program_s_does() {
	let (_, hints) = capture_hints(|| eval("use uniscript; xs = [1, 2]; xs[0]"));
	let originals: Vec<&str> = hints.iter().map(|hint| hint.original.as_str()).collect();
	assert_eq!(originals, ["xs[0]"], "{hints:?}");
}
