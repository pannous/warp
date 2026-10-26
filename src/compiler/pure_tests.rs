//! `warp test [folder…]` (card warp-test-runner, decision warp-is-tests, notes/is_tests.md): the test files under the
//! folders (default tests/), `*.pairs.warp` and the `is!` lines of `test_*.warp`, run by the installed binary through
//! `warp pure` (`--self`: `warp self`), no cargo. Each test runs as its own program in a child process,
//! `warp test --case <pure|self> <file> <index>`, so a hang, trap or crash fails only its own test; the children run in
//! parallel, their verdicts print in file order (`--each`: a line for every test, to diff two compilers' runs).
use crate::compiler::standalone::{prepare, run_code, Compiler};
use crate::lowering::test_blocks::{skipped_pairs, test_cases};
use crate::node::{error, Node};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const DEFAULT_FOLDER: &str = "tests";
/// The hidden command a child runs: `warp test --case <pure|self> <file> <index>`
pub const CASE_FLAG: &str = "--case";
/// `warp test --self`: the tests compiled by compiler-self.wasm, as `warp self` runs programs
const SELF_FLAG: &str = "--self";
/// `warp test --each`: a ✓ line for each passed test too
const EACH_FLAG: &str = "--each";
pub const FLAGS: [&str; 2] = [SELF_FLAG, EACH_FLAG];
const PAIRS_SUFFIX: &str = ".pairs.warp";
const TEST_PREFIX: &str = "test_";
const WARP_SUFFIX: &str = ".warp";
/// A test still running after this fails as timed out
const CASE_SECONDS: u64 = 30;
const POLL: Duration = Duration::from_millis(5);
/// Children at once: half the cores, the Mac is shared by several sessions
const MAX_PARALLEL: usize = 8;
/// The child's last stdout line on a wrong value: `got <value>`
const GOT: &str = "got ";
const QUIET_CHILD: [(&str, &str); 2] = [("WARP_NO_WINDOW", "1"), ("WARP_HINTS", "0")];

struct Case {
	file: PathBuf,
	index: usize,
	line: usize,
	code: String,
	expected: String,
}

/// `warp test [--self] [--each] [folder…]`: every test of the folders' test files run, ✗ lines (`--each`: ✓ lines too)
/// and a total printed; false when one failed
pub fn run_folders(words: &[String]) -> bool {
	let by = if words.iter().any(|word| word == SELF_FLAG) { Compiler::SelfCompiled } else { Compiler::RustCompiled };
	let each = words.iter().any(|word| word == EACH_FLAG);
	let folders: Vec<&String> = words.iter().filter(|word| !FLAGS.contains(&word.as_str())).collect();
	let files: Vec<PathBuf> = folders.iter().flat_map(|folder| test_files(Path::new(folder))).collect();
	let mut skipped = 0;
	let mut cases = vec![];
	for file in &files {
		let source = std::fs::read_to_string(file).unwrap_or_default();
		for skip in skipped_pairs(&source) {
			println!("⊘ {}: skipped {skip}", file.display());
			skipped += 1;
		}
		cases.extend(cases_of(file, &source));
	}
	if let Err(failure) = prepare(by) {
		println!("✗ no compiler to run the tests: {}", failure.serialize());
		return false;
	}
	let verdicts = run_cases(&cases, by);
	for (case, verdict) in cases.iter().zip(&verdicts) {
		// {:?}: a code of several lines stays on its line, its newlines escaped
		let test = format!("{}:{} {:?} {}", case.file.display(), case.line, case.code, case.expected);
		match verdict {
			Err(why) => println!("✗ {test}: {why}"),
			Ok(()) if each => println!("✓ {test}"),
			Ok(()) => {}
		}
	}
	let failed = verdicts.iter().filter(|verdict| verdict.is_err()).count();
	let skipped = if skipped > 0 { format!(", {skipped} skipped") } else { String::new() };
	let run = cases.len();
	let total = if failed > 0 { format!("{failed} of {run} failed") } else { format!("✓ {run} tests passed") };
	println!("{total} in {} files{skipped}", files.len());
	failed == 0
}

