//! Warnings that do not stop compilation: `it` hidden by loops, `x and y or z`, float equality, kebab data keys, conversions of arithmetic, negative modulo
use super::*;

/// Warnings that do not stop compilation: well-defined code that likely does not mean what it says
pub fn lint(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	lint_into(program, &mut warnings);
	warnings.extend(kebab_ambiguities(program));
	program.visit(&mut |node| {
		if let Node::Key(_, Op::Define, body) = node {
			if uses_it_outside_loops(body) {
				hidden_function_it(body, &mut warnings);
			}
		}
	});
	warnings
}

/// The warnings of the program as written, before the passes rewrite `it` (P174 braced_it_body)
pub fn source_warnings(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	program.visit(&mut |node| {
		if let Node::Key(head, Op::Define, body) = node {
			warnings.extend(braced_it_body(head, body));
		}
	});
	warnings
}

/// P174: `mk(k) := { it * k }`, a body that is one braced block reading `it`: it is the parameter k (mk(3) is 9), though
/// Kotlin/Swift readers see a returned lambda; both readings are offered
pub(super) fn braced_it_body(head: &Node, body: &Node) -> Option<Diagnostic> {
	let Node::List(items, _, _) = head.drop_meta() else {
		return None;
	};
	let [name, parameter] = items.as_slice() else {
		return None;
	};
	let (Node::Symbol(name), Node::Symbol(parameter)) = (name.drop_meta(), parameter.drop_meta()) else {
		return None;
	};
	if !matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) || !uses_it_outside_loops(body) {
		return None;
	}
	// a one-statement block reads without its braces: `x => x*k`
	let statement = match body.drop_meta() {
		Node::List(items, _, _) if items.len() == 1 => items[0].clone(),
		other => other.clone(),
	};
	let substituted = |word: &str| {
		crate::library_words::substitute(
			statement.clone(),
			crate::words::IMPLICIT_PARAMETER,
			&Node::Symbol(word.to_string()),
		)
	};
	let written = format!("{name}({parameter}) := {}", body.serialize().trim());
	let as_parameter = format!("{name}({parameter}) := {}", substituted(parameter).serialize().trim());
	let as_lambda = format!("{name}({parameter}) := x => {}", substituted("x").serialize().trim());
	Some(
		Diagnostic::at(
			body,
			format!(
				"`it` in {name}'s braced body is its parameter {parameter}, not the parameter of a returned lambda"
			),
		)
		.fix(format!("{as_parameter} or {as_lambda}"))
		.offer(format!("it is the parameter {parameter}"), &written, as_parameter)
		.offer("a lambda that the function returns", &written, as_lambda),
	)
}

/// `for 1..4 {…}`, `for 1..4: …` and `for i in xs {…}`: loops that bind the implicit `it` (to the item); their iterable
/// and body
pub(in crate::analyzer) fn it_loop(items: &[Node]) -> Option<(&Node, &Node)> {
	let (keyword, iterable, body) = match items {
		[keyword, iterable, body] => (keyword, iterable, body),
		[keyword, _, in_word, iterable, body] if in_word.is_symbol("in") => (keyword, iterable, body),
		_ => return None,
	};
	keyword.is_symbol("for").then_some((iterable, body))
}

/// Does a function body read its implicit parameter `it`, not counting the `it` its loops bind?
pub(in crate::analyzer) fn uses_it_outside_loops(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(word) => word == crate::words::IMPLICIT_PARAMETER,
		Node::Key(left, _, right) => uses_it_outside_loops(left) || uses_it_outside_loops(right),
		Node::List(items, _, _) if let Some((iterable, _)) = it_loop(items) => uses_it_outside_loops(iterable),
		Node::List(items, _, _) => items.iter().any(uses_it_outside_loops),
		_ => false,
	}
}

/// The name the fix of a hidden `it` gives the function's `it`, so the loop can read it
pub(in crate::analyzer) const FUNCTION_IT: &str = "outer_it";

/// The loops in a function with an implicit `it` whose own `it` hides the function's (user decision #23: kept, warned)
pub(in crate::analyzer) fn hidden_function_it(body: &Node, warnings: &mut Vec<Diagnostic>) {
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if it_loop(items).is_some_and(|(_, body)| uses_it(body)) {
				let renamed = crate::library_words::substitute(
					node.clone(),
					crate::words::IMPLICIT_PARAMETER,
					&Node::Symbol(FUNCTION_IT.to_string()),
				);
				let explicit = format!("{FUNCTION_IT}=it; {}", renamed.serialize().trim());
				warnings.push(
					Diagnostic::at(node, "the loop's `it` hides the function's `it` inside the loop")
						.fix(format!(
							"name the function's it before the loop: {FUNCTION_IT}=it; for … {{… {FUNCTION_IT} …}}"
						))
						.offer("the function's `it` inside the loop", node.serialize().trim(), explicit),
				);
			}
		}
	});
}

