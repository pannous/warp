// card cleanup-fresh-names: the temporaries a compile invents (`race·0`, `item·0`) are numbered per compile, so the same
// program lowers the same way whatever compiled before it

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program").serialize()
}

#[test]
fn a_program_lowers_alike_on_every_compile() {
	let programs = [
		"f(x) := x * 2; a = go f(1); b = go f(2); await any [a, b]",
		"names = match [[1, \"a\"], [2, \"b\"]] { [[n, s]...] => s }\nnames#2",
	];
	for program in programs {
		assert_eq!(lowered(program), lowered(program), "{program}");
	}
}
