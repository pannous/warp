// TEMPORARY probe (card play-block-overload): the lowered program's play definitions and calls, printed by a failing test
#[test]
fn probe_lowered_play() {
	for code in [
		"use sound\nplay(440Hz, 0.1s)\n1",
		"p(f: (float) -> float) := 1; p(f: (int, float) -> any) := 2; hum(t) := t; p(hum)",
	] {
		let lowered = match warp::pipeline::lower(code) {
			Ok(node) | Err(node) => node.serialize(),
		};
		let lines: Vec<&str> = lowered
			.lines()
			.filter(|line| {
				line.contains("play") || line.contains("p·") || line.contains("hum") || line.contains("closure")
			})
			.collect();
		eprintln!("==== {code}\n{}", lines.join("\n"));
	}
	panic!("probe");
}
