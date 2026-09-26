//! Export pure integer functions and their laws to Lean 4 and ask Lean to prove them.
//! Warp Int is a wrapping i64, so it is exported as `BitVec 64` with signed order and remainder:
//! Proved means proved for Warp's machine semantics, never for unbounded mathematical integers.
//! Results are cached per (definitions + law) hash under target/lean, so a proof runs once.
use super::{FunctionDefinition, Law, Verdict};
use crate::extensions::numbers::Number;
use crate::node::Node;
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const LEAN_TIMEOUT: Duration = Duration::from_secs(120);
/// grind proves ring identities over BitVec; bv_decide goes last so its counterexample is the reported error.
pub const TACTICS: &[&str] = &["rfl", "decide", "ac_rfl", "grind", "simp", "bv_decide"];
const HEADER: &str = "import Std.Tactic.BVDecide\n\n";
const WORD: &str = "BitVec 64";
const WORD_BITS: &str = "#64";
const COUNTEREXAMPLE_MARKER: &str = "found a counterexample";
const VIOLATED_PREFIX: &str = "violated: ";
const CACHE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/lean");
const PROVED: &str = "proved";

type Lean = Result<String, String>;

fn unsupported(node: &Node) -> Lean {
	Err(format!("not exportable to Lean: {}", node.serialize().trim()))
}

fn arithmetic(op: Op) -> Option<&'static str> {
	Some(match op {
		Op::Add => "+",
		Op::Sub => "-",
		Op::Mul => "*",
		_ => return None,
	})
}

/// Signed order on the wrapping word; `>` and `>=` swap operands.
fn relation(op: Op, left: String, right: String) -> Option<String> {
	Some(match op {
		Op::Eq => format!("{} = {}", left, right),
		Op::Ne => format!("{} ≠ {}", left, right),
		Op::Lt => format!("BitVec.slt {} {} = true", left, right),
		Op::Le => format!("BitVec.sle {} {} = true", left, right),
		Op::Gt => format!("BitVec.slt {} {} = true", right, left),
		Op::Ge => format!("BitVec.sle {} {} = true", right, left),
		_ => return None,
	})
}

fn word(value: i64) -> String {
	format!("(BitVec.ofInt 64 ({}))", value)
}

fn connective(op: Op) -> Option<&'static str> {
	Some(match op {
		Op::And => "∧",
		Op::Or => "∨",
		Op::Arrow | Op::FatArrow => "→",
		_ => return None,
	})
}

fn is_empty(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Empty)
}

/// Integer-valued term. Warp comparisons yield 1/0, so propositions are lifted with `if`.
fn term(node: &Node) -> Lean {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => Ok(word(*n)),
		Node::True => Ok(word(1)),
		Node::False => Ok(word(0)),
		Node::Symbol(name) => Ok(name.clone()),
		Node::Key(left, Op::Neg | Op::Sub, right) if is_empty(left) => Ok(format!("(-{})", term(right)?)),
		// Warp `/` yields a Float, so it has no wrapping-word counterpart and stays unsupported
		Node::Key(left, Op::Mod, right) => Ok(format!("(BitVec.srem {} {})", term(left)?, term(right)?)),
		Node::Key(left, op, right) if arithmetic(*op).is_some() => {
			Ok(format!("({} {} {})", term(left)?, arithmetic(*op).unwrap(), term(right)?))
		}
		Node::Key(base, Op::Pow, exponent) => match exponent.drop_meta() {
			Node::Number(Number::Int(n)) if *n >= 0 => Ok(format!("({} ^ {})", term(base)?, n)),
			_ => unsupported(node),
		},
		Node::Key(condition, Op::Question, branches) => match branches.drop_meta() {
			Node::Key(yes, Op::Colon, no) => Ok(format!("(if {} then {} else {})", proposition(condition)?, term(yes)?, term(no)?)),
			_ => unsupported(node),
		},
		Node::Key(head, Op::Else, no) => match head.drop_meta() {
			Node::Key(if_part, Op::Then, yes) => match if_part.drop_meta() {
				Node::Key(empty, Op::If, condition) if is_empty(empty) => {
					Ok(format!("(if {} then {} else {})", proposition(condition)?, term(yes)?, term(no)?))
				}
				_ => unsupported(node),
			},
			_ => unsupported(node),
		},
		Node::Key(..) => Ok(format!("(if {} then {} else {})", proposition(node)?, word(1), word(0))),
		Node::List(items, _, _) if items.len() == 1 => term(&items[0]),
		Node::List(items, _, _) if !items.is_empty() => {
			let parts: Result<Vec<String>, String> = items.iter().map(term).collect();
			Ok(format!("({})", parts?.join(" ")))
		}
		_ => unsupported(node),
	}
}

fn proposition(node: &Node) -> Lean {
	match node.drop_meta() {
		Node::Key(left, op, right) if relation(*op, String::new(), String::new()).is_some() => {
			Ok(relation(*op, term(left)?, term(right)?).unwrap())
		}
		Node::Key(left, op, right) if connective(*op).is_some() => {
			Ok(format!("({}) {} ({})", proposition(left)?, connective(*op).unwrap(), proposition(right)?))
		}
		Node::Key(left, Op::Not, right) if is_empty(left) => Ok(format!("¬({})", proposition(right)?)),
		Node::List(items, _, _) if items.len() == 1 => proposition(&items[0]),
		Node::True => Ok("True".into()),
		Node::False => Ok("False".into()),
		_ => Ok(format!("{} ≠ {}", term(node)?, word(0))),
	}
}

fn calls_itself(function: &FunctionDefinition) -> bool {
	let mut recursive = false;
	super::visit(&function.body, &mut |node| {
		if let Some((name, _)) = super::call_parts(node) {
			recursive |= name == function.name;
		}
	});
	recursive
}

