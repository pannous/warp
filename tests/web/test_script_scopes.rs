// The playground's classic scripts share one global scope per page or Worker: a top-level name declared twice is a
// SyntaxError (const, let, class) or silently replaces the first (function). playground-live's isLive clashed with
// markup.js and only CI's tours caught it (card test-page). Generated scripts (keywords.js, samples.js …) are skipped
// when not built.
use std::collections::HashMap;
use std::path::Path;

const PLAYGROUND: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/web/playground");
const PAGE: &str = "index.html";
const WORKERS: [&str; 3] = ["worker.js", "test-worker.js", "task-worker.js"];
const THIRD_PARTY: &str = "codemirror/";
const DECLARATION_WORDS: [&str; 5] = ["function", "const", "let", "var", "class"];
const HOST_PARTS: &str = "HOST_PART_FILES";
/// functions a later script replaces on purpose, by its file
const REPLACEMENTS: [(&str, &[&str]); 1] =
	[("markup-transitions.js", &["transitionPlaces", "enter", "leave", "moveFrom"])];

fn source(file: &str) -> Option<String> {
	std::fs::read_to_string(Path::new(PLAYGROUND).join(file)).ok()
}

/// the quoted names ending in .js in `text`, in order
fn quoted_scripts(text: &str, quote: char) -> Vec<String> {
	text.split(quote).skip(1).step_by(2).filter(|word| word.ends_with(".js")).map(String::from).collect()
}

fn page_scripts() -> Vec<String> {
	let page = source(PAGE).expect(PAGE);
	page.lines()
		.filter_map(|line| line.split("<script src=\"").nth(1))
		.filter_map(|rest| rest.split('"').next())
		.filter(|script| !script.starts_with(THIRD_PARTY))
		.map(String::from)
		.collect()
}

/// a Worker's script and the scripts it imports, host.js's HOST_PART_FILES where it spreads them
fn worker_scripts(worker: &str) -> Vec<String> {
	let text = source(worker).expect(worker);
	let mut scripts = vec![worker.to_string()];
	for line in text.lines().filter(|line| line.starts_with("importScripts(") || line.contains(" importScripts(")) {
		scripts.extend(quoted_scripts(line, '"'));
		if line.contains(HOST_PARTS) {
			let host = source("host.js").expect("host.js");
			let parts = host.lines().find(|line| line.starts_with(&format!("const {HOST_PARTS} ="))).expect(HOST_PARTS);
			scripts.extend(quoted_scripts(parts, '"'));
		}
	}
	scripts
}

/// the top-level names a script declares (lines that start unindented with a declaration word), with that word
fn top_level_names(text: &str) -> Vec<(String, &'static str)> {
	let mut names = vec![];
	for line in text.lines() {
		let line = line.strip_prefix("async ").unwrap_or(line);
		let Some(word) = DECLARATION_WORDS.iter().find(|word| line.starts_with(&format!("{word} "))) else { continue };
		let rest = line[word.len()..].trim_start().trim_start_matches('*').trim_start();
		let declared = match rest.chars().next() {
			Some('[') | Some('{') => rest[1..]
				.split([']', '}'])
				.next()
				.unwrap_or("")
				.split(',')
				.map(|part| {
					let part = part.rsplit(':').next().unwrap_or(part);
					part.split('=').next().unwrap_or(part).trim().to_string()
				})
				.collect(),
			_ => vec![rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect()],
		};
		names.extend(declared.into_iter().filter(|name: &String| !name.is_empty()).map(|name| (name, *word)));
	}
	names
}

fn is_replacement(file: &str, name: &str) -> bool {
	REPLACEMENTS.iter().any(|(replacing, names)| *replacing == file && names.contains(&name))
}

/// "name: first.js, second.js" for each name declared twice in one scope
fn twice_declared(scripts: &[String]) -> Vec<String> {
	let mut declared_in: HashMap<String, (String, &str)> = HashMap::new();
	let mut clashes = vec![];
	for script in scripts {
		let Some(text) = source(script) else { continue }; // a generated script not built here
		for (name, word) in top_level_names(&text) {
			match declared_in.get(&name) {
				Some((first, first_word))
					if !(word == "function" && *first_word == "function" && is_replacement(script, &name)) =>
				{
					clashes.push(format!("{name}: {first}, {script}"))
				}
				Some(_) => {}
				None => {
					declared_in.insert(name, (script.clone(), word));
				}
			}
		}
	}
	clashes
}

#[test]
fn the_page_scripts_declare_no_name_twice() {
	let scripts = page_scripts();
	assert!(
		scripts.contains(&"markup.js".to_string()) && scripts.contains(&"playground.js".to_string()),
		"{scripts:?}"
	);
	assert_eq!(twice_declared(&scripts), Vec::<String>::new());
}

#[test]
fn the_worker_scripts_declare_no_name_twice() {
	for worker in WORKERS {
		let scripts = worker_scripts(worker);
		assert!(scripts.contains(&"host-tasks.js".to_string()), "{worker}: {scripts:?}");
		assert_eq!(twice_declared(&scripts), Vec::<String>::new(), "{worker}");
	}
}

#[test]
fn a_name_declared_twice_is_found() {
	let clash = top_level_names("const isLive = 1;\nfunction isLive() {}\n  const inner = 2;\nconst [a, b] = [1, 2];");
	let names: Vec<&str> = clash.iter().map(|(name, _)| name.as_str()).collect();
	assert_eq!(names, ["isLive", "isLive", "a", "b"]);
	let loaded_twice = ["markup.js".to_string(), "markup.js".to_string()];
	assert!(twice_declared(&loaded_twice).iter().any(|clash| clash.starts_with("transitionPlaces: ")));
	let replaced = ["markup.js".to_string(), "markup-transitions.js".to_string()];
	assert_eq!(twice_declared(&replaced), Vec::<String>::new());
}
