// TEMPORARY probe (card play-block-overload): the lowered program's play definitions and calls, printed by a failing test
#[test]
fn probe_lowered_play() {
	for code in ["play(C4, 10ms)\nbeep\n7", "play(C4, 10ms)\n7", "beep\n7"] {
		let lowered = match warp::pipeline::lower(code) {
			Ok(node) | Err(node) => node.serialize(),
		};
		let wanted = ["play", "tone", "beep"];
		let lines: Vec<&str> = lowered.lines().filter(|line| wanted.iter().any(|word| line.contains(word))).collect();
		eprintln!("==== {code}\n{}\n---- value: {:?}", lines.join("\n"), warp::pipeline::eval(code));
	}
	panic!("probe");
}
