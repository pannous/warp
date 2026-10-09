// card compiler-picks-dot: plain `xs = float[n]` arrays paired by dot or `.*` live in linear memory by themselves,
// so dot runs as linear_dotf (10^6: 3 ms instead of 13 ms over GC lists), when every use is one linear arrays support
use crate::is;
use warp::{float, int, list};

const FILLED: &str = "xs = float[3]; ys = float[3]; xs#1 = 2.0; ys#1 = 3.0; xs#2 = 4.0; ys#2 = 0.5\n";

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program").serialize()
}

#[test]
fn float_arrays_paired_by_dot_are_linear() {
	assert!(lowered(&format!("{FILLED}dot(xs, ys)")).contains("linear_dotf"));
	assert!(lowered(&format!("{FILLED}sum(xs .* ys)")).contains("linear_dotf"));
	is!(&format!("{FILLED}dot(xs, ys)"), 8.0);
	is!(&format!("{FILLED}sum(xs .* ys)"), 8.0);
	is!(&format!("{FILLED}zs = xs .* ys; [zs#1, zs#2, #zs]"), list(vec![float(6.0), float(2.0), int(3)]));
	is!(&format!("{FILLED}s = 0.0; for x in xs {{ s += x }}; dot(xs, ys) + s + count(ys) + xs.count"), 20.0);
}

#[test]
fn float_arrays_used_otherwise_stay_lists() {
	assert!(!lowered(&format!("{FILLED}xs = xs + [1.0]; dot(xs, ys)")).contains("linear_dotf"));
	assert!(!lowered(&format!("{FILLED}print(xs); dot(xs, ys)")).contains("linear_dotf"));
	assert!(!lowered("xs = float[3]; ys = [1.0, 2.0, 3.0]; dot(xs, ys)").contains("linear_dotf"));
	is!(&format!("{FILLED}xs = xs + [1.0]; #xs"), 4);
}

// the hint against `linear` names the array's own element type
#[test]
fn the_linear_hint_names_float_arrays() {
	use warp::normalize::{capture_hints, clear_shown_hints};
	clear_shown_hints();
	let (_, hints) = capture_hints(|| warp::wasm_emitter::eval("linear xs = float[3]; xs#1 = 2.0; xs#1"));
	assert!(hints.iter().any(|hint| hint.original == "linear xs = float[n]" && hint.canonical == "xs = float[n]"), "{hints:?}");
}