/// The kinds of lint warnings "got it" silences
pub const AND_OR_TOPIC: &str = "and-or";
pub(super) const ARITHMETIC_CONVERSION_TOPIC: &str = "arithmetic-conversion";
pub(super) const NEGATIVE_MODULO_TOPIC: &str = "negative-modulo";

/// `"done"`, `7`: a value written out that is never falsy
pub(super) fn is_truthy_literal(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True) && !node.is_falsy()
}

/// `x and y or z` yields z when y is falsy; none when y is written out and never falsy: `done and "done" or "open"`
/// can't go wrong (card done-done)
pub(super) fn and_or_warning(node: &Node, condition: &Node, then: &Node, otherwise: &Node) -> Option<Diagnostic> {
	if is_truthy_literal(then) {
		return None;
	}
	let (condition, then, otherwise) = (condition.serialize(), then.serialize(), otherwise.serialize());
	let written = format!("{condition} and {then} or {otherwise}");
	let explicit = format!("if {condition} then {then} else {otherwise}");
	Some(
		Diagnostic::at(node, format!("`{written}` yields {otherwise} whenever {then} is falsy"))
			.fix(&explicit)
			.offer(format!("{then} whenever {condition} holds"), &written, explicit)
			.about(AND_OR_TOPIC, &written),
	)
}

pub(in crate::analyzer) fn lint_into(node: &Node, warnings: &mut Vec<Diagnostic>) {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			if let (Op::Or, Node::Key(condition, Op::And, then)) = (op, left.drop_meta()) {
				warnings.extend(and_or_warning(node, condition, then, right));
			}
			if *op == Op::As && is_ungrouped_arithmetic(left) {
				let diagnostic = Diagnostic::at(node, conversion_of_arithmetic_warning(left, right));
				let written = format!("{} as {}", left.serialize(), right.serialize());
				let readings = conversion_readings(left, right);
				let forms: Vec<&str> = readings.iter().map(|(_, form)| form.as_str()).collect();
				let diagnostic = diagnostic.fix(forms.join(" or "));
				let diagnostic = readings
					.iter()
					.fold(diagnostic, |diagnostic, (meaning, form)| diagnostic.offer(meaning, &written, form));
				warnings.push(diagnostic.about(ARITHMETIC_CONVERSION_TOPIC, &written));
			}
			if *op == Op::Mod && (is_negative(left) || is_negative(right)) {
				let (a, b) = (left.serialize(), right.serialize());
				let written = format!("{} % {}", a.trim(), b.trim());
				warnings.push(
					Diagnostic::at(node, negative_modulo_warning(left, right))
						.offer(
							"the truncated remainder of C/Java/JS",
							&written,
							format!("{} rem {}", a.trim(), b.trim()),
						)
						.about(NEGATIVE_MODULO_TOPIC, &written),
				);
			}
			if matches!(op, Op::Eq | Op::Ne) && (is_float_literal_value(left) || is_float_literal_value(right)) {
				warnings.push(float_equality_warning(node, left, *op, right));
			}
			if let (Op::Add, Some(spelled)) = (op, number_text_plus_number(left, right)) {
				warnings.push(number_text_plus_number_warning(node, left, right, &spelled));
			}
			lint_into(left, warnings);
			lint_into(right, warnings);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| lint_into(item, warnings)),
		_ => {}
	}
}

/// `name:literal` as a data binding: the name and its value
pub(crate) fn data_binding(item: &Node) -> Option<(&str, &Node)> {
	let Node::Key(key, Op::Colon, value) = item.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = key.drop_meta() else {
		return None;
	};
	// a value, or data made of values: `colors:{red:(1 0 0)}` is read by a sibling as `colors.red` (wiki variable.md)
	let is_data = matches!(value.drop_meta(), Node::Number(_) | Node::Text(_) | Node::Char(_))
		|| matches!(value.drop_meta(), Node::List(items, _, _) if !items.is_empty()) && is_data_node(value)
		|| matches!(value.drop_meta(), Node::Key(from, Op::Range | Op::To, to) if is_data_node(from) && is_data_node(to));
	is_data.then(|| (name.as_str(), value.drop_meta()))
}

