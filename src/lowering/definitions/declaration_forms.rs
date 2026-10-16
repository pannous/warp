//! Lowering declarations: typed declarations (`x:T = v`, `fast x=v`, `x:[T]`), parameter copies, object applications

use crate::analyzer::*;
use crate::extensions::numbers::Number;
use crate::filled_lists::*;
use crate::list_mutations::*;
use crate::lowering::nodes::{assign, block, children_rewritten, key, word_headed};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::LIST_WORD;
use std::collections::{HashMap, HashSet};

/// `x:T = v` → `x = v` with T kept as metadata on x (see `declared_type`);
/// the widening of an Int literal assigned to a float becomes an explicit Float literal, a codepoint assigned to a text a Text
pub fn lower_declarations(node: Node) -> Node {
	let variables = assigned_names(&node);
	let mut names = Names { values: names_used_as_values(&node), variables, texts: text_variables(&node) };
	names.values.extend(names.variables.iter().cloned());
	lower_declarations_among(node, &names)
}

/// A function that indexes or counts a list parameter (`def swap(arr, i, j) { … arr[i] … }`) works on a local copy of it,
/// `arr·list = arr`, which the emitter holds as an array (list_dispatch.rs `$NodeList`): one conversion per call
/// instead of a walk per index
pub fn indexed_parameter_copies(node: Node) -> Node {
	let maps = map_parameters(&node);
	let texts = text_parameters(&node);
	parameter_copies(node, &maps, &texts)
}

pub(crate) fn parameter_copies(
	node: Node,
	maps: &HashMap<(String, usize), bool>,
	texts: &HashSet<(String, usize)>,
) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if word_headed(items).is_some()) =>
		{
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			let function = items[0].name();
			let mut body = parameter_copies(*body, maps, texts);
			// a parameter that is assigned (`arr = swap(arr, i, j)`) would convert at every assignment: it keeps walking
			for (index, parameter) in items[1..]
				.iter()
				.enumerate()
				.filter_map(|(index, parameter)| Some((index, parameter_symbol(parameter)?)))
			{
				// every call passing a map: `m[k]` looks a key up, no list copy (a one-entry map would lose its key)
				let takes_maps = maps.get(&(function.clone(), index));
				if takes_maps.is_some_and(|fresh| keys(&body, &parameter, *fresh)) && !assigns(&body, &parameter) {
					body = with_copy(body, &parameter, crate::wasm_emitter::MAP_COPY_SUFFIX); // a hash table (map_backend.rs)
				} else if takes_maps.is_none()
					&& !texts.contains(&(function.clone(), index))
					&& indexes(&body, &parameter)
					&& !assigned_from_call_in_loop(&body, &parameter)
				{
					body = with_copy(body, &parameter, LIST_COPY_SUFFIX);
				}
			}
			Node::Key(head, op, Box::new(body))
		}
		other => children_rewritten(other, |child| parameter_copies(child, maps, texts)),
	}
}

/// The parameters some call passes a text: a text or character literal, a variable only ever given texts, a word giving
/// a text, or a parameter of the calling function that is one of these in turn. They keep the generic way: an array
/// copy would hold the text as its one item, and `source[i]` read the whole text (card self-lexer)
pub(crate) fn text_parameters(program: &Node) -> HashSet<(String, usize)> {
	let mut calls = TextCalls { texts: TextValues::of(program), found: HashSet::new(), passed_on: vec![] };
	calls.visit(program, None);
	let mut changed = true;
	while changed {
		changed = false;
		for (called, from) in &calls.passed_on {
			if calls.found.contains(from) {
				changed |= calls.found.insert(called.clone());
			}
		}
	}
	calls.found
}

struct TextCalls {
	texts: TextValues,
	/// (function, parameter position) some call passes a text
	found: HashSet<(String, usize)>,
	/// (function, parameter position) given a parameter of the calling function: (caller, its parameter position)
	passed_on: Vec<((String, usize), (String, usize))>,
}

