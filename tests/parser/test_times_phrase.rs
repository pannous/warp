// The parser keeps `n times …` as written, the phrase [n times operand]; lowering/times_phrases.rs gives it its meaning
// (a loop, a filled list or a repeated text), so the parser twin in lib/compiler/parser.warp stays syntactic
use crate::is;
use warp::WarpParser;

fn parsed(source: &str) -> String {
	WarpParser::parse(source).serialize().trim().to_string()
}

#[test]
fn times_parses_as_the_phrase_written() {
	assert_eq!(parsed("3 times {print 1}"), "3 times {print 1}");
	assert_eq!(parsed("n times [0]"), "n times [0]");
	assert_eq!(parsed("2 times \"ab\""), "2 times \"ab\"");
}

#[test]
fn times_phrases_still_mean_what_they_did() {
	is!("a = []; 3 times { a.push(1) }; a.length", 3);
	is!("s = 0; n = 4; n times: s += 2; s", 8);
	is!("x = 5 times [0]; x.length", 5);
	is!("2 times \"ab\"", "abab");
	is!("k = 0; 2 times { 3 times { k += 1 } }; k", 6);
}
