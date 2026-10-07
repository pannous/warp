// card entities-index: the parser's `\:name` entities are the single-word names of the pinned uniscript package's
// index (data/entities.idx, format in uniscript's AGENTS.md "Index format"), embedded as src/uniscript_entities.tsv
use crate::is;
use std::path::Path;
use warp::modules::fetch_package_into;

const TABLE: &str = "src/uniscript_entities.tsv";
const REGENERATE: &str = "WARP_REGENERATE_ENTITIES";
const RECORD_BYTES: usize = 20;

#[test]
fn hello_world_is_the_uniscript_world() {
	is!("\"Hello, \\:world\"", "Hello, 🌍");
}

#[test]
fn the_entity_table_is_the_pinned_uniscript_index() {
	let packages = Path::new("scratch/entity_table");
	std::fs::create_dir_all(packages).unwrap();
	let package = fetch_package_into(packages, "uniscript").unwrap();
	let expected = entity_table(&std::fs::read(package.join("data/entities.idx")).unwrap());
	if std::env::var_os(REGENERATE).is_some() {
		std::fs::write(TABLE, &expected).unwrap();
	}
	assert!(std::fs::read_to_string(TABLE).unwrap() == expected, "{TABLE} differs from the pinned uniscript index: {REGENERATE}=1 tests/queue.sh --test tests -- the_entity_table");
}

/// `name\tcharacters` lines of the index's names table that `\:name` can spell (ASCII letters only), sorted by name
fn entity_table(index: &[u8]) -> String {
	let word = |at: usize| u32::from_le_bytes(index[at..at + 4].try_into().unwrap()) as usize;
	assert_eq!(&index[..4], b"USX1");
	let (records, count) = (word(8), word(12)); // table 0: names
	let text = |at: usize, length: usize| std::str::from_utf8(&index[at..at + length]).unwrap();
	let mut entities: Vec<(&str, &str)> = (0..count)
		.map(|record| records + record * RECORD_BYTES)
		.map(|record| (text(word(record + 4), word(record + 8)), text(word(record + 12), word(record + 16))))
		.filter(|(name, characters)| name.bytes().all(|byte| byte.is_ascii_alphabetic()) && !characters.is_empty() && !characters.contains(['\t', '\n']))
		.collect();
	entities.sort();
	entities.iter().map(|(name, characters)| format!("{name}\t{characters}\n")).collect()
}

// uniscript 1.0.4: LaTeX glyphs win over the HTML entities, and the short math names of warp's former table are back
#[test]
fn latex_names_of_uniscript_1_0_4() {
	is!("\"\\:circ \\:neq\"", "∘ ≠"); // in texts: ∘ and ≠ are no operators of code yet (card emoji-code)
	is!("\"\\:nat \\:to \\:varepsilon\"", "ℕ → ε");
}
