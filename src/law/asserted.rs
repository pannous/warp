//! Asserted laws: checked on the values the program itself calls its functions with
use super::*;

fn is_closed(node: &Node, functions: &[FunctionDefinition]) -> bool {
	let mut closed = true;
	node.visit(&mut |inner| {
		if let Node::Symbol(name) = inner {
			if !functions.iter().any(|f| &f.name == name) {
				closed = false;
			}
		}
	});
	closed
}

/// Bind law variables by matching law calls `f(x, y)` against concrete program calls `f(3, 4)`.
fn observed_bindings(lawful: &Lawful, law: &Law) -> Vec<HashMap<String, Node>> {
	let mut patterns: Vec<(&str, Vec<String>)> = vec![];
	law.statement.visit(&mut |node| {
		if let Some((name, args)) = node.call_parts() {
			let names: Vec<String> = args
				.iter()
				.filter_map(|a| match a.drop_meta() {
					Node::Symbol(var) if law.variables.iter().any(|(v, _)| v == var) => Some(var.clone()),
					_ => None,
				})
				.collect();
			if !args.is_empty() && names.len() == args.len() {
				patterns.push((name, names));
			}
		}
	});
	let mut bindings = vec![];
	lawful.program.visit(&mut |node| {
		if let Some((name, args)) = node.call_parts() {
			for (pattern, vars) in &patterns {
				let covers_all = law.variables.iter().all(|(v, _)| vars.contains(v));
				if *pattern == name
					&& vars.len() == args.len()
					&& covers_all && args.iter().all(|a| is_closed(a, &lawful.functions))
				{
					bindings.push(vars.iter().cloned().zip(args.iter().cloned()).collect());
				}
			}
		}
	});
	bindings
}

/// Debug builds check every law at the concrete call sites of the program; a violation replaces the result.
pub fn assert_laws(lawful: &Lawful, code: &str) -> Option<Node> {
	if !cfg!(debug_assertions) {
		return None;
	}
	for law in &lawful.laws {
		for bindings in observed_bindings(lawful, law) {
			if let Verdict::Violated(why) = check_instance(lawful, law, &bindings, code) {
				return Some(Node::Error(Box::new(Node::Text(format!("{} violated: {}", law, why)))));
			}
		}
	}
	None
}
