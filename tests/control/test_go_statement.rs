// Card go-play: a go block of one statement runs it like a block of several: `go { play "x.wav" }` plays, it did not
// return the data `{play "x.wav"}`
use warp::wasm_emitter::eval;

const MISSING_FILE_ERROR: &str = "cannot open x.wav";

fn played(program: &str) -> bool {
	eval(program).serialize().contains(MISSING_FILE_ERROR)
}

#[test]
fn a_go_block_of_one_statement_runs_it() {
	assert!(played("job = go { play \"x.wav\" }; await job"));
	assert!(played("job = go {\nplay \"x.wav\"\n}; await job"));
}
