// card web-bundle: a built site ships only the parts of host.js its module imports words of (src/site.rs HOST_PARTS,
// host.js addHostPart); the playground's workers load them all (host.js HOST_PART_FILES)
use warp::site::{scripts_of, HOST_PARTS};

const HOST_SCRIPT: &str = include_str!("../../web/playground/host.js");
const PART_KEYS: [&str; 2] = ["words: ", "\n\t}"];

fn part_files(code: &str) -> Vec<&'static str> {
	let module = warp::pipeline::for_a_page(|| warp::pipeline::compile(code)).expect("compiled");
	let scripts = scripts_of(&module.bytes, false).expect("the module's scripts");
	scripts.into_iter().map(|(name, _)| name).filter(|name| name.starts_with("host-")).collect()
}

#[test]
fn a_hello_world_page_ships_no_part_of_the_host() {
	assert_eq!(part_files("p{ \"hello world\" }"), Vec::<&str>::new());
}

#[test]
fn a_page_ships_the_parts_its_module_imports() {
	assert_eq!(part_files("twice(x) := x * 2\ntask = go twice(21)\np{ await task }"), ["host-tasks.js"]);
	assert_eq!(part_files("use hash\np{ sha256(\"a\") }"), ["host-hashes.js"]);
	assert_eq!(part_files("use file\nwrite(\"a.txt\", \"x\")\np{ exists(\"a.txt\") }"), ["host-files.js"]);
	assert_eq!(part_files("use js Math\np{ Math.floor(2.5) }"), ["host-files.js", "host-foreign.js"]);
}

// the host words a part gives (its `words` object), as each part's selection must recognize them
fn words_of(part: &str) -> Vec<String> {
	let start = part.find("addHostPart({").expect("the part adds itself");
	let Some(words) = part[start..].find(PART_KEYS[0]).map(|offset| &part[start + offset..]) else { return vec![] };
	let words = &words[..words.find(PART_KEYS[1]).expect("the end of the words")];
	let key = |line: &str| line.split_once(": ").map(|(key, _)| key.to_string()).filter(|key| key.chars().all(|c| c.is_ascii_lowercase() || c == '_'));
	let depth = |line: &str| line.len() - line.trim_start_matches('\t').len();
	let keyed: Vec<&str> = words.lines().skip(1).filter(|line| key(line.trim_start()).is_some()).collect();
	let shallowest = keyed.iter().map(|line| depth(line)).min().unwrap_or(0);
	keyed.into_iter().filter(|line| depth(line) == shallowest).filter_map(|line| key(line.trim_start())).collect()
}

#[test]
fn each_part_is_chosen_by_the_words_it_gives() {
	for part in HOST_PARTS {
		let (file, text) = part.script;
		let words = words_of(text);
		assert!(file == "host-hashes.js" || !words.is_empty(), "{file} gives no words");
		for word in words {
			assert!((part.gives)("host", &word), "{file} gives {word}, but src/site.rs HOST_PARTS does not choose it for {word}");
		}
	}
}

#[test]
fn the_sites_parts_are_the_playgrounds_parts() {
	let listed = HOST_SCRIPT.lines().find(|line| line.starts_with("const HOST_PART_FILES")).expect("host.js lists its parts");
	let files: Vec<&str> = HOST_PARTS.iter().map(|part| part.script.0).collect();
	assert_eq!(listed, format!("const HOST_PART_FILES = [{}];", files.iter().map(|file| format!("\"{file}\"")).collect::<Vec<_>>().join(", ")));
}
