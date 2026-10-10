//! paint in the playground's host (web/playground/host.js) under node, the same refusals as natively (src/paint.rs):
//! card canvas-zero, a 0×0 image (show() of use draw before canvas()) fails loudly instead of an empty canvas
use std::process::Command;

/// Runs the module in reader.js + host.js with a paint hook that says each painting's size; the outcome as JSON
const HOST_RUN: &str = r#"
const { readFileSync } = await import("node:fs");
globalThis.self = globalThis;
const [playground, module] = process.argv.slice(1);
const source = ["reader.js", "host.js"].map(file => readFileSync(`${playground}/${file}`, "utf8")).join("\n");
const runProgram = new Function(`${source}\nreturn runProgram;`)();
const outcome = await runProgram(readFileSync(module), { paint: (pixels, width, height) => console.log(`painted ${width}×${height}`) });
console.log(outcome.failure ?? "ran");
"#;

fn run_in_host(code: &str, name: &str) -> String {
	let module = warp::pipeline::for_a_page(|| warp::pipeline::compile(code)).expect("compiled");
	let path = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.wasm"));
	std::fs::write(&path, &module.bytes).unwrap();
	let playground = concat!(env!("CARGO_MANIFEST_DIR"), "/web/playground");
	let output = Command::new("node").args(["--input-type=module", "-e", HOST_RUN, playground]).arg(&path).output().expect("node runs");
	assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
	String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn the_playground_paints_a_canvas_of_its_size() {
	assert_eq!(run_in_host("use draw\ncanvas(2, 1)\nshow()", "paint-small"), "painted 2×1\nran");
}

#[test]
fn the_playground_refuses_an_empty_painting() {
	assert_eq!(run_in_host("use draw\nshow()", "paint-empty"), "paint: a 0×0 image is empty (with use draw: canvas(width, height) before show())");
}