impl TextCalls {
	fn visit(&mut self, node: &Node, caller: Option<(&str, &[String])>) {
		match node.drop_meta() {
			Node::Key(head, Op::Define | Op::Assign, body) => match head.drop_meta() {
				Node::List(items, Bracket::Round, _) if word_headed(items).is_some() => {
					let parameters: Vec<String> = items[1..].iter().filter_map(parameter_symbol).collect();
					self.visit(body, Some((&items[0].name(), &parameters)));
				}
				_ => self.visit(body, caller),
			},
			Node::Key(left, _, right) => {
				self.visit(left, caller);
				self.visit(right, caller);
			}
			Node::List(items, _, _) => {
				for (function, arguments) in called_functions(items) {
					for (index, argument) in arguments.into_iter().enumerate() {
						let parameter = caller.and_then(|(caller, parameters)| {
							let position = parameters.iter().position(|name| argument.is_symbol(name))?;
							Some((caller.to_string(), position))
						});
						match parameter {
							Some(from) => self.passed_on.push(((function.clone(), index), from)),
							None if self.texts.given_by(argument) => {
								self.found.insert((function.clone(), index));
							}
							None => {}
						}
					}
				}
				items.iter().for_each(|item| self.visit(item, caller));
			}
			_ => {}
		}
	}
}

/// What gives a text where a function is called: the variables only ever given texts, the fields declared text
struct TextValues {
	holders: HashSet<String>,
	fields: HashSet<String>,
}

impl TextValues {
	fn of(program: &Node) -> Self {
		let no_holders = TextValues { holders: HashSet::new(), fields: text_fields(program) };
		TextValues {
			holders: variables_only_given(program, |value| no_holders.given_by(value)),
			fields: no_holders.fields,
		}
	}

	/// A text or character literal, a variable of `holders`, a word giving a text (`read(path)`), a text joined to it, or
	/// a field declared text (`p.code`, which reads `p#("code"+1)` by now)
	fn given_by(&self, value: &Node) -> bool {
		match value.drop_meta() {
			Node::Text(_) | Node::Char(_) => true,
			Node::Symbol(name) => self.holders.contains(name),
			Node::Key(left, Op::Add, right) => self.given_by(left) || self.given_by(right),
			Node::Key(_, Op::Dot, field) => field.symbol_name().is_some_and(|name| self.fields.contains(name)),
			Node::Key(_, Op::Hash, index) => match index.drop_meta() {
				Node::Key(field, Op::Add, _) => {
					matches!(field.drop_meta(), Node::Text(name) if self.fields.contains(name))
				}
				_ => false,
			},
			Node::List(items, _, _) => match items.split_first() {
				Some((head, arguments)) => head.symbol_name().is_some_and(|word| {
					crate::library_words::result_kind(word) == Some(Kind::Text)
						|| crate::wasm_emitter::text::text_builtins::text_builtin_kind(word, arguments.len())
							== Some(Kind::Text)
				}),
				None => false,
			},
			_ => false,
		}
	}
}

/// The fields some class declares text: `class Scanner { code: text }`
fn text_fields(program: &Node) -> HashSet<String> {
	let mut fields = HashSet::new();
	program.visit(&mut |part| {
		let Node::Type { body, .. } = part else { return };
		let items = match body.drop_meta() {
			Node::List(items, _, _) => items.as_slice(),
			field => std::slice::from_ref(field),
		};
		for item in items {
			if let Node::Key(field, Op::Colon, field_type) = item.drop_meta() {
				let type_name = match field_type.drop_meta() {
					Node::Type { name, .. } => name.symbol_name(),
					other => other.symbol_name(),
				};
				if let (Some(field), Some(type_name)) = (field.symbol_name(), type_name) {
					if crate::analyzer::type_word_kind(type_name) == Some(Kind::Text) {
						fields.insert(field.to_string());
					}
				}
			}
		}
	});
	fields
}

/// The parameters every call passes a map: a map literal of name keys, or a variable only ever assigned one. Only those
/// may become hash tables; a list or an instance given there keeps the generic way. The flag: every call passes a
/// literal, a new map no one else holds
pub(crate) fn map_parameters(program: &Node) -> HashMap<(String, usize), bool> {
	let is_map_literal = |value: &Node| {
		matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.iter().all(|item|
		matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if matches!(key.drop_meta(), Node::Symbol(key) | Node::Text(key) if !key.starts_with(crate::node::ATTRIBUTE_MARK)))))
	};
	let mut assigned: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Node::Symbol(name) = target.drop_meta() {
				*assigned.entry(name.clone()).or_insert(true) &= is_map_literal(value);
			}
		}
	});
	let is_map = |argument: &Node| {
		is_map_literal(argument)
			|| matches!(argument.drop_meta(), Node::Symbol(name) if assigned.get(name) == Some(&true))
	};
	// (every call passes a map, every call passes a literal)
	let mut passed: HashMap<(String, usize), (bool, bool)> = HashMap::new();
	calls_outside_heads(program, &mut |items| {
		for (function, arguments) in called_functions(items) {
			for (index, argument) in arguments.into_iter().enumerate() {
				let (maps, literals) = passed.entry((function.clone(), index)).or_insert((true, true));
				*maps &= is_map(argument);
				*literals &= is_map_literal(argument);
			}
		}
	});
	passed.into_iter().filter(|(_, (maps, _))| *maps).map(|(position, (_, literals))| (position, literals)).collect()
}

