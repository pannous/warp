// TEMPORARY probe (card play-block-overload): the lowered program's play definitions and calls, printed by a failing test
#[test]
fn probe_lowered_play() {
	for code in [
		"play(C4, 10ms)\n7",
		"use sound\nhum(block: samples, t) := for i in 1 to #block { block#i = 0.5 * t }\nplay(hum, 0.1s)\n5",
	] {
		let lowered = match warp::pipeline::lower(code) {
			Ok(node) | Err(node) => node.serialize(),
		};
		let wanted = ["play", "hum", "pitches_of", "is_signal"];
		let lines: Vec<&str> = lowered.lines().filter(|line| wanted.iter().any(|word| line.contains(word))).collect();
		eprintln!("==== {code}\n{}", lines.join("\n"));
	}
	panic!("probe");
}
