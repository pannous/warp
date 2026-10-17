//! Asking the Lean model: export, `lake build`, the verdicts, outcomes and axioms beside warp's own, and `warp types`
use super::*;

/// The program as the Lean list of W0 items, or why it is outside W0
pub fn export(code: &str) -> Result<String, String> {
	// the items written into lists and fields, marked as warp's pipeline marks them
	let program = crate::lowering::data_items::lower(crate::warp_parser::parse(code));
	let items = Exporter::default().items(&program)?;
	Ok(format!("[{}]", items.join(",\n  ")))
}

/// Runs Lean commands against the built model, from `.lake/<name>.lean` (one file per kind of request: tests run in
/// parallel)
pub(super) fn ask_model(name: &str, requests: &str) -> Result<String, String> {
	let _one_lake_at_a_time = MODEL_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	build_model()?;
	let file = Path::new(MODEL_DIR).join(".lake").join(format!("{name}.lean"));
	std::fs::write(&file, format!("import WarpTypes\nopen Warp\n\n{requests}"))
		.map_err(|e| format!("cannot write {}: {e}", file.display()))?;
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("env").arg("lean").arg(&file);
	run_with_timeout(command, LEAN_TIMEOUT)
}

/// Builds the model (`lake build` fails on any proof error) and returns its verdict for each exported program
pub fn verdicts(exported: &[String]) -> Result<Vec<ModelVerdict>, String> {
	let requests = exported.iter().map(|items| format!("verdict {items}")).collect();
	ask_each("verdicts", requests, "programs", |line| match line.strip_prefix(ACCEPTED_PREFIX) {
		Some(type_name) => Some(ModelVerdict::Accepted(type_name.to_string())),
		None => (line == REJECTED).then_some(ModelVerdict::Rejected),
	})
}

/// One `#eval IO.println (<request>)` per request; the answers are the output lines `answer` reads, one per request
pub(super) fn ask_each<T>(
	name: &str,
	requests: Vec<String>,
	what: &str,
	answer: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, String> {
	let evaluations: String = requests.iter().map(|request| format!("#eval IO.println ({request})\n")).collect();
	let output = ask_model(name, &evaluations)?;
	let answers: Vec<T> = output.lines().filter_map(answer).collect();
	if answers.len() != requests.len() {
		return Err(format!("the model answered {} of {} {what}:\n{output}", answers.len(), requests.len()));
	}
	Ok(answers)
}

/// Where warp's run-time admission of a value to a builtin type (analyzer admits) differs from W0's `Ty.admits` (its
/// `Ty.sub`, plus an exact number a float place converts), for
/// every builtin type word W0 has a type for and every W0 scalar value type: "word ← value type: warp admits / W0 sub"
pub fn admits_disagreements() -> Result<Vec<String>, String> {
	let words = crate::type_tests::builtin_type_words()
		.filter(|word| crate::type_tests::held_kind(word).is_some() && type_of_word(word).is_some());
	let pairs: Vec<(&str, &str, Kind)> =
		words.flat_map(|word| VALUE_KINDS.iter().map(move |(value, kind)| (word, *value, *kind))).collect();
	let requests = pairs
		.iter()
		.map(|(word, value, _)| format!("Ty.admits ({value}) ({})", type_of_word(word).expect("a W0 type word")))
		.collect();
	let answers: Vec<bool> = ask_each("admits", requests, "pairs", |line| line.parse().ok())?;
	Ok(pairs
		.iter()
		.zip(answers)
		.filter_map(|((word, value, kind), sub)| {
			let admits = crate::analyzer::admits(word, *kind);
			(admits != sub).then(|| format!("{word} ← {value}: warp admits {admits} / W0 sub {sub}"))
		})
		.collect())
}

/// What each exported program gives when the model runs it: `rejected`, `error`, a value as warp prints it, or `?`
/// where the model does not keep the value (instances)
pub fn outcomes(exported: &[String]) -> Result<Vec<String>, String> {
	let requests = exported.iter().map(|items| format!("outcome {items}")).collect();
	ask_each("outcomes", requests, "programs", |line| Some(line.to_string()))
}

/// Warp's value of a program, printed as the model's `outcome` prints it (`?` for what the model does not keep)
pub fn warp_value(code: &str) -> String {
	pub(super) fn shown(value: &Node) -> String {
		match value.drop_meta() {
			Node::Number(Number::Int(n)) => n.to_string(),
			Node::Number(exact @ (Number::Quotient(..) | Number::BigQuotient(_))) => exact.to_string(),
			Node::Number(Number::Float(x)) => crate::extensions::numbers::float_text(*x),
			Node::True => "yes".to_string(),
			Node::False => "no".to_string(),
			Node::Text(text) => format!("\"{text}\""),
			Node::Char(c) => format!("\"{c}\""),
			Node::List(items, _, _) => format!("[{}]", items.iter().map(shown).collect::<Vec<_>>().join(" ")),
			Node::Error(_) => "error".to_string(),
			_ => "?".to_string(),
		}
	}
	shown(&crate::pipeline::eval(code))
}

/// The axioms the named theorems rest on, as `#print axioms` lists them (a `sorry` shows as sorryAx)
pub fn axioms(theorems: &[&str]) -> Result<String, String> {
	ask_model("axioms", &theorems.iter().map(|theorem| format!("#print axioms {theorem}\n")).collect::<String>())
}

/// `lake build` of the model: every theorem is checked again
pub fn build_model() -> Result<String, String> {
	let mut command = Command::new(lean_executable("lake"));
	command.current_dir(MODEL_DIR).arg("build");
	if !Path::new(MODEL_DIR).join(MODEL_BUILD_OUTPUT).exists() {
		eprintln!("building the Lean type model once, a few minutes (lake build in {MODEL_DIR})");
	}
	run_with_timeout(command, MODEL_BUILD_TIMEOUT)
}

/// The proof files of the model
pub fn model_sources() -> Vec<PathBuf> {
	let sources = Path::new(MODEL_DIR).join("WarpTypes");
	let mut files: Vec<PathBuf> = std::fs::read_dir(&sources)
		.map(|entries| entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect())
		.unwrap_or_default();
	files.retain(|path| path.extension().is_some_and(|extension| extension == "lean"));
	files.sort();
	files
}

/// Warp's own verdict: does it compile the program? An `Err` that is no Error is the constant answer of a program
/// that needs no module, an acceptance
pub fn warp_verdict(code: &str) -> Result<(), String> {
	match crate::pipeline::compile(code) {
		Err(answer) => match answer.drop_meta() {
			Node::Error(why) => Err(why.serialize()),
			_ => Ok(()),
		},
		Ok(_) => Ok(()),
	}
}

/// `warp types <code>`: the program in W0, the model's verdict and warp's
pub fn report(code: &str) -> String {
	let exported = match export(code) {
		Ok(items) => items,
		Err(why) => return why,
	};
	let model = match verdicts(std::slice::from_ref(&exported)) {
		Ok(verdicts) => format!("{:?}", verdicts[0]),
		Err(why) => format!("model failed: {why}"),
	};
	let warp = match warp_verdict(code) {
		Ok(()) => "Accepted".to_string(),
		Err(why) => format!("Rejected: {why}"),
	};
	let value = match outcomes(std::slice::from_ref(&exported)) {
		Ok(values) => values[0].clone(),
		Err(why) => format!("model failed: {why}"),
	};
	format!("W0: {exported}\nmodel: {model}\nmodel value: {value}\nwarp: {warp}\nwarp value: {}", warp_value(code))
}