/// Every list of the program but a definition's head `(f a b) := …`, which names parameters and calls nothing
pub(crate) fn calls_outside_heads<'a>(node: &'a Node, action: &mut dyn FnMut(&'a [Node])) {
	match node.drop_meta() {
		Node::Key(head, Op::Define | Op::Assign, body)
			if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) =>
		{
			calls_outside_heads(body, action)
		}
		Node::Key(left, _, right) => {
			calls_outside_heads(left, action);
			calls_outside_heads(right, action);
		}
		Node::List(items, _, _) => {
			action(items);
			items.iter().for_each(|item| calls_outside_heads(item, action));
		}
		_ => {}
	}
}

pub(crate) const LIST_COPY_SUFFIX: &str = "·list";

pub(crate) fn parameter_symbol(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.symbol_name(),
		other => other.symbol_name(),
	}
	.map(String::from)
}

/// Does the body index (`xs#i`) or count (`#xs`) the variable in a loop, not by a text key: once is cheaper as a walk
/// than a conversion
pub(crate) fn indexes(body: &Node, name: &str) -> bool {
	let key_variables = key_variables(body);
	let mut found = false;
	let mut loops = vec![];
	body.visit(&mut |part| {
		if let Node::Key(_, Op::While | Op::Do, _) = part {
			loops.push(part)
		}
	});
	loops.into_iter().for_each(|body| {
		body.visit(&mut |part| {
			let Node::Key(list, Op::Hash, index) = part else { return };
			let counted = if matches!(list.drop_meta(), Node::Empty) { index } else { list };
			found |= !looks_up_a_key(index, &key_variables) && counted.is_symbol(name);
		})
	});
	found
}

/// A text key or a variable holding one of the map's keys (`m[k]` in `for k in keys(m)`), not a position
pub fn looks_up_a_key(index: &Node, key_variables: &HashSet<String>) -> bool {
	is_text_key(index)
		|| matches!(crate::warp_parser::subscript_key(index).unwrap_or(index).drop_meta(), Node::Symbol(index) if key_variables.contains(index))
}

/// The variables that hold a map's keys: `k` of a lowered `for k in keys(m)`, `k·items = map_keys(m)` … `k = k·items#i`,
/// and `k = keys(m)#i`
pub fn key_variables(body: &Node) -> HashSet<String> {
	let is_keys_call = |value: &Node| matches!(value.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.name() == crate::words::MAP_KEYS));
	let mut key_lists = HashSet::new();
	body.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			if let (Node::Symbol(list), true) = (target.drop_meta(), is_keys_call(value)) {
				key_lists.insert(list.clone());
			}
		}
	});
	let mut variables = HashSet::new();
	body.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			let reads_a_key = matches!(value.drop_meta(), Node::Key(list, Op::Hash, _)
			if is_keys_call(list) || matches!(list.drop_meta(), Node::Symbol(list) if key_lists.contains(list)));
			if let (Node::Symbol(variable), true) = (target.drop_meta(), reads_a_key) {
				variables.insert(variable.clone());
			}
		}
	});
	variables
}

/// Is the variable assigned a call's result inside a loop (`arr = swap(arr, i, j)`, a conversion per iteration)
pub(crate) fn assigned_from_call_in_loop(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part {
		part.visit(&mut |inner| if let Node::Key(target, Op::Assign, value) = inner {
			let is_call = matches!(value.drop_meta(), Node::List(items, Bracket::Round, Separator::None) if word_headed(items).is_some());
			found |= is_call && target.is_symbol(name);
		});
	});
	found
}

