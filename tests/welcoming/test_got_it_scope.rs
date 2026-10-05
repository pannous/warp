// "got it" per expression (user, 2026-10-05): the prompt offers [y this / a all of this kind / n]; "this" remembers one
// expression by its written text, "all" the whole topic; a `// got it` comment on its line silences a warning there
use warp::diagnostic::{take_warnings, with_acknowledger, Acknowledger, GotIt};
use warp::wasm_emitter::eval;

const UPTO_A: &str = "x=0; for i in 1 upto 4 {x+=i}; x";
const UPTO_B: &str = "y=0; for j in 2 upto 5 {y+=j}; y";

/// Answers every "got it?" the same way
struct Answering(GotIt);

impl Acknowledger for Answering {
	fn acknowledge(&self, _topic: &str, _written: &str) -> GotIt {
		self.0
	}
}

fn upto_warnings(code: &str) -> usize {
	take_warnings();
	eval(code);
	take_warnings().iter().filter(|warning| warning.message.contains("upto")).count()
}

#[test]
fn got_it_for_this_silences_only_that_expression() {
	with_acknowledger(Answering(GotIt::This), || {
		assert_eq!(upto_warnings(UPTO_A), 1, "shown, then acknowledged for this expression");
		assert_eq!(upto_warnings(UPTO_A), 0, "the same expression stays quiet");
		assert_eq!(upto_warnings(UPTO_B), 1, "another expression of the kind still warns");
	});
}

#[test]
fn got_it_for_all_silences_the_kind() {
	with_acknowledger(Answering(GotIt::All), || {
		assert_eq!(upto_warnings(UPTO_A), 1);
		assert_eq!(upto_warnings(UPTO_B), 0);
	});
}

#[test]
fn no_keeps_reminding() {
	with_acknowledger(Answering(GotIt::No), || {
		assert_eq!(upto_warnings(UPTO_A), 1);
		assert_eq!(upto_warnings(UPTO_A), 1);
	});
}

#[test]
fn a_got_it_comment_silences_its_line() {
	with_acknowledger(Answering(GotIt::No), || {
		assert_eq!(upto_warnings("x=0; for i in 1 upto 4 {x+=i}; x // got it"), 0);
		assert_eq!(upto_warnings("// got it\nx=0; for i in 1 upto 4 {x+=i}; x"), 1, "only its own line");
	});
}
