// Wisp, warp's s-expression form (src/wisp_parser.rs), reads back what it writes (user decision P11, code quality 4:
// "Keep + add a roundtrip test"): warp → Node → emit_wisp → parse_wisp → the same Node
use warp::{emit_wisp, parse, parse_wisp};

const SAMPLES: [&str; 8] = [
	"42",
	"\"hello\"",
	"[1 2 3]",
	"name: \"Alice\"",
	"Person{name:\"Alice\" age:30}",
	"Person { name: \"Alice\"  age: 30  hobbies: [\"reading\" \"hiking\"] }",
	"square := it*it",
	"f(x y)",
];

#[test]
fn wisp_reads_back_what_it_writes() {
	for sample in SAMPLES {
		let node = parse(sample);
		let wisp = emit_wisp(&node);
		assert_eq!(parse_wisp(&wisp).serialize(), node.serialize(), "{sample} → {wisp}");
	}
}