/// Does the body set an entry of the map parameter (`m[k] = v`) or look one up in a loop, by a text key. A map is a
/// reference (P200b): a set entry must reach the caller's map, so a table copy only when every call passes a new one
pub(crate) fn keys(body: &Node, name: &str, fresh: bool) -> bool {
	let is_name = |part: &Node| part.is_symbol(name);
	let key_variables = key_variables(body);
	let by_key = |index: &Node| looks_up_a_key(index, &key_variables);
	let (mut looked_up, mut set) = (false, false);
	body.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, value) = part {
			set |= matches!(target.drop_meta(), Node::Key(map, Op::Hash, _) if is_name(map));
			set |= is_name(target) && crate::library_words::is_field_update_of(name, value);
			looked_up |= matches!(target.drop_meta(), Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index));
			looked_up |= is_name(target) && field_update(value, name);
		}
	});
	body.visit(&mut |part| {
		if let Node::Key(_, Op::While | Op::Do, _) = part {
			part.visit(&mut |inner| {
				looked_up |= matches!(inner, Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index))
			});
		}
	});
	looked_up && (fresh || !set)
}

/// `field_with(m, "k", v)` of the variable m with a text key: the lowered `m["k"] = v`
pub(crate) fn field_update(value: &Node, name: &str) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, map, key, _]
		if word.name() == crate::library_words::FIELD_WITH && map.is_symbol(name) && is_text(key)))
}

/// An index by a text key: `m["k"]`, `m["k\(i)"]` (arriving as `"k" + text_form(i)`), not a position
pub(crate) fn is_text_key(index: &Node) -> bool {
	is_text(crate::warp_parser::subscript_key(index).unwrap_or(index))
}

pub(crate) fn is_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Text(_) | Node::Char(_) => true,
		Node::Key(left, Op::Add, right) => is_text(left) || is_text(right),
		_ => false,
	}
}

/// Is the variable itself assigned in the body (`m = …`, `m += …`), other than by setting an entry
pub(crate) fn assigns(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| {
		if let Node::Key(target, op, value) = part {
			let is_variable = target.is_symbol(name);
			found |= is_variable && (op.is_compound_assign() || (*op == Op::Assign && !field_update(value, name)));
		}
	});
	found
}

/// `name·list = name` (or another copy suffix) first, then the body with every `name` read as the copy
pub(crate) fn with_copy(body: Node, name: &str, suffix: &str) -> Node {
	let copy = format!("{name}{suffix}");
	let renamed = rename_symbol(body, name, &copy);
	let start = assign(Node::Symbol(copy), Node::Symbol(name.to_string()));
	match renamed {
		Node::List(items, Bracket::Curly, separator) => Node::List(
			std::iter::once(start).chain(items).collect(),
			Bracket::Curly,
			match separator {
				Separator::Semicolon | Separator::Newline => separator,
				_ => Separator::Semicolon,
			},
		),
		other => block(vec![start, other]),
	}
}

pub(crate) fn rename_symbol(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		other => children_rewritten(other, |child| rename_symbol(child, from, to)),
	}
}

/// The names of a program: `variables` it assigns (`return count` returns the variable, it is no `return.count`), and
/// every word it uses as a value or binds (a parameter, a loop variable): `chars[i]` then indexes, `int[3]` makes zeros
pub(crate) struct Names {
	variables: HashSet<String>,
	values: HashSet<String>,
	/// Variables only ever given a text: `add "c" to x` appends to the text
	texts: HashSet<String>,
}

/// Variables whose every assigned value is a text literal or an update of the variable itself (`x = x + "c"`)
pub(crate) fn text_variables(program: &Node) -> HashSet<String> {
	variables_only_given(program, |value| matches!(value, Node::Text(_) | Node::Char(_)))
}

/// Variables whose every assigned value is `wanted` or an update of the variable itself (`x = x + "c"`)
fn variables_only_given(program: &Node, wanted: impl Fn(&Node) -> bool) -> HashSet<String> {
	let mut only: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| {
		let Node::Key(target, Op::Assign | Op::Define, value) = part else {
			return;
		};
		let Node::Symbol(name) = target.drop_meta() else { return };
		let updates_itself = matches!(value.drop_meta(), Node::Key(left, _, _) if left.is_symbol(name));
		*only.entry(name.clone()).or_insert(true) &= updates_itself || wanted(value.drop_meta());
	});
	only.into_iter().filter(|(_, given)| *given).map(|(name, _)| name).collect()
}

