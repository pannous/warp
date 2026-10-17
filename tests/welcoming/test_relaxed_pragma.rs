//! `use relaxed` (aliases `use lenient`, `use loose`), the mirror of `use strict`: the program shows no hints, as
//! `--no-hints` does on the command line; its warnings and values stay
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

fn hint_count(code: &str) -> usize {
	set_hint_mode(HintMode::Always);
	capture_hints(|| eval(code)).1.len()
}

#[test]
fn a_relaxed_program_shows_no_hints() {
	for hinted in ["2**3", "3 == int"] {
		assert!(hint_count(hinted) > 0, "{hinted} is hinted without the pragma");
		for pragma in ["use relaxed", "use lenient", "use loose"] {
			assert_eq!(hint_count(&format!("{pragma}\n{hinted}")), 0, "{pragma}: {hinted}");
		}
	}
	is!("use relaxed\n2**3", 8);
}
