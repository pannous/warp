// card web-dev-marks: the `warp dev` overlay marks a failure as the playground's editor does (playground.js markWord):
// the word under the excerpt's carets underlined in its source line, the message on hover, the caret line gone
use std::process::Command;

/// dev.js under node (no document, so it polls nothing): the pieces of the failure, each a text or a mark, as JSON
const PIECES: &str = r#"
const { readFileSync } = await import("node:fs");
const [script, failure] = process.argv.slice(1);
const devErrorPieces = new Function(`${readFileSync(script, "utf8")}\nreturn devErrorPieces;`)();
console.log(JSON.stringify(devErrorPieces(failure)));
"#;

fn pieces(failure: &str) -> serde_json::Value {
	let output = Command::new("node")
		.args(["--input-type=module", "-e", PIECES])
		.arg(concat!(env!("CARGO_MANIFEST_DIR"), "/web/playground/dev.js"))
		.arg(failure)
		.output()
		.expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	serde_json::from_slice(&output.stdout).expect("JSON")
}

#[test]
fn the_overlay_marks_the_failing_word() {
	let failure = "undefined: y at 2:14\n  2 | p{ \"three\" + y }\n    |              ^";
	assert_eq!(
		pieces(failure),
		serde_json::json!([
			"undefined: y at 2:14\n  2 | p{ \"three\" + ",
			{ "mark": "y", "message": "undefined: y at 2:14" },
			" }"
		])
	);
}

#[test]
fn a_failure_without_an_excerpt_is_its_text() {
	assert_eq!(pieces("no such file"), serde_json::json!(["no such file"]));
}
