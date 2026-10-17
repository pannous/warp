// `%`, `rem` and `//` of Ints beyond the fixnums take the small int_* runtime; a module that makes ratios keeps the
// ratio-aware exact_* one (card int-i64)
use crate::is;

fn runtime_names(code: &str) -> String {
	let bytes = warp::wasm_emitter::compile(code).expect("compiles").bytes;
	String::from_utf8_lossy(&bytes).into_owned()
}

#[test]
fn int_division_is_euclidean() {
	is!("-7 % 3", 2);
	is!("7 % -3", 1);
	is!("-7 rem 3", -1);
	is!("a = -7; a/!2", -4);
	is!("2^70 % 7", 2);
	is!("n = -(2^70); n % 1000", 576);
	is!("(2^70 + 5) rem 7", 0);
	is!("n = -(2^70); n/!3 == -393530540239137101142", true);
}

#[test]
fn ratios_keep_their_division() {
	is!("x = -7.5; x % 2", 0.5);
	is!("x = 1/2; x % 3", 0.5);
	is!("x = 1/2; x/!1", 0);
	// draw's hsv takes `h % 360` of a ratio in a module the emitter emits twice for its own needs (samples/sound/visualizer.warp)
	is!("use draw\ncolors = [hsv(band * 280 / 16, 1, 1) for band in 0..16]\ncolors#4", 4294958848i64);
}

#[test]
fn int_programs_skip_the_ratio_runtime() {
	let names = runtime_names("x = 255; print(x % 16)");
	assert!(names.contains("int_mod") && !names.contains("exact_mod"), "int_mod without exact_mod");
	assert!(runtime_names("x = 1/2; x % 3").contains("exact_mod"));
}
