// card code-golf: every solution of samples/golf/ (<hole>.warp readable, <hole>.short.warp golfed) prints its code.golf
// hole's output, <hole>.txt, called with the lines of <hole>.args; a quine prints its own source (notes/code_golf.md)
#![cfg(feature = "native")]
use std::fs;
use std::path::Path;

const GOLF: &str = "samples/golf";
const MINIMUM_SOLUTIONS: usize = 38; // 19 holes, each solved twice

fn trimmed(text: &str) -> String {
	text.trim_end().lines().map(str::trim_end).collect::<Vec<_>>().join("\n")
}

fn printed_by(solution: &Path, arguments: &[String]) -> String {
	let output = crate::common::warp_command().args(["run", "--no-ask"]).arg(solution).args(arguments).output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string()
}

/// None when the solution prints what its hole asks for, else what went wrong
fn failure(solution: &Path) -> Option<String> {
	let name = solution.file_name()?.to_str()?;
	let hole = name.split('.').next()?;
	let expected = fs::read_to_string(Path::new(GOLF).join(format!("{hole}.txt")));
	let arguments: Vec<String> = fs::read_to_string(Path::new(GOLF).join(format!("{hole}.args")))
		.map(|lines| lines.lines().map(str::to_string).collect()).unwrap_or_default();
	let printed = printed_by(solution, &arguments);
	let passes = match &expected {
		Ok(expected) => trimmed(&printed) == trimmed(expected),
		Err(_) => printed == fs::read_to_string(solution).ok()?,
	};
	(!passes).then(|| format!("{name} printed:\n{printed}"))
}

#[test]
fn every_golf_solution_prints_its_holes_output() {
	let mut solutions: Vec<_> = fs::read_dir(GOLF).expect(GOLF).filter_map(|entry| Some(entry.ok()?.path()))
		.filter(|path| path.extension().is_some_and(|extension| extension == "warp")).collect();
	solutions.sort();
	assert!(solutions.len() >= MINIMUM_SOLUTIONS, "only {} solutions in {GOLF}", solutions.len());
	let failures: Vec<String> = solutions.iter().filter_map(|solution| failure(solution)).collect();
	assert!(failures.is_empty(), "{}", failures.join("\n"));
}