pub(crate) fn assigned_names(program: &Node) -> HashSet<String> {
	program.assigned_names(|op| matches!(op, Op::Assign | Op::Define))
}

/// Arithmetic on literals that gives a float: a decimal (`0.1 + 0.2`), √2, π (decision exact-default)
pub(super) fn is_float_literal_value(node: &Node) -> bool {
	!matches!(node.drop_meta(), Node::Symbol(_)) && infer_type(node, &Scope::new()) == Kind::Float
}

/// `0.1f + 0.2 == 0.3` is false: decimals are floats, which `==` compares bit for bit (decision exact-default)
pub(super) fn float_equality_warning(node: &Node, left: &Node, op: Op, right: &Node) -> Diagnostic {
	let (a, b) = (left.serialize(), right.serialize());
	let (a, b) = (a.trim(), b.trim());
	let (written, similar) = match op {
		Op::Ne => (format!("{a} != {b}"), format!("not {a} ≈ {b}")),
		_ => (format!("{a} == {b}"), format!("{a} ≈ {b}")),
	};
	Diagnostic::at(
		node,
		format!("`{written}` compares floats exactly: rounding results in classical footgun 0.1f + 0.2 ≠ 0.3"),
	)
	.fix("use ≈ or exact numbers")
	.offer("equal within rounding", written, similar)
}

/// A data key `a-b:2` also reads as the subtraction `a - b` when `a` and `b` are variables
pub(in crate::analyzer) fn kebab_ambiguities(program: &Node) -> Vec<Diagnostic> {
	let variables = assigned_names(program);
	let mut warnings = vec![];
	program.visit(&mut |item| {
		let Some((name, _)) = data_binding(item) else { return };
		let parts: Vec<&str> = name.split('-').collect();
		if parts.len() < 2 || !parts.iter().all(|part| variables.contains(*part)) {
			return;
		}
		let subtraction = parts.join(" - ");
		let message = format!(
			"`{name}` is a data key here, but {} are also variables: `{subtraction}` would subtract",
			parts.join(" and ")
		);
		let warning = Diagnostic::at(item, message).fix(format!("write {subtraction} for the subtraction"));
		warnings.push(kebab_fixes(warning, program, name, &subtraction));
	});
	warnings
}

/// The readings of a kebab data key (user, P81): the data key quoted, when nothing reads it bare; the subtraction at
/// each bare read (the key quoted); the data key renamed with underscores, at the key and at each bare read
pub(in crate::analyzer) fn kebab_fixes(
	warning: Diagnostic,
	program: &Node,
	name: &str,
	subtraction: &str,
) -> Diagnostic {
	let mut reads = vec![];
	bare_reads(program, name, &mut reads);
	let quoted = format!("\"{name}\"");
	let at_reads = |fix: crate::errors::fixits::Fix, replacement: &str| {
		reads.iter().fold(fix, |fix, read| fix.and(name, replacement, *read))
	};
	let renamed = name.replace('-', "_");
	let warning = match reads.is_empty() {
		true => warning.offer(format!("the data key {name}"), name, quoted),
		false => warning.offering(at_reads(
			crate::errors::fixits::fix(format!("the subtraction {subtraction}"), name, quoted),
			subtraction,
		)),
	};
	warning.offering(at_reads(
		crate::errors::fixits::fix(format!("the data key, renamed {renamed}"), name, &renamed),
		&renamed,
	))
}

/// Where `name` is read bare: not as a data key (`a-b:2`) or a member (`x.a-b`), which already mean the key
pub(in crate::analyzer) fn bare_reads(node: &Node, name: &str, reads: &mut Vec<(usize, usize)>) {
	match node {
		Node::Meta { node: inner, .. } if inner.is_symbol(name) => {
			reads.extend(crate::errors::diagnostic::position(node))
		}
		Node::Meta { node, .. } => bare_reads(node, name, reads),
		Node::Key(_, Op::Colon, value) if data_binding(node).is_some() => bare_reads(value, name, reads),
		Node::Key(left, Op::Dot, _) => bare_reads(left, name, reads),
		Node::Key(left, _, right) => {
			bare_reads(left, name, reads);
			bare_reads(right, name, reads);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| bare_reads(item, name, reads)),
		_ => {}
	}
}

/// An ungrouped binary arithmetic expression: `2 * 1.5`, not `(2 * 1.5)`
pub(in crate::analyzer) fn is_ungrouped_arithmetic(node: &Node) -> bool {
	matches!(node, Node::Key(_, op, _) if op.is_arithmetic())
}