/// Words that stand as values somewhere: not as the head of a call, a type annotation (`x:int`, `as int`) or the
/// element type of `int[3]`
pub(crate) fn names_used_as_values(program: &Node) -> HashSet<String> {
	fn visit(node: &Node, names: &mut HashSet<String>) {
		match node.drop_meta() {
			Node::Symbol(name) => {
				names.insert(name.clone());
			}
			Node::Key(left, Op::Colon | Op::As, _) => visit(left, names),
			Node::Key(element, Op::Hash, index) if matches!(element.drop_meta(), Node::Symbol(_)) => {
				visit(index, names)
			}
			Node::Key(left, _, right) => {
				visit(left, names);
				visit(right, names);
			}
			Node::List(items, _, _) => {
				let called = word_headed(items).is_some() && items.len() > 1;
				items.iter().skip(called as usize).for_each(|item| visit(item, names))
			}
			_ => {}
		}
	}
	let mut names = HashSet::new();
	visit(program, &mut names);
	names
}

pub(crate) fn lower_declarations_among(node: Node, names: &Names) -> Node {
	let variables = &names.variables;
	let lower = |node| lower_declarations_among(node, names);
	// `data (1..3)` stays the range as written, the Node the run-time `parse("1..3")` gives (card data-quote-lowered)
	if crate::blocks::quoted_data(&node).is_some() {
		return node;
	}
	let node = match crate::for_loop::lower(node) {
		Ok(loop_as_while) => return lower(loop_as_while),
		Err(node) => node,
	};
	match node {
		phrase if let Some(rephrased) = rephrased(&phrase, variables) => lower(rephrased),
		// `x : 100 int` and `x:int[100]` declare x as a zero-filled list of 100 ints
		declaration if let Some((name, zeros)) = typed_array_declaration(&declaration, &names.values) => {
			assign(name, zeros)
		}
		// `letters = char[3]` and `upcases = 26 * char` are zero-filled typed arrays like `x : 100 int`
		Node::Key(target, Op::Assign, value) if let Some(zeros) = typed_array_value(&value, &names.values) => {
			Node::Key(target, Op::Assign, Box::new(zeros))
		}
		Node::Key(target, Op::Assign, value) if let Some(typed) = typed_spelling(&target) => {
			lower(Node::Key(Box::new(typed), Op::Assign, value))
		}
		// `x:[number]` is `x:list of number`
		Node::Key(name, Op::Colon, type_node) if let Some(list_type) = bracketed_list_type(&type_node) => {
			Node::Key(name, Op::Colon, Box::new(list_type))
		}
		range if let Some(list) = range_list(&range, lower) => list,
		// `fast x=v` → `x:fast=v`, parsed either as `(fast x)=v` or as the statement pair `fast (x=v)`;
		// `double(x) := x+x` and `double x := x+x` stay function definitions
		Node::Key(target, Op::Assign, value) if let Some((type_name, name)) = number_type_prefix(&target) => {
			lower(Node::Key(Box::new(key(name, Op::Colon, type_name)), Op::Assign, value))
		}
		Node::List(items, bracket, separator)
			if items.windows(2).any(|pair| paired_declaration(&pair[0], &pair[1]).is_some()) =>
		{
			Node::single_or_list(with_paired_declarations(items, lower), bracket, separator)
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => typed_assignment(target, op, lower(*value)),
		Node::Key(list, Op::Dot, call) if inserted_element(&list, &call).is_some() => lowered_insert(list, &call),
		Node::Key(list, Op::Dot, call) if let Some(element) = appended_element(&list, &call).cloned() => {
			appended(list, lower(element), names)
		}
		Node::Key(list, Op::Dot, call) if let Some(popped) = popped_list(&list, &call) => popped,
		Node::Key(map, Op::Dot, call) if let Some(removal) = removed_key(&map, &call) => lower(removal),
		Node::Key(collection, Op::Dot, call) if let Some(emptied) = cleared_collection(&collection, &call) => emptied,
		// `int[n]`, `#(int[n])`: n zeros of the type, wherever it stands, unless the word is a variable (`chars[i]`)
		Node::Key(element, Op::Hash, one_based)
			if let Some(zeros) = zero_filled_subscript(&element, &one_based, &names.values) =>
		{
			zeros
		}
		// x² and x³ are x^2 and x^3 for emission; the parse keeps the suffix operators
		Node::Key(base, op, _) if let Some(exponent) = op.suffix_exponent() => {
			key(lower(*base), Op::Pow, Node::int(exponent))
		}
		// `const x=v` → `x=v`; check_constants already enforced the single assignment; `let x=v` and `var x=v` → `x=v`
		Node::List(items, bracket, separator) if items.len() >= 2 && is_declaration_keyword(&items[0]) => {
			let declaration = items.into_iter().skip(1).map(lower).collect::<Vec<_>>();
			Node::single_or_list(declaration, bracket, separator)
		}
		Node::List(items, Bracket::None, _) if let Some((object, key)) = applied_object(&items) => {
			lower(crate::warp_parser::subscript(object, key))
		}
		other => children_rewritten(other, lower),
	}
}

/// A range as the list it stands for. `r=1…3` stores the list [1 2 3]; the range itself only lives in a `for` header;
/// `xs = a..b` of computed bounds: the list a loop over the range collects. A range anywhere else a value is wanted
/// (`print 1..5`, `str(a..b)`, `(1..5)`) is the same list: the ranges of `for` headers are loops by now (for_loop)
fn range_list(node: &Node, lower: impl Fn(Node) -> Node) -> Option<Node> {
	match node {
		Node::Key(target, Op::Assign, value) if let Some(elements) = range_elements(value) => {
			Some(Node::Key(target.clone(), Op::Assign, Box::new(elements)))
		}
		Node::Key(target, Op::Assign, value) if let Some(collected) = computed_range(target, value) => {
			Some(lower(Node::Key(target.clone(), Op::Assign, Box::new(collected))))
		}
		range => {
			range_elements(range).or_else(|| computed_range(&Node::Symbol(RANGE_VALUE.to_string()), range).map(lower))
		}
	}
}

/// The phrases that read as another form: `print each …`, counting phrases, `x of type`, postfix list declarations and
/// counts of units
fn rephrased(node: &Node, variables: &HashSet<String>) -> Option<Node> {
	match node {
		Node::List(items, bracket, separator) => print_walk(items, variables)
			.or_else(|| counting_phrase(items, bracket, separator, variables))
			.or_else(|| of_type_declaration(items, bracket, separator))
			.or_else(|| postfix_list_declaration(items, bracket, separator))
			.or_else(|| hashed_unit_count(items)),
		Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) => unit_count(counted),
		_ => None,
	}
}

