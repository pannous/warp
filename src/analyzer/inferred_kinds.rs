//! The analyzer's result as text, what its warp twin lib/compiler/analyzer.warp writes (card inferred-kinds,
//! notes/self_hosting.md step 14): the kinds of each function's parameters and result, of each main-level variable and
//! of the program's value

use super::*;

/// `f(int, float) -> float`, `x: text`, `= int` (the value), the definitions sorted by name, the value last
pub fn inferred_kinds(program: &Node) -> Vec<String> {
	let context = function_context(program);
	let function_kinds: HashMap<String, Kind> =
		context.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind)).collect();
	let mut main = Scope::with_function_kinds(function_kinds);
	collect_variables(program, &mut main);
	let functions = context.user_functions.values().map(|function| {
		let params: Vec<String> = function.params.iter().map(|param| param_kind(param).to_string()).collect();
		format!("{}({}) -> {}", function.name, params.join(", "), function.return_kind)
	});
	let variables = main.locals.values().map(|local| format!("{}: {}", local.name, local.kind));
	let mut definitions: Vec<String> = functions.chain(variables).collect();
	definitions.sort();
	definitions.push(format!("= {}", infer_type(program, &main)));
	definitions
}