fn binders(variables: &[(String, Kind)]) -> Result<String, String> {
	variables
		.iter()
		.map(|(name, kind)| match kind {
			Kind::Int => Ok(format!("({} : {})", name, WORD)),
			other => Err(format!("only Int is exported to Lean, {} is {:?}", name, other)),
		})
		.collect::<Result<Vec<_>, _>>()
		.map(|parts| parts.join(" "))
}

fn definition(function: &FunctionDefinition) -> Lean {
	if calls_itself(function) {
		return Err(format!("recursive {} is not exported to Lean yet", function.name));
	}
	Ok(format!("def {} {} : {} := {}\n", function.name, binders(&function.parameters)?, WORD, term(&function.body)?))
}

fn theorem_name(law: &Law) -> String {
	format!("{}_law", if law.function.is_empty() { "warp" } else { &law.function })
}

/// Each alternative must close the goal: `simp` alone may merely rewrite it and "succeed".
fn closing_alternatives(tactics: &[&str]) -> String {
	tactics.iter().map(|tactic| format!("({}; done)", tactic)).collect::<Vec<_>>().join(" | ")
}

/// A complete Lean file trying `tactics` after unfolding all definitions.
pub fn export(functions: &[FunctionDefinition], law: &Law, tactics: &[&str]) -> Lean {
	let definitions: Result<Vec<String>, String> = functions.iter().map(definition).collect();
	let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
	Ok(format!(
		"{}{}\ntheorem {} {} : {} := by\n  try simp only [{}]\n  all_goals first | {}\n",
		HEADER,
		definitions?.join(""),
		theorem_name(law),
		binders(&law.variables)?,
		proposition(&law.statement)?,
		names.join(", "),
		closing_alternatives(tactics)
	))
}

fn expand_home(path: &str) -> PathBuf {
	match (path.strip_prefix("~/"), std::env::var("HOME")) {
		(Some(rest), Ok(home)) => Path::new(&home).join(rest),
		_ => PathBuf::from(path),
	}
}

fn lean_executable(name: &str) -> PathBuf {
	let elan = expand_home(&format!("~/.elan/bin/{}", name));
	if elan.exists() { elan } else { PathBuf::from(name) }
}

fn run_with_timeout(mut command: Command) -> Result<String, String> {
	let mut child = command
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| format!("lean not runnable: {}", e))?;
	let started = Instant::now();
	while child.try_wait().map_err(|e| e.to_string())?.is_none() {
		if started.elapsed() > LEAN_TIMEOUT {
			let _ = child.kill();
			return Err(format!("lean timed out after {:?}", LEAN_TIMEOUT));
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	let output = child.wait_with_output().map_err(|e| e.to_string())?;
	let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
	if output.status.success() && !text.contains("error") {
		Ok(text)
	} else {
		Err(text)
	}
}

/// bv_decide prints `x = 13835058055282163505#64`; show that as Warp's signed value.
fn counterexample(lean_output: &str) -> Option<String> {
	let assignment = lean_output.split(COUNTEREXAMPLE_MARKER).nth(1)?;
	let pairs: Vec<String> = assignment
		.lines()
		.filter_map(|line| {
			let (name, value) = line.trim().split_once(" = ")?;
			let unsigned: u64 = value.strip_suffix(WORD_BITS)?.parse().ok()?;
			Some(format!("{}={}", name, unsigned as i64))
		})
		.collect();
	Some(format!("{}lean counterexample {}", VIOLATED_PREFIX, pairs.join(" ")))
}

fn first_error(lean_output: &str) -> String {
	lean_output.lines().find(|l| l.contains("error")).unwrap_or("lean failed").trim().to_string()
}

fn check_file(file: &Path) -> Result<String, String> {
	let mut command = Command::new(lean_executable("lean"));
	command.arg(file);
	run_with_timeout(command).map_err(|output| counterexample(&output).unwrap_or_else(|| first_error(&output)))
}

fn verdict_of(record: &str) -> Verdict {
	match record.strip_prefix(VIOLATED_PREFIX) {
		_ if record == PROVED => Verdict::Holds,
		Some(why) => Verdict::Violated(why.to_string()),
		None => Verdict::Unknown(record.to_string()),
	}
}

fn cached(source: &str, attempt: impl FnOnce(&Path) -> Result<String, String>) -> Verdict {
	let mut hasher = DefaultHasher::new();
	source.hash(&mut hasher);
	let base = Path::new(CACHE_DIR).join(format!("law_{:016x}", hasher.finish()));
	let result_file = base.with_extension("result");
	if let Ok(previous) = std::fs::read_to_string(&result_file) {
		return verdict_of(&previous);
	}
	let lean_file = base.with_extension("lean");
	if let Err(e) = std::fs::create_dir_all(CACHE_DIR).and_then(|_| std::fs::write(&lean_file, source)) {
		return Verdict::Unknown(format!("cannot write {}: {}", lean_file.display(), e));
	}
	let record = match attempt(&lean_file) {
		Ok(_) => PROVED.to_string(),
		Err(why) if why.contains("not runnable") || why.contains("timed out") => return Verdict::Unknown(why),
		Err(why) => why,
	};
	let _ = std::fs::write(result_file, &record);
	verdict_of(&record)
}

/// Holds: proved for wrapping i64. Violated: bv_decide found a counterexample. Unknown: neither.
pub fn prove(functions: &[FunctionDefinition], law: &Law) -> Verdict {
	match export(functions, law, TACTICS) {
		Ok(source) => cached(&source, check_file),
		Err(why) => Verdict::Unknown(why),
	}
}