/// The target `x:number` of another spelling: `x as number = 9` declares `x:number=9`, `x:[number]=v` is
/// `x:list of number=v`
fn typed_spelling(target: &Node) -> Option<Node> {
	match target.drop_meta() {
		Node::Key(name, Op::As, type_node)
			if matches!(name.drop_meta(), Node::Symbol(_)) && is_declaration_type(type_node) =>
		{
			Some(Node::Key(name.clone(), Op::Colon, type_node.clone()))
		}
		Node::Key(name, Op::Colon, type_node) => {
			let list_type = declared_bracketed_list_type(type_node)?;
			Some(Node::Key(name.clone(), Op::Colon, Box::new(list_type)))
		}
		_ => None,
	}
}

/// The items with each pair that declares together (`fast x`, `100 int x`) as its declaration, all lowered
fn with_paired_declarations(items: Vec<Node>, lower: impl Fn(Node) -> Node) -> Vec<Node> {
	let mut lowered = Vec::with_capacity(items.len());
	let mut items = items.into_iter().peekable();
	while let Some(item) = items.next() {
		match items.peek().and_then(|next| paired_declaration(&item, next)) {
			Some(declaration) => {
				items.next();
				lowered.push(lower(declaration));
			}
			None => lowered.push(lower(item)),
		}
	}
	lowered
}

