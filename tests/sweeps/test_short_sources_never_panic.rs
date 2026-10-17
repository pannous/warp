// card crash-closure: a three-character program crashed the parser ("range end index 4 out of range for slice of
// length 3" in at_pseudo_class_selector): an operator ran the position past the end of the source
use warp::parse;

const ALPHABET: &str = "a1 :.-+*/=<>!?&|^%#@$~,;'\"()[]{}\\—→…\n";
const LONGEST_SOURCE: usize = 3;

fn sources(length: usize) -> Vec<String> {
	if length == 0 {
		return vec![String::new()];
	}
	sources(length - 1).into_iter().flat_map(|prefix| ALPHABET.chars().map(move |ch| format!("{prefix}{ch}"))).collect()
}

#[test]
fn short_sources_never_panic() {
	let quiet_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(|_| {}));
	let panicking: Vec<String> = (1..=LONGEST_SOURCE)
		.flat_map(sources)
		.filter(|source| std::panic::catch_unwind(|| parse(source)).is_err())
		.collect();
	std::panic::set_hook(quiet_hook);
	assert!(
		panicking.is_empty(),
		"{} sources crash the parser, e.g. {:?}",
		panicking.len(),
		&panicking[..panicking.len().min(20)]
	);
}
