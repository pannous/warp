//! Export pure integer functions and their laws to Lean 4 and ask Lean to prove them.
//! Warp Int promotes to arbitrary precision on overflow, so it is exported as Lean `Int`.
//! Proved therefore covers the same mathematical integer semantics as the runtime.
//! Results are cached per (definitions + law) hash under ~/.cache/warp/lean (shared by all checkouts, never inside one), so a
//! proof runs once.
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

pub(crate) const LEAN_TIMEOUT: Duration = Duration::from_secs(120);
pub const TACTICS: &[&str] =
	&["rfl", "decide", "ac_rfl", "grind", "simp", "exact warp_mul_self_nonneg _", "simp_all [Int.mul_pos]"];
/// The name of the i-th law proved before the one being proved, a lemma of its proof
const LEMMA_PREFIX: &str = "warp_lemma_";
const HEADER: &str = "import Std.Tactic.BVDecide\n\n\
theorem warp_mul_self_nonneg (x : Int) : 0 ≤ x * x := by\n  \
rcases Int.le_total 0 x with hx | hx\n  \
· exact Int.mul_nonneg hx hx\n  \
· exact Int.mul_nonneg_of_nonpos_of_nonpos hx hx\n\n";
const INTEGER: &str = "Int";
const CACHE_DIR: &str = "~/.cache/warp/lean";
const PROVED: &str = "proved";
const ERROR_MARK: &str = ": error";

type Lean = Result<String, String>;

fn not_exportable(node: &Node) -> Lean {
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
		Op::Arrow => "→",
		_ => return None,
	})
}

/// Integer-valued term. Warp comparisons yield 1/0, so propositions are lifted with `if`.
fn term(node: &Node) -> Lean {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => Ok(integer(*n)),
		Node::True => Ok(integer(1)),
		Node::False => Ok(integer(0)),
		Node::Symbol(name) => Ok(name.clone()),
		Node::Key(left, Op::Neg | Op::Sub, right) if left.is_empty_node() => Ok(format!("(-{})", term(right)?)),
		// Warp `%` is Euclidean (0 ≤ r < |b|) like Lean's Int.emod; `rem` is the truncated Int.tmod
		Node::Key(left, Op::Mod, right) => Ok(format!("(Int.emod {} {})", term(left)?, term(right)?)),
		Node::Key(left, Op::Rem, right) => Ok(format!("(Int.tmod {} {})", term(left)?, term(right)?)),
		Node::Key(left, op, right) if arithmetic(*op).is_some() => {
			Ok(format!("({} {} {})", term(left)?, arithmetic(*op).unwrap(), term(right)?))
		}
		Node::Key(base, Op::Pow, exponent) => match exponent.drop_meta() {
			Node::Number(Number::Int(n)) if *n >= 0 => Ok(format!("({} ^ {})", term(base)?, n)),
			_ => not_exportable(node),
		},
		Node::Key(condition, Op::Question, branches) => match branches.drop_meta() {
			Node::Key(yes, Op::Colon, no) => {
				Ok(format!("(if {} then {} else {})", proposition(condition)?, term(yes)?, term(no)?))
			}
			_ => not_exportable(node),
		},
		Node::Key(head, Op::Else, no) => match head.drop_meta() {
			Node::Key(if_part, Op::Then, yes) => match if_part.drop_meta() {
				Node::Key(empty, Op::If, condition) if empty.is_empty_node() => {
					Ok(format!("(if {} then {} else {})", proposition(condition)?, term(yes)?, term(no)?))
				}
				_ => not_exportable(node),
			},
			_ => not_exportable(node),
		},
		Node::Key(..) => Ok(format!("(if {} then {} else {})", proposition(node)?, integer(1), integer(0))),
		Node::List(items, _, _) if items.len() == 1 => term(&items[0]),
		Node::List(items, _, _) if !items.is_empty() => {
			let parts: Result<Vec<String>, String> = items.iter().map(term).collect();
			Ok(format!("({})", parts?.join(" ")))
		}
		_ => not_exportable(node),
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
		Node::Key(left, Op::Not, right) if left.is_empty_node() => Ok(format!("¬({})", proposition(right)?)),
		Node::List(items, _, _) if items.len() == 1 => proposition(&items[0]),
		Node::True => Ok("True".into()),
		Node::False => Ok("False".into()),
		_ => Ok(format!("{} ≠ {}", term(node)?, integer(0))),
	}
}