/// `x:float = 1` and `c:text = 'a'` of a built-in type: the value as the type (1.0, "a"), the type kept as the name's
/// meta
fn typed_assignment(target: Box<Node>, op: Op, value: Node) -> Node {
	let value = Box::new(value);
	let Node::Key(name, Op::Colon, type_name) = target.drop_meta() else {
		return Node::Key(target, op, value);
	};
	if !matches!((name.drop_meta(), type_name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) {
		return Node::Key(target, op, value);
	}
	let value = match (builtin_type_kind(&type_name.name()), value.drop_meta()) {
		(Some(Kind::Float), Node::Number(number @ (Number::Int(_) | Number::BigInt(_))))
			if !crate::type_tests::holds_any_number(&type_name.name()) =>
		{
			Box::new(Node::Number(Number::Float((*number).into())))
		}
		(Some(Kind::Text), Node::Char(character)) => Box::new(Node::Text(character.to_string())),
		_ => value,
	};
	Node::Key(Box::new(Node::meta(name.drop_meta().clone(), type_name.drop_meta().clone())), op, value)
}

/// `add "c" to x` of a text: the text grows (wiki row 29); a list gets the element in place, `xs += [v]` like Python's
/// extend, so every holder of the list sees it (P200b)
fn appended(list: Box<Node>, element: Node, names: &Names) -> Node {
	if names.texts.contains(&list.name()) {
		let appended = Node::Key(list.clone(), Op::Add, Box::new(element));
		return Node::Key(list, Op::Assign, Box::new(appended));
	}
	Node::Key(list, Op::AddAssign, Box::new(Node::List(vec![element], Bracket::Square, Separator::Space)))
}

/// A rewritten declaration and the items after it, as they were grouped
fn followed(declared: Node, rest: &[Node], bracket: &Bracket, separator: &Separator) -> Node {
	match rest {
		[] => declared,
		rest => Node::List([vec![declared], rest.to_vec()].concat(), bracket.clone(), separator.clone()),
	}
}

/// `xs:float list = [0.5]` (parsed as the items `xs:float`, `list = [0.5]`) is `xs:"list of float" = [0.5]`
fn postfix_list_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let [declaration, list, rest @ ..] = items else {
		return None;
	};
	let Node::Key(name, Op::Colon, element) = declaration.drop_meta() else {
		return None;
	};
	let element = element.symbol_name()?;
	let typed_name = |name: &Node| key(name.clone(), Op::Colon, Node::Symbol(format!("{LIST_OF_PREFIX}{element}")));
	let declared = match list.drop_meta() {
		Node::Symbol(word) if word == LIST_WORD => typed_name(name),
		Node::Key(word, op @ (Op::Assign | Op::Define), value) if word.is_symbol(LIST_WORD) => {
			Node::Key(Box::new(typed_name(name)), *op, value.clone())
		}
		_ => return None,
	};
	Some(followed(declared, rest, bracket, separator))
}

/// `x:list of int=[1 2]` (parsed as the items `x:list`, `of`, `int=[1 2]`) is `x:"list of int"=[1 2]`, the same type as `x:list<int>`;
/// nested applications chain: `list of list of int`. `x: maybe int = 1` (the items `x:maybe`, `int=1`) is `x:int? = 1`
pub(crate) fn of_type_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	if let Some(declaration) = maybe_type_declaration(items, bracket, separator) {
		return Some(declaration);
	}
	let [declaration, of, ..] = items else { return None };
	let Node::Key(name, Op::Colon, head) = declaration.drop_meta() else {
		return None;
	};
	let Node::Symbol(mut type_name) = head.drop_meta().clone() else {
		return None;
	};
	if !of.is_symbol("of") {
		return None;
	}
	let mut next = 2;
	let mut assignment = None;
	while let Some(item) = items.get(next) {
		next += 1;
		match item.drop_meta() {
			Node::Symbol(word) => type_name = format!("{type_name} of {word}"),
			Node::Key(word, op, value) => {
				let word = word.symbol_name()?;
				type_name = format!("{type_name} of {word}");
				assignment = Some((*op, value.clone()));
				break;
			}
			_ => return None,
		}
		match items.get(next) {
			Some(of) if of.is_symbol("of") => next += 1,
			_ => break,
		}
	}
	let typed_name = Node::Key(name.clone(), Op::Colon, Box::new(Node::Symbol(type_name)));
	let declared = match assignment {
		Some((op, value)) => Node::Key(Box::new(typed_name), op, value),
		None => typed_name,
	};
	Some(followed(declared, &items[next..], bracket, separator))
}

/// `x: maybe int = 1`, `x: maybe int`: the optional type `int?`
fn maybe_type_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let [declaration, typed, rest @ ..] = items else {
		return None;
	};
	let Node::Key(name, Op::Colon, maybe) = declaration.drop_meta() else {
		return None;
	};
	if !maybe.is_symbol(MAYBE_WORD) {
		return None;
	}
	let optional = |word: &Node| match word.drop_meta() {
		Node::Symbol(word) => Some(Node::Key(name.clone(), Op::Colon, Box::new(Node::Symbol(format!("{word}?"))))),
		_ => None,
	};
	let declared = match typed.drop_meta() {
		Node::Key(word, op, value) if matches!(op, Op::Assign | Op::Define) => {
			Node::Key(Box::new(optional(word)?), *op, value.clone())
		}
		word => optional(word)?,
	};
	Some(followed(declared, rest, bracket, separator))
}