/// The test files under `folder`, sorted: `*.pairs.warp` and `test_*.warp`
fn test_files(folder: &Path) -> Vec<PathBuf> {
	let Ok(entries) = std::fs::read_dir(folder) else {
		return vec![];
	};
	let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
	paths.sort();
	paths
		.into_iter()
		.flat_map(|path| match path.is_dir() {
			true => test_files(&path),
			false => vec![path].into_iter().filter(|path| is_test_file(path)).collect(),
		})
		.collect()
}

fn is_test_file(path: &Path) -> bool {
	let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
	name.ends_with(PAIRS_SUFFIX) || (name.starts_with(TEST_PREFIX) && name.ends_with(WARP_SUFFIX))
}

fn cases_of(file: &Path, source: &str) -> Vec<Case> {
	let program = crate::compiler::standalone::parsed(source);
	test_cases(&program)
		.into_iter()
		.enumerate()
		.map(|(index, (statement, code, expected))| Case {
			file: file.to_path_buf(),
			index,
			line: crate::errors::diagnostic::position(&statement).map_or(0, |(line, _)| line),
			code,
			expected: expected.serialize(),
		})
		.collect()
}

/// The cases run by children, MAX_PARALLEL at once: their verdicts in the cases' order
fn run_cases(cases: &[Case], by: Compiler) -> Vec<Result<(), String>> {
	let next = AtomicUsize::new(0);
	let verdicts = Mutex::new(vec![Ok(()); cases.len()]);
	let parallel = std::thread::available_parallelism().map_or(1, |cores| cores.get() / 2).clamp(1, MAX_PARALLEL);
	std::thread::scope(|scope| {
		for _ in 0..parallel {
			scope.spawn(|| loop {
				let index = next.fetch_add(1, Ordering::Relaxed);
				let Some(case) = cases.get(index) else { break };
				let verdict = run_case(case, by);
				verdicts.lock().unwrap_or_else(|poisoned| poisoned.into_inner())[index] = verdict;
			});
		}
	});
	verdicts.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One case in its child: Ok when it gave the expected value, else what it gave
fn run_case(case: &Case, by: Compiler) -> Result<(), String> {
	let child = Command::new(std::env::current_exe().map_err(|failure| failure.to_string())?)
		.args(["test", CASE_FLAG, by.command(), &case.file.display().to_string(), &case.index.to_string()])
		.envs(QUIET_CHILD)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|failure| failure.to_string())?;
	let output = finished(child)?;
	if output.status.success() {
		return Ok(());
	}
	let stdout = String::from_utf8_lossy(&output.stdout);
	let last_line = stdout.lines().last().unwrap_or_default();
	match last_line.strip_prefix(GOT) {
		Some(got) => Err(format!("got {got}")),
		None => {
			let stderr = String::from_utf8_lossy(&output.stderr);
			Err(format!("crashed ({}): {}", output.status, stderr.trim().lines().last().unwrap_or_default()))
		}
	}
}

/// The child's output once it exits, or an error after CASE_SECONDS (the child killed)
fn finished(mut child: Child) -> Result<std::process::Output, String> {
	let start = Instant::now();
	// output piped beyond the pipe's buffer would block the child: read while waiting
	let stdout = child.stdout.take().map(read_in_background);
	let stderr = child.stderr.take().map(read_in_background);
	let status = loop {
		if let Some(status) = child.try_wait().map_err(|failure| failure.to_string())? {
			break status;
		}
		if start.elapsed() > Duration::from_secs(CASE_SECONDS) {
			let _ = child.kill();
			let _ = child.wait();
			return Err(format!("timed out after {CASE_SECONDS} s"));
		}
		std::thread::sleep(POLL);
	};
	let collected = |reader: Option<std::thread::JoinHandle<Vec<u8>>>| {
		reader.map(|reader| reader.join().unwrap_or_default()).unwrap_or_default()
	};
	Ok(std::process::Output { status, stdout: collected(stdout), stderr: collected(stderr) })
}

