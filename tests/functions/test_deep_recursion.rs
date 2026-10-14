// The warp command gives wasm most of its main thread's stack (card deep-recursion): recursion a million frames
// deep, as a recursive descent parser over a long input needs; test threads keep wasmtime's default stack
#[cfg(feature = "native")]
#[test]
fn the_warp_command_recurses_a_million_frames_deep() {
	let output = crate::common::warp_command()
		.arg("down(n) := if n == 0 then 0 else 1 + down(n - 1); down(1000000)")
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.unwrap();
	let printed = String::from_utf8_lossy(&output.stdout);
	assert!(printed.contains("1000000"), "{printed}{}", String::from_utf8_lossy(&output.stderr));
}