/// `{a:1 b:2}(key)`: applying an object to a key looks it up, like `{a:1 b:2}[key]`
pub(crate) fn applied_object(items: &[Node]) -> Option<(Node, Node)> {
	let [object, argument] = items else { return None };
	let is_object = matches!(object.drop_meta(), Node::List(entries, Bracket::Curly, _)
		if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))));
	match argument.drop_meta() {
		Node::List(key, Bracket::Round, _) if is_object && key.len() == 1 => {
			Some((object.drop_meta().clone(), key[0].drop_meta().clone()))
		}
		_ => None,
	}
}

const MAYBE_WORD: &str = "maybe";

/// The name of a constant field key (`p.x`, `p["x"]`) when it is spelled with letters, digits, `_` and `-` (`phone-number`, P164):
/// the runtime error for its miss is a function named after it (`no_field_x`), and the trace of the trap carries the name.
/// A symbol key `p[k]` is evaluated when `k` is a variable, so it is not a constant.
pub fn constant_field_name(key: &Node) -> Option<String> {
	let name = key.text_value()?;
	(!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')).then_some(name)
}

/// The type `[number]`: `list of number` for one element type word or union (`[int or error]`)
pub(crate) fn bracketed_list_type(type_node: &Node) -> Option<Node> {
	let word = bracketed_element(type_node)?;
	type_word_kind(&word).or_else(|| union_parts(&word).map(|_| Kind::Empty))?;
	Some(Node::Symbol(format!("list of {word}")))
}

/// The element type of `[word]` and of `[int or error]`
fn bracketed_element(type_node: &Node) -> Option<String> {
	let Node::List(items, Bracket::Square, _) = type_node.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[element] => element.symbol_name().map(str::to_string).or_else(|| union_type_name(element.drop_meta())),
		_ => None,
	}
}

/// The declared type of `x:[T]=v`: `list of T`, of a builtin type, a class (`xs:[Circle]=[c]`) or a union
fn declared_bracketed_list_type(type_node: &Node) -> Option<Node> {
	Some(Node::Symbol(format!("list of {}", bracketed_element(type_node)?)))
}

/// Two neighbouring statements that together form one declaration
pub(crate) fn paired_declaration(first: &Node, second: &Node) -> Option<Node> {
	prefixed_declaration(first, second).or_else(|| counted_array_declaration(first, second))
}

/// The type a lowered declaration `x:T = v` attached to its target x
pub(crate) fn declared_type(target: &Node) -> Option<&Node> {
	match target {
		Node::Meta { data, .. } if matches!(data.as_ref(), Node::Symbol(_)) => Some(data),
		_ => None,
	}
}

/// A builtin type written before a declaration: `int`, `string`, `float`, `real`, `fast` … (type_tests BUILTIN_TYPES),
/// optional as `int?`
pub(crate) fn is_declaration_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if builtin_type_kind(name.trim_end_matches('?')).is_some() || plural_element_type(name).is_some())
}

/// `(fast x)` as the target of `fast x=v`: the type and the name
pub(crate) fn number_type_prefix(target: &Node) -> Option<(Node, Node)> {
	match target.drop_meta() {
		Node::List(items, bracket, _)
			if *bracket != Bracket::Round
				&& items.len() == 2
				&& is_declaration_type(&items[0])
				&& matches!(items[1].drop_meta(), Node::Symbol(_)) =>
		{
			Some((items[0].drop_meta().clone(), items[1].drop_meta().clone()))
		}
		_ => None,
	}
}

/// The statement pair `fast`, `x=v` as the declaration `x:fast=v`
pub(crate) fn prefixed_declaration(type_name: &Node, next: &Node) -> Option<Node> {
	match next.drop_meta() {
		Node::Key(name, op @ (Op::Assign | Op::Define), value)
			if is_declaration_type(type_name) && matches!(name.drop_meta(), Node::Symbol(_)) =>
		{
			let target = key(name.drop_meta().clone(), Op::Colon, type_name.drop_meta().clone());
			Some(Node::Key(Box::new(target), *op, value.clone()))
		}
		_ => None,
	}
}
