// card browser-play: in the browser play of a local file that is not there fails as natively ("cannot open …"), asked
// with a HEAD request (host-files.js servedFileExists), not by downloading the song; a written file plays from its bytes
// (test_page_rendered_sound)
#![cfg(not(feature = "native"))]
use warp::wasm_emitter::eval;

#[test]
fn playing_a_missing_file_fails() {
	let shown = eval("play \"no/such.wav\"").serialize();
	assert!(shown.contains("cannot open no/such.wav"), "{shown}");
}