pub(in crate::analyzer) fn conversion_of_arithmetic_warning(left: &Node, right: &Node) -> String {
	format!(
		"`{} as {}` converts the whole `{}`, not only its last operand",
		left.serialize(),
		right.serialize(),
		left.serialize()
	)
}

/// Both readings of `a * b as T` with their explicit forms: the whole expression, or the nearest operand
pub(in crate::analyzer) fn conversion_readings(left: &Node, right: &Node) -> Vec<(String, String)> {
	let (whole, target) = (left.serialize(), right.serialize());
	let mut readings = vec![(format!("convert the whole {whole}"), format!("({whole}) as {target}"))];
	if let Node::Key(first, op, last) = left {
		let tight = match last.drop_meta() {
			Node::Number(_) => format!("{}:{target}", last.serialize()),
			_ => format!("({} as {target})", last.serialize()),
		};
		readings.push((
			format!("convert only {}", last.serialize()),
			format!("{} {} {tight}", first.serialize(), op.as_str()),
		));
	}
	readings
}

/// A negative literal or a negation: `-7`, `-x`, `(-7)`
pub(crate) fn is_negative(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n < 0,
		Node::Number(Number::Float(f)) => *f < 0.0,
		Node::Number(Number::Quotient(numerator, _)) => *numerator < 0,
		Node::Number(Number::BigQuotient(q)) => q.is_negative(),
		Node::Number(Number::BigInt(big)) => big.sign() == num_bigint::Sign::Minus,
		Node::Key(left, Op::Neg | Op::Sub, _) => left.is_empty_node(),
		Node::List(items, _, _) if items.len() == 1 => is_negative(&items[0]),
		_ => false,
	}
}

/// `%` is Euclidean (0 ≤ r < |b|); C, Java, JS and Rust truncate, Python floors
pub(in crate::analyzer) fn negative_modulo_warning(left: &Node, right: &Node) -> String {
	let (a, b) = (left.serialize(), right.serialize());
	let (a, b) = (a.trim(), b.trim());
	let values = match (left.drop_meta(), right.drop_meta()) {
		(Node::Number(Number::Int(x)), Node::Number(Number::Int(y))) => x.checked_rem_euclid(*y).zip(x.checked_rem(*y)),
		_ => None,
	};
	match values {
		Some((euclidean, truncated)) if euclidean != truncated => format!(
			"`{a} % {b}` is {euclidean}: % is Euclidean as in mathematics; C/Java/JS give {truncated}; use `rem` for the truncated remainder"
		),
		_ => format!(
			"`{a} % {b}`: % is Euclidean as in mathematics (never negative), unlike C/Java/JS; use `rem` for the truncated remainder"
		),
	}
}

/// `"3"+3`: a text spelling a number added to a number (card print-oldest: "a"+3 joins silently, "3"+3 warns); the
/// spelling text, quoted
pub(crate) fn number_text_plus_number(left: &Node, right: &Node) -> Option<String> {
	let spelled = |text: &Node, number: &Node| match (text.drop_meta(), number.drop_meta()) {
		(Node::Text(digits), Node::Number(_)) if digits.trim().parse::<f64>().is_ok() => {
			Some(text.serialize().trim().to_string())
		}
		(Node::Char(digit), Node::Number(_)) if digit.is_ascii_digit() => Some(format!("\"{digit}\"")),
		_ => None,
	};
	spelled(left, right).or_else(|| spelled(right, left))
}

pub(crate) fn number_text_plus_number_warning(node: &Node, left: &Node, right: &Node, spelled: &str) -> Diagnostic {
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let written_with = |number_form: &dyn Fn(String) -> String, text_form: &dyn Fn(&str) -> String| {
		let operand = |side: &Node| {
			if is_number(side) {
				number_form(side.serialize().trim().to_string())
			} else {
				text_form(spelled)
			}
		};
		format!("{} + {}", operand(left), operand(right))
	};
	let written = written_with(&|number| number, &|text| text.to_string());
	let conversion = if spelled.contains('.') { "float" } else { "int" };
	let as_sum = written_with(&|number| number, &|text| format!("{conversion}({text})"));
	let as_text = written_with(&|number| format!("str({number})"), &|text| text.to_string());
	Diagnostic::at(node, format!("{written} joins the texts; {spelled} spells a number"))
		.fix(format!("{as_sum} or {as_text}"))
		.offer("the sum of the numbers", &written, as_sum)
		.offer("the joined text", &written, as_text)
}