fn read_in_background(mut pipe: impl std::io::Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
	std::thread::spawn(move || {
		let mut bytes = vec![];
		let _ = pipe.read_to_end(&mut bytes);
		bytes
	})
}

/// `warp test --case <pure|self> <file> <index>`, in the child: the case's code run through `warp pure` or `warp self`
/// against its expected value,
/// matched permissively (decision warp-is-tests): `got == expected`, or both empty/falsy. Prints `got <value>` and
/// returns false when it differs
pub fn run_child_case(command: &str, file: &str, index: &str) -> bool {
	let by = if command == Compiler::SelfCompiled.command() { Compiler::SelfCompiled } else { Compiler::RustCompiled };
	let file = Path::new(file).canonicalize().unwrap_or_else(|_| PathBuf::from(file));
	let source = std::fs::read_to_string(&file).unwrap_or_default();
	let cases = test_cases(&crate::compiler::standalone::parsed(&source));
	let Some((_, code, expected)) = index.parse::<usize>().ok().and_then(|index| cases.into_iter().nth(index)) else {
		println!("{GOT}{}", error(&format!("no test {index} in {}", file.display())).serialize());
		return false;
	};
	let folder = file.parent().unwrap_or(Path::new("/"));
	let got = run_code(&code, folder, by);
	let expected = if is_data(&expected) { expected } else { run_code(&expected.serialize(), folder, by) };
	let matches = !matches!(got.drop_meta(), Node::Error(_))
		&& (got == expected || as_is_compares(&got, &expected) || (got.is_falsy() && expected.is_falsy()));
	if !matches {
		println!("{GOT}{}", got.serialize());
	}
	matches
}

/// The two looser matches of the Rust `is!` tests these cases were translated from (scripts/is_tests.py), which
/// compared the result with a Rust literal (src/node/comparison.rs): an int and a float of the same value,
/// `PartialEq<i64>` / `PartialEq<f64>` for Node (`Float(2.0) == 2`), and a one-character text and its char,
/// `PartialEq<char>` for Node (`Text("b") == 'b'`). Only at the top, as `is!` was: lists compare as Node does
fn as_is_compares(got: &Node, expected: &Node) -> bool {
	use crate::extensions::numbers::Number::{Float, Int};
	match (got.drop_meta(), expected.drop_meta()) {
		(Node::Number(Int(int)), Node::Number(Float(float))) | (Node::Number(Float(float)), Node::Number(Int(int))) => {
			*int as f64 == *float
		}
		(Node::Text(text), Node::Char(char)) | (Node::Char(char), Node::Text(text)) => {
			let mut chars = text.chars();
			chars.next() == Some(*char) && chars.next().is_none()
		}
		_ => false,
	}
}

/// An expected value written as data, compared as it is; anything else (`π`, `(-1)`, `1 + pi`) is code to run
fn is_data(expected: &Node) -> bool {
	match expected.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
		Node::List(items, crate::node::Bracket::Square, _) => items.iter().all(is_data),
		_ => false,
	}
}

#[cfg(test)]
mod tests {
	use super::as_is_compares;
	use crate::node::Node;
	use crate::extensions::numbers::Number::{Float, Int};

	#[test]
	fn compares_as_is_did() {
		let text = |t: &str| Node::Text(t.into());
		assert!(as_is_compares(&Node::Number(Float(2.0)), &Node::Number(Int(2))));
		assert!(as_is_compares(&Node::Number(Int(2)), &Node::Number(Float(2.0))));
		assert!(!as_is_compares(&Node::Number(Float(2.5)), &Node::Number(Int(2))));
		assert!(as_is_compares(&text("b"), &Node::Char('b')));
		assert!(as_is_compares(&Node::Char('b'), &text("b")));
		assert!(!as_is_compares(&text("bc"), &Node::Char('b')));
		assert!(!as_is_compares(&text(""), &Node::Char('b')));
		assert!(!as_is_compares(&Node::Number(Int(1)), &Node::True));
	}
}
