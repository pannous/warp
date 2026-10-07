// The language guide on the playground page (web/playground/guide.md): every ```wasp => value fence shows that value
// in the page, and every example a chapter links exists in the tour or samples/
use std::collections::HashSet;
use warp::web::evaluate;

const GUIDE: &str = include_str!("../../web/playground/guide.md");
const TOUR: &str = include_str!("../../web/playground/examples.js");
const EXCLUDED_SAMPLES: &str = include_str!("../../web/playground/excluded_samples.txt");
const FENCE_START: &str = "```wasp => ";
const FENCE_END: &str = "```";
const EXAMPLES_LINE: &str = "Examples: ";

fn fences() -> Vec<(&'static str, &'static str)> {
	GUIDE.split(FENCE_START).skip(1).map(|fence| {
		let (expected, rest) = fence.split_once('\n').unwrap();
		(expected, rest.split(FENCE_END).next().unwrap())
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
	let failures: Vec<String> = fences().into_iter().filter_map(|(expected, code)| {
		let shown = evaluate(code, HashSet::new())["value"].as_str().unwrap_or_default().to_string();
		(shown != expected).then(|| format!("{code}  shows {shown}, the guide says {expected}"))
	}).collect();
	assert!(fences().len() > 30);
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
		assert!(std::path::Path::new(&format!("samples/{name}.wasp")).exists(), "the guide links samples/{name}.wasp");
		assert!(!EXCLUDED_SAMPLES.lines().any(|line| line.starts_with(&format!("{name}\t"))), "{name} is not in the playground menu");
	}
}
