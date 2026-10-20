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
	let functions =
		context.user_functions.values().filter(|function| !is_synthesized(&function.name)).map(|function| {
			let params: Vec<String> = function.params.iter().map(|param| param_kind(param).to_string()).collect();
			format!("{}({}) -> {}", function.name, params.join(", "), function.return_kind)
		});
	let variables = main.locals.values().map(|local| format!("{}: {}", local.name, local.kind));
	let mut definitions: Vec<String> = functions.chain(variables).collect();
	definitions.sort();
	definitions.push(format!("= {}", infer_type(program, &main)));
	definitions
}

/// The helpers the analyzer adds for closures (`closure_call_2`, the capture readers), not functions the program defines
fn is_synthesized(name: &str) -> bool {
	crate::closures::closure_call_arity(name).is_some() || crate::closures::capture_reader(name).is_some()
}

/// Each slot's kind, by name: `f(0)` for f's first parameter, `f()` its result, a variable by name, `=` the value
pub type KindSlots = std::collections::BTreeMap<String, String>;

/// The slots of an inferred kinds text (`f(int) -> int`, `x: text`, `= int`)
pub fn kind_slots(lines: &[String]) -> KindSlots {
	let mut slots = KindSlots::new();
	for line in lines {
		if let Some(value) = line.strip_prefix("= ") {
			slots.insert("=".to_string(), value.to_string());
		} else if let Some((signature, result)) = line.split_once(") -> ") {
			let (name, params) = signature.split_once('(').unwrap_or((signature, ""));
			for (index, kind) in params.split(", ").filter(|kind| !kind.is_empty()).enumerate() {
				slots.insert(format!("{name}({index})"), kind.to_string());
			}
			slots.insert(format!("{name}()"), result.to_string());
		} else if let Some((name, kind)) = line.rsplit_once(": ") {
			slots.insert(name.to_string(), kind.to_string());
		}
	}
	slots
}

/// The same slots, each of warp's kinds Rust's or a more general one: a node (empty) holds any value, a number of
/// run-time kind (data) any number; a narrower kind would hold a value Rust's may not fit
pub fn covers(warp: &KindSlots, rust: &KindSlots) -> bool {
	let general = |warp: &str, rust: &str| {
		warp == rust || warp == "empty" || (warp == "data" && (rust == "int" || rust == "float"))
	};
	warp.len() == rust.len()
		&& rust.iter().all(|(slot, kind)| warp.get(slot).is_some_and(|warp_kind| general(warp_kind, kind)))
}
