//! `fast` on a function, a program or a module (user 2026-10-16, card fast-scope): sin cos exp pow atan2 are
//! lib/fast_math.warp's polynomials, Ints machine i64s that wrap, `/` of Ints a float; the rest stays exact.
//! `x: fast int` is i64, `x: fast float` f64
use crate::is;

#[test]
fn a_fast_function_is_machine_arithmetic() {
	is!("fast g(n) := n * n; g(10^10)", 7766279631452241920i64);
	is!("fast k(n) := n + 1; k(2^63 - 1)", -9223372036854775808i64);
	is!("fast f(x) := x/3; f(1)", 0.3333333333333333);
	is!("fast g(n) := n * n; g(10^10); 10^10 * 10^10 == 10^20", true);
}

#[test]
fn a_fast_function_takes_the_polynomial_math_alone() {
	is!("fast f(x) := sin(x); f(1.0) == 0.8414709848078937", true);
	is!("fast f(x) := sin(x); f(1.0); sin(1.0) == 0.8414709848078965", true);
}

#[test]
fn a_fast_program_is_machine_arithmetic() {
	is!("fast\nx = 2^62\nx * 4", 0);
	is!("fast\nx = 1.0\nsin(x) == 0.8414709848078937", true);
	is!("fast = 2; fast + 1", 3);
}

#[test]
fn fast_types_are_machine_types() {
	is!("x: fast int = 3; x + 1", 4);
	is!("x: fast float = 1/3; x", 0.3333333333333333);
}

#[cfg(feature = "native")]
#[test]
fn a_fast_module_makes_its_functions_fast() {
	let folder = crate::common::scratch_directory("fast_module");
	std::fs::create_dir_all(&folder).unwrap();
	std::fs::write(folder.join("fm.warp"), "fast\nsquare(n) := n * n\nwave(x) := sin(x)\n").unwrap();
	let program = folder.join("main.warp");
	std::fs::write(
		&program,
		"use fm\nprint [square(10^10), wave(1.0) == 0.8414709848078937, sin(1.0) == 0.8414709848078965]\n",
	)
	.unwrap();
	let output = crate::common::warp_command().arg("--no-ask").arg(&program).output().expect("warp runs");
	assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "[7766279631452241920 yes yes]");
}
