// The language guide on the playground page (web/playground/guide.md): every ```warp => value fence shows that value
// in the page, a ```printed fence after a snippet is what it prints, and every example a chapter links exists in the
// tour or samples/
use std::collections::HashSet;
use warp::web::evaluate;

const GUIDE: &str = include_str!("../../web/playground/guide.md");
const TOUR: &str = include_str!("../../web/playground/examples.js");
const EXCLUDED_SAMPLES: &str = include_str!("../../web/playground/excluded_samples.txt");
const FENCE_START: &str = "```warp";
const VALUE_MARK: &str = " => ";
const FENCE_END: &str = "```";
const PRINTED_FENCE: &str = "```printed\n";
#[cfg(feature = "native")]
const RESULT_PROMPT: &str = "» "; // the CLI's result line, after what the program printed
const EXAMPLES_LINE: &str = "Examples: ";

struct Snippet {
	code: &'static str,
	value: Option<&'static str>,
	#[cfg_attr(not(feature = "native"), allow(dead_code))] // read by the native test only
	printed: Option<&'static str>,
}

fn snippets() -> Vec<Snippet> {
	GUIDE.split(FENCE_START).skip(1).map(|fence| {
		let (info, rest) = fence.split_once('\n').unwrap();
		let (code, after) = rest.split_once(FENCE_END).unwrap();
		let printed = after.strip_prefix('\n').and_then(|next| next.strip_prefix(PRINTED_FENCE)).map(|block| block.split(FENCE_END).next().unwrap());
		Snippet { code, value: info.strip_prefix(VALUE_MARK), printed }
	}).collect()
}

// `Examples: welcome, "element events"; samples: hello` → tour names and sample names
fn linked_examples() -> (Vec<String>, Vec<String>) {
	let names = |list: &str| list.split(", ").map(|name| name.trim().trim_matches('"').to_string()).collect::<Vec<_>>();
	let (mut tour, mut samples) = (vec![], vec![]);
	for line in GUIDE.lines().filter_map(|line| line.strip_prefix(EXAMPLES_LINE)) {
		let (tour_part, samples_part) = line.split_once("; samples: ").unwrap_or((line, ""));
		tour.extend(names(tour_part));
		samples.extend(names(samples_part).into_iter().filter(|name| !name.is_empty()));
	}
	(tour, samples)
}

#[test]
fn every_guide_snippet_shows_its_value() {
	let failures: Vec<String> = snippets().into_iter().filter_map(|Snippet { code, value, .. }| {
		let expected = value?;
		let shown = evaluate(code, HashSet::new())["value"].as_str().unwrap_or_default().to_string();
		(shown != expected).then(|| format!("{code}  shows {shown}, the guide says {expected}"))
	}).collect();
	assert!(snippets().len() > 30);
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[cfg(feature = "native")] // runs the warp binary
#[test]
fn every_guide_snippet_prints_what_the_guide_shows() {
	let failures: Vec<String> = snippets().into_iter().filter_map(|Snippet { code, printed, .. }| {
		let expected = printed?;
		let output = crate::common::printed(code);
		let shown: String = output.lines().filter(|line| !line.starts_with(RESULT_PROMPT)).map(|line| format!("{line}\n")).collect();
		(shown != expected).then(|| format!("{code}  prints {shown:?}, the guide says {expected:?}"))
	}).collect();
	assert!(snippets().iter().filter(|snippet| snippet.printed.is_some()).count() > 5);
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_example_a_chapter_links_exists() {
	let (tour, samples) = linked_examples();
	for name in &tour {
		let key = if name.contains(' ') { format!("\t\"{name}\": {{") } else { format!("\t{name}: {{") };
		assert!(TOUR.contains(&key), "the guide links the tour example {name}, examples.js has none");
	}
	for name in &samples {
		assert!(std::path::Path::new(&format!("samples/{name}.warp")).exists(), "the guide links samples/{name}.warp");
		assert!(!EXCLUDED_SAMPLES.lines().any(|line| line.starts_with(&format!("{name}\t"))), "{name} is not in the playground menu");
	}
}