fn calls_itself(function: &FunctionDefinition) -> bool {
	let mut recursive = false;
	function.body.visit(&mut |node| {
		if let Some((name, _)) = node.call_parts() {
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

/// A recursive function terminates by the sum of its parameters' sizes, which each call must lower (omega shows it
/// from the branch's condition); Lean refuses one that does not, and the law stays Tested
fn definition(function: &FunctionDefinition) -> Lean {
	let header = format!(
		"def {} {} : {} := {}\n",
		function.name,
		binders(&function.parameters)?,
		INTEGER,
		term(&function.body)?
	);
	if !calls_itself(function) {
		return Ok(header);
	}
	let measure: Vec<String> = function.parameters.iter().map(|(name, _)| format!("{name}.toNat")).collect();
	Ok(format!("{header}termination_by {}\ndecreasing_by all_goals omega\n", measure.join(" + ")))
}

fn theorem_name(law: &Law) -> String {
	let name = law.name.as_deref().unwrap_or_else(|| if law.function.is_empty() { "warp" } else { &law.function });
	let lean_name: String = name
		.chars()
		.map(|character| if character.is_ascii_alphanumeric() || character == '_' { character } else { '_' })
		.collect();
	format!("{lean_name}_law")
}

/// Each alternative must close the goal: `simp` alone may merely rewrite it and "succeed".
fn closing_alternatives(tactics: &[&str]) -> String {
	tactics.iter().map(|tactic| format!("({}; done)", tactic)).collect::<Vec<_>>().join(" | ")
}

fn called_names(node: &Node) -> Vec<String> {
	let mut names = vec![];
	node.visit(&mut |part| {
		if let Some((name, _)) = part.call_parts() {
			names.push(name.to_string())
		}
	});
	names
}

/// The functions the law calls, directly or through their bodies, in definition order: unfolding any other one is
/// an unused simp argument, which Lean warns about
fn reached<'a>(functions: &'a [FunctionDefinition], law: &Law) -> Vec<&'a str> {
	let mut pending = called_names(&law.statement);
	let mut seen: Vec<String> = vec![];
	while let Some(name) = pending.pop() {
		if seen.contains(&name) {
			continue;
		}
		if let Some(function) = functions.iter().find(|f| f.name == name) {
			pending.extend(called_names(&function.body));
		}
		seen.push(name);
	}
	functions.iter().map(|f| f.name.as_str()).filter(|name| seen.iter().any(|s| s == name)).collect()
}

/// The induction over each recursive function the law calls with its variables: `fun_induction sum_to n`, the
/// recursive call's value its hypothesis, every case closed knowing the reached functions' equations (grind) or the
/// unfolded ones' (simp_all loops on a recursive equation), and the lemmas
fn induction_alternatives(
	functions: &[FunctionDefinition],
	law: &Law,
	reached: &[&str],
	unfolded: &[&str],
	lemmas: &[String],
) -> Vec<String> {
	let recursive = |name: &str| functions.iter().any(|function| function.name == name && calls_itself(function));
	let is_variable = |argument: &Node| matches!(argument.drop_meta(), Node::Symbol(name) if law.variables.iter().any(|(variable, _)| variable == name));
	let mut inductions: Vec<String> = vec![];
	law.statement.visit(&mut |node| {
		if let Some((name, arguments)) = node.call_parts() {
			if recursive(name) && arguments.iter().all(is_variable) {
				let names: Vec<String> = arguments.iter().map(|argument| argument.name()).collect();
				inductions.push(format!("fun_induction {name} {}", names.join(" ")));
			}
		}
	});
	inductions.dedup();
	let lemmas: Vec<&str> = lemmas.iter().map(String::as_str).collect();
	let (equations, simplifications) =
		([reached, &lemmas].concat().join(", "), [unfolded, &lemmas].concat().join(", "));
	let cases =
		closing_alternatives(&[&format!("grind [{equations}]"), &format!("simp_all [{simplifications}]"), "omega"]);
	let mut alternatives: Vec<String> =
		inductions.iter().map(|induction| format!("({induction} <;> first | {cases})")).collect();
	if !alternatives.is_empty() {
		alternatives.push(format!("(grind [{equations}]; done)"));
	}
	alternatives
}

/// A complete Lean file trying `tactics` after unfolding the definitions the law reaches
pub fn export(functions: &[FunctionDefinition], law: &Law, tactics: &[&str]) -> Lean {
	export_using(functions, law, &[], tactics)
}

/// The laws proved before, stated as lemmas whose proofs Lean checked in their own files (`sorry` here: a warning)
fn lemma_theorems(lemmas: &[Law]) -> Result<(Vec<String>, String), String> {
	let names: Vec<String> = (1..=lemmas.len()).map(|index| format!("{LEMMA_PREFIX}{index}")).collect();
	let statements: Result<Vec<String>, String> = lemmas
		.iter()
		.zip(&names)
		.map(|(lemma, name)| {
			Ok(format!(
				"theorem {name} {} : {} := by sorry\n",
				binders(&lemma.variables)?,
				proposition(&lemma.statement)?
			))
		})
		.collect();
	Ok((names, statements?.join("")))
}

/// A law of a recursive function tries induction first, and unfolds only the other functions (unfolding the recursive
/// one loops); the lemmas close cases too
pub fn export_using(functions: &[FunctionDefinition], law: &Law, lemmas: &[Law], tactics: &[&str]) -> Lean {
	let definitions: Result<Vec<String>, String> = functions.iter().map(definition).collect();
	let (lemma_names, lemma_statements) = lemma_theorems(lemmas)?;
	let names = reached(functions, law);
	let unfolded: Vec<&str> = names
		.iter()
		.copied()
		.filter(|name| functions.iter().any(|function| function.name == *name && !calls_itself(function)))
		.collect();
	let with_lemmas: Vec<String> = match lemma_names.is_empty() {
		true => vec![],
		false => vec![format!("grind [{}]", lemma_names.join(", ")), format!("simp_all [{}]", lemma_names.join(", "))],
	};
	let tactics: Vec<&str> = tactics.iter().copied().chain(with_lemmas.iter().map(String::as_str)).collect();
	let (unfold, closers) = (format!("try simp only [{}]", unfolded.join(", ")), closing_alternatives(&tactics));
	let inductions = induction_alternatives(functions, law, &names, &unfolded, &lemma_names);
	let proof = match inductions.is_empty() {
		true => format!("{unfold}\n  all_goals first | {closers}"),
		false => format!("first\n  | {}\n  | (({unfold}) <;> first | {closers})", inductions.join("\n  | ")),
	};
	Ok(format!(
		"{}{}{}\ntheorem {} {} : {} := by\n  {}\n",
		HEADER,
		definitions?.join(""),
		lemma_statements,
		theorem_name(law),
		binders(&law.variables)?,
		proposition(&law.statement)?,
		proof
	))
}

fn expand_home(path: &str) -> PathBuf {
	PathBuf::from(crate::util::home_expanded(path))
}

pub(crate) fn lean_executable(name: &str) -> PathBuf {
	let elan = expand_home(&format!("~/.elan/bin/{}", name));
	if elan.exists() {
		elan
	} else {
		PathBuf::from(name)
	}
}

pub(crate) fn run_with_timeout(mut command: Command, limit: Duration) -> Result<String, String> {
	let mut child = command
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| format!("lean not runnable: {}", e))?;
	let started = Instant::now();
	while child.try_wait().map_err(|e| e.to_string())?.is_none() {
		if started.elapsed() > limit {
			let _ = child.kill();
			return Err(format!("lean timed out after {limit:?}"));
		}
		std::thread::sleep(Duration::from_millis(50));
	}
	let output = child.wait_with_output().map_err(|e| e.to_string())?;
	let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
	if output.status.success() && !text.lines().any(is_error_line) {
		Ok(text)
	} else {
		Err(text)
	}
}

/// `file:line:col: error: …`; a warning's file path may contain the word "error"
fn is_error_line(line: &str) -> bool {
	line.contains(ERROR_MARK)
}

fn first_error(lean_output: &str) -> String {
	lean_output.lines().find(|line| is_error_line(line)).unwrap_or("lean failed").trim().to_string()
}

fn check_file(file: &Path) -> Result<String, String> {
	let mut command = Command::new(lean_executable("lean"));
	command.arg(file);
	run_with_timeout(command, LEAN_TIMEOUT).map_err(|output| first_error(&output))
}

fn verdict_of(record: &str) -> Verdict {
	if record == PROVED {
		Verdict::Holds
	} else {
		Verdict::Unknown(record.to_string())
	}
}

fn cached(source: &str, attempt: impl FnOnce(&Path) -> Result<String, String>) -> Verdict {
	let mut hasher = DefaultHasher::new();
	source.hash(&mut hasher);
	let cache = expand_home(CACHE_DIR);
	let base = cache.join(format!("law_{:016x}", hasher.finish()));
	let result_file = base.with_extension("result");
	if let Ok(previous) = std::fs::read_to_string(&result_file) {
		return verdict_of(&previous);
	}
	let lean_file = base.with_extension("lean");
	if let Err(e) = std::fs::create_dir_all(&cache).and_then(|_| std::fs::write(&lean_file, source)) {
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
	prove_using(functions, law, &[])
}

/// As prove, with the laws proved before as lemmas
pub fn prove_using(functions: &[FunctionDefinition], law: &Law, lemmas: &[Law]) -> Verdict {
	match export_using(functions, law, lemmas, TACTICS) {
		Ok(source) => cached(&source, check_file),
		Err(why) => Verdict::Unknown(why),
	}
}
