//! The program's uses of a module's exports: calls qualified by the module, its globals read and set, its entry point and classes
use super::*;

/// The import that sets a mutable global g of a module: `set g` (P140), a host function of the value, giving it back
pub(crate) const SETTER_PREFIX: &str = "set ";
/// The exports `include m` runs, the first one m has (P139)
pub(crate) const ENTRY_POINTS: [&str; 2] = ["main", "_start"];

/// The classes of a module warp compiled, as their definitions (its warp.meta `classes`): the program importing it
/// declares them too, so `P(1, 2)` and its methods are the program's own, and an instance crosses as a value
pub fn classes(path: &str) -> Vec<Node> {
	let Some(Node::List(classes, _, _)) = module_bytes(path)
		.ok()
		.and_then(|bytes| crate::formats::meta_section::entry(&bytes, crate::reflection::META_CLASSES))
	else {
		return vec![];
	};
	classes
		.iter()
		.filter_map(|class| match class[crate::reflection::DEFINITION_WORD].drop_meta() {
			Node::Text(definition) => {
				Some(crate::normalize::without_hints(|| crate::warp_parser::parse(definition)).drop_meta().clone())
			}
			_ => None,
		})
		.collect()
}

/// `shapes.P(1, 2)` of a class of the module with that alias (classes): the program's class `P(1, 2)`
pub fn with_bare_classes(node: Node, alias: &str, classes: &[String]) -> Node {
	if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
		let constructed = match member.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().map(|word| word.drop_meta().name()),
			_ => None,
		};
		if receiver.drop_meta().name() == alias && constructed.is_some_and(|class| classes.contains(&class)) {
			return with_bare_classes(member.drop_meta().clone(), alias, classes);
		}
	}
	node.map_children(|child| with_bare_classes(child, alias, classes))
}

