//! card sugar-twins: what each pass of SUGAR_PASSES (src/pipeline.rs) changes in the sources, to sort the passes into
//! ones to port, drop, move to a library or merge (notes/self_hosting_dropped.md)
use crate::common::twins::sources;
use warp::pipeline::{around_source_pass, SUGAR_PASSES};

/// Per pass: its index in SUGAR_PASSES, how many sources it changes and the first of them
const SURVEY: &str = "data/self_passes/sugar_survey.txt";
/// Sources named per pass
const SHOWN: usize = 4;

/// `tests/queue.sh -- sugar_pass_survey --ignored`
#[test]
#[ignore]
fn sugar_pass_survey() {
	let texts: Vec<_> =
		sources().into_iter().filter_map(|path| Some((std::fs::read_to_string(&path).ok()?, path))).collect();
	let lines: Vec<String> = SUGAR_PASSES
		.iter()
		.enumerate()
		.map(|(index, pass)| {
			let changed: Vec<String> = texts
				.iter()
				.filter(|(text, _)| {
					let around = std::panic::catch_unwind(|| around_source_pass(text, *pass)).ok().flatten();
					around.is_some_and(|(before, after)| before.serialize() != after.serialize())
				})
				.map(|(_, path)| path.display().to_string())
				.collect();
			let shown: Vec<&str> = changed.iter().take(SHOWN).map(String::as_str).collect();
			format!("{index}\t{}\t{}", changed.len(), shown.join(" "))
		})
		.collect();
	std::fs::create_dir_all(std::path::Path::new(SURVEY).parent().unwrap()).unwrap();
	std::fs::write(SURVEY, lines.join("\n") + "\n").unwrap();
	println!("{}", lines.join("\n"));
}
