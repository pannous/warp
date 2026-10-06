// An error or warning shows where it is: the CLI prints the source line with `^^` under the word at its position,
// the web demo underlines it in the editor (card g-_ZNg)
use std::collections::HashSet;
use warp::diagnostic::{excerpt, message_position};
use warp::web::evaluate;

const CONSTANT_PROGRAM: &str = "x = 1\npi = 4\n2 * pi";

#[test]
fn an_excerpt_marks_the_word_at_the_position() {
	assert_eq!(excerpt(CONSTANT_PROGRAM, 2, 1), Some("  2 | pi = 4\n    | ^^".to_string()));
	assert_eq!(excerpt("\tx = 1 + y", 1, 10), Some("  1 | \tx = 1 + y\n    | \t        ^".to_string()));
	assert_eq!(excerpt("x", 3, 1), None);
}

#[test]
fn a_message_names_its_position() {
	assert_eq!(message_position("pi is a constant at 2:1; fix: another name"), Some((2, 1)));
	assert_eq!(message_position("say 3 == 3 is ambiguous at 3:5"), Some((3, 5)));
	assert_eq!(message_position("no position here"), None);
}

#[test]
fn the_page_learns_where_the_error_is() {
	let report = evaluate(CONSTANT_PROGRAM, HashSet::new());
	assert_eq!(report["error_at"], serde_json::json!({"line": 2, "column": 1}));
}

#[cfg(feature = "native")]
#[test]
fn the_cli_prints_the_line_of_an_error_and_a_warning() {
	let failed = crate::common::warp_command().args(["eval", CONSTANT_PROGRAM]).output().unwrap();
	assert!(String::from_utf8_lossy(&failed.stderr).contains("  2 | pi = 4\n    | ^^"), "{failed:?}");
	let warned = crate::common::warp_command().args(["--no-ask", "eval", "x=0\nfor i in 1 upto 4 { x += i }\nx"]).output().unwrap();
	assert!(String::from_utf8_lossy(&warned.stderr).contains("  2 | for i in 1 upto 4 { x += i }\n    |            ^^^^"), "{warned:?}");
}