/// The name a module's exports are qualified with: its file stem (`fourty_two.twice(21)`)
pub fn module_alias(path: &str) -> String {
	Path::new(path).file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The import of `export` qualified by its module's alias: `fourty_two.twice`, which no builtin of the same name hides
pub fn qualified(path: &str, export: &str) -> String {
	format!("{}.{export}", module_alias(path))
}

/// `include m` (P139): the call of m's entry point (main, _start), whose value the include is
pub fn entry_call(path: &str) -> Option<Node> {
	let exports = exports(path);
	ENTRY_POINTS
		.iter()
		.find(|entry| exports.get(**entry).is_some_and(|export| export.role == Role::Function))
		.map(|entry| call(&qualified(path, entry), vec![]))
}

/// The program's uses of the imported modules' exports: `m.f(x)` is the call of the qualified import, `m.g` and a bare
/// `g` of an exported global read its value (the getter call `g()`), `g = v` sets a mutable global (P140) and is an
/// error for an immutable one (P130); a bare call of an export that a builtin would take (`double(21)`) is ambiguous (P141)
pub fn rewrite_uses(program: Node, modules: &[String]) -> Node {
	if modules.is_empty() {
		return program;
	}
	let globals: HashMap<&String, bool> = modules
		.iter()
		.flat_map(|path| exports(path).iter())
		.filter_map(|(name, export)| match export.role {
			Role::Global { mutable } => Some((name, mutable)),
			_ => None,
		})
		.collect();
	let rewritten = set_globals(program, &globals, modules).and_then(|program| {
		let reads: HashMap<String, Node> =
			globals.keys().map(|global| (global.to_string(), call(global, vec![]))).collect();
		qualify(crate::law::substitute(&program, &reads), modules)
	});
	rewritten.unwrap_or_else(|failure| failure)
}

/// An assignment target naming an exported global, `g` or `m.g`: as written, whether it is mutable, its setter import
pub(crate) fn assigned_global(
	target: &Node,
	globals: &HashMap<&String, bool>,
	modules: &[String],
) -> Option<(String, bool, String)> {
	match target.drop_meta() {
		Node::Symbol(global) => {
			globals.get(global).map(|mutable| (global.clone(), *mutable, format!("{SETTER_PREFIX}{global}")))
		}
		Node::Key(receiver, Op::Dot, member) => {
			let (Node::Symbol(alias), Node::Symbol(global)) = (receiver.drop_meta(), member.drop_meta()) else {
				return None;
			};
			modules.iter().filter(|path| module_alias(path) == *alias).find_map(|path| {
				match exports(path).get(global)?.role {
					Role::Global { mutable } => Some((
						format!("{alias}.{global}"),
						mutable,
						qualified(path, &format!("{SETTER_PREFIX}{global}")),
					)),
					_ => None,
				}
			})
		}
		_ => None,
	}
}

/// `g = v` and `g += v` of an exported global (also qualified, `m.g = v`): the setter call `set g(v)`, `set g(g + v)` of
/// a mutable one; an immutable one is no variable of the program, like `pi = 4` (P130)
pub(crate) fn set_globals(node: Node, globals: &HashMap<&String, bool>, modules: &[String]) -> Result<Node, Node> {
	if let Node::Key(target, op, value) = node.drop_meta() {
		let assigns = *op == Op::Assign || op.is_compound_assign();
		match assigned_global(target, globals, modules) {
			Some((global, false, _)) if assigns => {
				return Err(crate::node::error(&format!(
					"{global} is an immutable global of an imported module; fix: another name"
				)))
			}
			Some((_, true, setter)) if assigns => {
				let value = set_globals(value.as_ref().clone(), globals, modules)?;
				let value = match op.is_compound_assign() {
					true => Node::Key(target.clone(), op.base_op(), Box::new(value)),
					false => value,
				};
				return Ok(call(&setter, vec![value]));
			}
			_ => {}
		}
	}
	map_children_or_failure(node, |child| set_globals(child, globals, modules))
}

/// The node with `map` of each child, or the first failure of a child
pub(crate) fn map_children_or_failure(
	node: Node,
	mut map: impl FnMut(Node) -> Result<Node, Node>,
) -> Result<Node, Node> {
	let mut failure = None;
	let node = node.map_children(|child| {
		map(child).unwrap_or_else(|error| {
			failure.get_or_insert(error);
			Node::Empty
		})
	});
	failure.map_or(Ok(node), Err)
}

/// `m.f` naming an exported function f of the imported module m
pub(crate) fn is_module_function(node: &Node, modules: &[String]) -> bool {
	let Node::Key(receiver, Op::Dot, member) = node.drop_meta() else {
		return false;
	};
	let (Node::Symbol(alias), Node::Symbol(export)) = (receiver.drop_meta(), member.drop_meta()) else {
		return false;
	};
	modules.iter().any(|path| {
		module_alias(path) == *alias && exports(path).get(export).is_some_and(|export| export.role == Role::Function)
	})
}

/// `m.f(x)`, `m.g` of an imported module m: the call of its qualified import; a bare `f(x)` of an export a builtin takes
/// (a cast like `double(21)`) is the ambiguity error naming both (P141)
pub(crate) fn qualify(node: Node, modules: &[String]) -> Result<Node, Node> {
	let module_of = |alias: &str, export: &str| {
		modules.iter().find(|path| module_alias(path) == alias && exports(path).contains_key(export))
	};
	match node.drop_meta() {
		// `m.f.params`: a word of the function itself, not of its result (reflection.rs reads it off warp.meta)
		Node::Key(receiver, Op::Dot, member)
			if matches!(member.drop_meta(), Node::Symbol(_)) && is_module_function(receiver, modules) =>
		{
			return Ok(node)
		}
		Node::Key(receiver, Op::Dot, member) => {
			if let Node::Symbol(alias) = receiver.drop_meta() {
				let (export, arguments) = match member.drop_meta() {
					Node::List(items, _, _) => match items.split_first() {
						Some((Node::Symbol(export), arguments)) => (export.clone(), arguments.to_vec()),
						_ => (String::new(), vec![]),
					},
					// `m.g` read after the global pass: `m.(g)`
					Node::Symbol(export) => (export.clone(), vec![]),
					_ => (String::new(), vec![]),
				};
				if let Some(path) = module_of(alias, &export) {
					return qualified_call(path, &export, arguments, modules);
				}
			}
		}
		Node::List(items, _, _) => {
			if let Some((Node::Symbol(name), arguments)) = items.split_first() {
				let cast = crate::analyzer::type_word_kind(&name.to_lowercase());
				let exporting = modules
					.iter()
					.find(|path| exports(path).get(name).is_some_and(|export| export.role == Role::Function));
				if let (Some(kind), Some(path)) = (cast, exporting) {
					let written: Vec<String> =
						arguments.iter().map(|argument| argument.serialize().trim().to_string()).collect();
					let written = written.join(", ");
					let message = format!(
						"{name} is ambiguous: {}({written}) for the export, {written} as {kind} for the cast",
						qualified(path, name)
					);
					return Err(crate::node::error(&message));
				}
				// `minus(amount: 2, from: 10)`: an export called by its parameter names
				let named = arguments.iter().any(|argument| crate::named_arguments::named_argument(argument).is_some());
				if let (true, Some(path)) = (named, exporting) {
					return qualified_call(path, name, arguments.to_vec(), modules);
				}
			}
		}
		_ => {}
	}
	map_children_or_failure(node, |child| qualify(child, modules))
}

/// The call of `export` of the module at `path`, its arguments qualified and in the export's parameter order
fn qualified_call(path: &str, export: &str, arguments: Vec<Node>, modules: &[String]) -> Result<Node, Node> {
	let arguments = arguments.into_iter().map(|argument| qualify(argument, modules)).collect::<Result<_, _>>()?;
	Ok(call(&qualified(path, export), ordered_arguments(path, export, arguments)?))
}

/// The arguments of a call of the function `export` in its parameter order, a named one (`amount: 2`, `amount=2`) at its
/// parameter of the module's name section; an unknown name or a wrong count is an error showing the parameters
pub(crate) fn ordered_arguments(path: &str, export: &str, arguments: Vec<Node>) -> Result<Vec<Node>, Node> {
	let Some(found @ Export { role: Role::Function, parameters, .. }) = exports(path).get(export) else {
		return Ok(arguments);
	};
	let signature = format!("{}({})", qualified(path, export), parameters.join(", "));
	let wrong_count =
		|| crate::node::error(&format!("{signature} takes {} arguments, got {}", parameters.len(), arguments.len()));
	// the lengths of texts may be left out (CParameter::TextLength): the call passes the texts' own lengths then
	let passed = found.c_parameters.iter().filter(|parameter| parameter.is_passed());
	let lengths: Vec<usize> = passed
		.enumerate()
		.filter(|(_, parameter)| **parameter == CParameter::TextLength)
		.map(|(index, _)| index)
		.collect();
	let left_out = |index: usize| arguments.len() + lengths.len() == parameters.len() && lengths.contains(&index);
	let mut slots: Vec<Option<Node>> = vec![None; parameters.len()];
	for argument in &arguments {
		let (slot, value) = match crate::named_arguments::named_argument(argument) {
			Some((name, value)) => {
				let slot = parameters
					.iter()
					.position(|parameter| *parameter == name)
					.ok_or_else(|| crate::node::error(&format!("{signature} has no parameter {name}")))?;
				(slot, value)
			}
			None => (
				(0..slots.len()).find(|index| slots[*index].is_none() && !left_out(*index)).ok_or_else(wrong_count)?,
				argument,
			),
		};
		slots[slot] = Some(value.clone());
	}
	let given = slots.into_iter().enumerate().filter(|(index, slot)| slot.is_some() || !left_out(*index));
	given.map(|(_, slot)| slot).collect::<Option<Vec<_>>>().ok_or_else(wrong_count)
}
