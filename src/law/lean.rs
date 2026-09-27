//! Export pure integer functions and their laws to Lean 4 and ask Lean to prove them.
//! Warp Int promotes to arbitrary precision on overflow, so it is exported as Lean `Int`.
//! Proved therefore covers the same mathematical integer semantics as the runtime.
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
pub const TACTICS: &[&str] = &["rfl", "decide", "ac_rfl", "grind", "simp", "exact warp_mul_self_nonneg _"];
const HEADER: &str = "import Std.Tactic.BVDecide\n\n\
theorem warp_mul_self_nonneg (x : Int) : 0 ≤ x * x := by\n  \
rcases Int.le_total 0 x with hx | hx\n  \
· exact Int.mul_nonneg hx hx\n  \
· exact Int.mul_nonneg_of_nonpos_of_nonpos hx hx\n\n";
const INTEGER: &str = "Int";
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

fn relation(op: Op, left: String, right: String) -> Option<String> {
	Some(match op {
		Op::Eq => format!("{} = {}", left, right),
		Op::Ne => format!("{} ≠ {}", left, right),
		Op::Lt => format!("{} < {}", left, right),
		Op::Le => format!("{} ≤ {}", left, right),
		Op::Gt => format!("{} > {}", left, right),
		Op::Ge => format!("{} ≥ {}", left, right),
		_ => return None,
	})
}

fn integer(value: i64) -> String {
	format!("({} : Int)", value)
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
		Node::Number(Number::Int(n)) => Ok(integer(*n)),
		Node::True => Ok(integer(1)),
		Node::False => Ok(integer(0)),
		Node::Symbol(name) => Ok(name.clone()),
		Node::Key(left, Op::Neg | Op::Sub, right) if is_empty(left) => Ok(format!("(-{})", term(right)?)),
		// Warp `/` yields a Float. `%` remains signed truncating remainder for integer operands.
		Node::Key(left, Op::Mod, right) => Ok(format!("(Int.tmod {} {})", term(left)?, term(right)?)),
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
		Node::Key(..) => Ok(format!("(if {} then {} else {})", proposition(node)?, integer(1), integer(0))),
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
		_ => Ok(format!("{} ≠ {}", term(node)?, integer(0))),
	}
}

fn calls_itself(function: &FunctionDefinition) -> bool {
	let mut recursive = false;
	function.body.visit(&mut |node| {
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
			Kind::Int => Ok(format!("({} : {})", name, INTEGER)),
			other => Err(format!("only Int is exported to Lean, {} is {:?}", name, other)),
		})
		.collect::<Result<Vec<_>, _>>()
		.map(|parts| parts.join(" "))
}

fn definition(function: &FunctionDefinition) -> Lean {
	if calls_itself(function) {
		return Err(format!("recursive {} is not exported to Lean yet", function.name));
	}
	Ok(format!("def {} {} : {} := {}\n", function.name, binders(&function.parameters)?, INTEGER, term(&function.body)?))
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

fn first_error(lean_output: &str) -> String {
	lean_output.lines().find(|l| l.contains("error")).unwrap_or("lean failed").trim().to_string()
}

fn check_file(file: &Path) -> Result<String, String> {
	let mut command = Command::new(lean_executable("lean"));
	command.arg(file);
	run_with_timeout(command).map_err(|output| first_error(&output))
}

fn verdict_of(record: &str) -> Verdict {
	if record == PROVED { Verdict::Holds } else { Verdict::Unknown(record.to_string()) }
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

/// Holds: proved for unbounded Warp Int. Unknown: Lean could not prove or export the law.
pub fn prove(functions: &[FunctionDefinition], law: &Law) -> Verdict {
	match export(functions, law, TACTICS) {
		Ok(source) => cached(&source, check_file),
		Err(why) => Verdict::Unknown(why),
	}
}
