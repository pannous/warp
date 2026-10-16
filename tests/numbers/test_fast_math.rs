//! `use fast_math` and `warp --fast` rebind sin cos exp pow atan2 to wasm polynomials (lib/fast_math.warp, card
//! switch-rebinds); the errors its header states, here at a few points against the C library's digits
use crate::is;

#[test]
fn fast_words_are_close_to_the_c_library() {
	is!("use fast_math; x = 1.0; abs(sin(x) - 0.8414709848078965) < 1e-12", true);
	is!("use fast_math; x = 1000.0; abs(sin(x) - 0.8268795405320025) < 1e-11", true);
	is!("use fast_math; x = -2.0; abs(cos(x) - -0.4161468365471424) < 1e-12", true);
	is!("use fast_math; x = 1.0; abs(exp(x) - 2.718281828459045) < 1e-14", true);
	is!("use fast_math; x = -700.0; abs(exp(x) / 9.85967654375977e-305 - 1) < 1e-13", true);
	is!("use fast_math; x = 2.0; abs(pow(x, 0.5) - 1.4142135623730951) < 1e-14", true);
	is!("use fast_math; pow(-2.0, 3.0)", -8.0);
	is!("use fast_math; atan2(0.0, 0.0)", 0.0);
	is!("use fast_math; x = -1.0; abs(atan2(x, -2.0) - -2.677945044588987) < 1e-12", true);
	is!("use fast_math; x = 3.0; abs(atan2(x, 1.0) - 1.2490457723982544) < 1e-12", true);
}

#[test]
fn fast_words_reach_the_math_module_and_stay_out_of_other_programs() {
	is!("use fast_math; use math; x = 1.0; abs(sine(x) - 0.8414709848078965) < 1e-12", true);
	is!("x = 1.0; sin(x) == 0.8414709848078965", true);
}

#[cfg(feature = "native")]
#[test]
fn the_fast_flag_uses_fast_math_also_for_a_kept_program_file() {
	let folder = crate::common::scratch_directory("fast_flag");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("fast.warp");
	std::fs::write(&program, "x = 1.0\nprint sin(x) - 0.8414709848078965\n").unwrap();
	let printed = |flags: &[&str]| {
		let output = crate::common::warp_command().args(flags).arg(&program).output().expect("warp runs");
		String::from_utf8_lossy(&output.stdout).trim().to_string()
	};
	assert_eq!(printed(&["--no-ask"]), "0");
	assert_ne!(printed(&["--no-ask", "--fast"]), "0");
}
