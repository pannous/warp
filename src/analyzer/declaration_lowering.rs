//! Lowering declarations and list forms: typed declarations, parameter copies, list mutation methods, typed arrays

use super::*;

/// `x:T = v` → `x = v` with T kept as metadata on x (see `declared_type`);
/// the widening of an Int literal assigned to a float becomes an explicit Float literal, a codepoint assigned to a text a Text
pub fn lower_declarations(node: Node) -> Node {
	let variables = assigned_names(&node).into_iter().map(str::to_string).collect();
	let mut names = Names { values: names_used_as_values(&node), variables, texts: text_variables(&node) };
	names.values.extend(names.variables.iter().cloned());
	lower_declarations_among(node, &names)
}

/// A function that indexes or counts a list parameter (`def swap(arr, i, j) { … arr[i] … }`) works on a local copy of it,
/// `arr·list = arr`, which the emitter holds as an array (list_dispatch.rs `$NodeList`): one conversion per call
/// instead of a walk per index
pub fn indexed_parameter_copies(node: Node) -> Node {
	let maps = map_parameters(&node);
	parameter_copies(node, &maps)
}

pub(super) fn parameter_copies(node: Node, maps: &HashSet<(String, usize)>) -> Node {
	match node {
		Node::Key(head, op @ (Op::Define | Op::Assign), body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_)))) => {
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			let function = items[0].name();
			let mut body = parameter_copies(*body, maps);
			// a parameter that is assigned (`arr = swap(arr, i, j)`) would convert at every assignment: it keeps walking
			for (index, parameter) in items[1..].iter().enumerate().filter_map(|(index, parameter)| Some((index, parameter_symbol(parameter)?))) {
				// every call passing a map: `m[k]` looks a key up, no list copy (a one-entry map would lose its key)
				let takes_maps = maps.contains(&(function.clone(), index));
				if takes_maps && keys(&body, &parameter) && !assigns(&body, &parameter) {
					body = with_copy(body, &parameter, crate::wasm_emitter::MAP_COPY_SUFFIX); // a hash table (map_backend.rs)
				} else if !takes_maps && indexes(&body, &parameter) && !assigned_from_call_in_loop(&body, &parameter) {
					body = with_copy(body, &parameter, LIST_COPY_SUFFIX);
				}
			}
			Node::Key(head, op, Box::new(body))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(parameter_copies(*left, maps)), op, Box::new(parameter_copies(*right, maps))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| parameter_copies(item, maps)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(parameter_copies(*node, maps)), data },
		other => other,
	}
}

/// The parameters every call passes a map: a map literal of name keys, or a variable only ever assigned one. Only those
/// may become hash tables; a list or an instance given there keeps the generic way
pub(super) fn map_parameters(program: &Node) -> HashSet<(String, usize)> {
	let is_map_literal = |value: &Node| matches!(value.drop_meta(), Node::List(items, Bracket::Curly, _) if items.iter().all(|item|
		matches!(item.drop_meta(), Node::Key(key, Op::Colon, _) if matches!(key.drop_meta(), Node::Symbol(key) | Node::Text(key) if !key.starts_with(crate::node::ATTRIBUTE_MARK)))));
	let mut assigned: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, value) = part {
		if let Node::Symbol(name) = target.drop_meta() {
			*assigned.entry(name.clone()).or_insert(true) &= is_map_literal(value);
		}
	});
	let is_map = |argument: &Node| is_map_literal(argument) || matches!(argument.drop_meta(), Node::Symbol(name) if assigned.get(name) == Some(&true));
	let mut passed: HashMap<(String, usize), bool> = HashMap::new();
	calls_outside_heads(program, &mut |items| for (function, arguments) in called_functions(items) {
		for (index, argument) in arguments.into_iter().enumerate() {
			*passed.entry((function.clone(), index)).or_insert(true) &= is_map(argument);
		}
	});
	passed.into_iter().filter(|(_, maps)| *maps).map(|(position, _)| position).collect()
}

/// Every list of the program but a definition's head `(f a b) := …`, which names parameters and calls nothing
pub(super) fn calls_outside_heads<'a>(node: &'a Node, action: &mut dyn FnMut(&'a [Node])) {
	match node.drop_meta() {
		Node::Key(head, Op::Define | Op::Assign, body) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) => calls_outside_heads(body, action),
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

pub(super) const LIST_COPY_SUFFIX: &str = "·list";

pub(super) fn parameter_symbol(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Colon, _) => match name.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// Does the body index (`xs#i`) or count (`#xs`) the variable in a loop, not by a text key: once is cheaper as a walk
/// than a conversion
pub(super) fn indexes(body: &Node, name: &str) -> bool {
	let key_variables = key_variables(body);
	let mut found = false;
	let mut loops = vec![];
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part { loops.push(part) });
	loops.into_iter().for_each(|body| body.visit(&mut |part| {
		let Node::Key(list, Op::Hash, index) = part else { return };
		let counted = if matches!(list.drop_meta(), Node::Empty) { index } else { list };
		found |= !looks_up_a_key(index, &key_variables) && matches!(counted.drop_meta(), Node::Symbol(symbol) if symbol == name);
	}));
	found
}

/// A text key or a variable holding one of the map's keys (`m[k]` in `for k in keys(m)`), not a position
pub fn looks_up_a_key(index: &Node, key_variables: &HashSet<String>) -> bool {
	is_text_key(index) || matches!(crate::wasp_parser::subscript_key(index).unwrap_or(index).drop_meta(), Node::Symbol(index) if key_variables.contains(index))
}

/// The variables that hold a map's keys: `k` of a lowered `for k in keys(m)`, `k·items = map_keys(m)` … `k = k·items#i`,
/// and `k = keys(m)#i`
pub fn key_variables(body: &Node) -> HashSet<String> {
	let is_keys_call = |value: &Node| matches!(value.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.name() == crate::library_words::MAP_KEYS));
	let mut key_lists = HashSet::new();
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if let (Node::Symbol(list), true) = (target.drop_meta(), is_keys_call(value)) {
			key_lists.insert(list.clone());
		}
	});
	let mut variables = HashSet::new();
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		let reads_a_key = matches!(value.drop_meta(), Node::Key(list, Op::Hash, _)
			if is_keys_call(list) || matches!(list.drop_meta(), Node::Symbol(list) if key_lists.contains(list)));
		if let (Node::Symbol(variable), true) = (target.drop_meta(), reads_a_key) {
			variables.insert(variable.clone());
		}
	});
	variables
}

/// Is the variable assigned a call's result inside a loop (`arr = swap(arr, i, j)`, a conversion per iteration)
pub(super) fn assigned_from_call_in_loop(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part {
		part.visit(&mut |inner| if let Node::Key(target, Op::Assign, value) = inner {
			let is_call = matches!(value.drop_meta(), Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
			found |= is_call && matches!(target.drop_meta(), Node::Symbol(target) if target == name);
		});
	});
	found
}

/// Does the body set an entry of the map variable (`m[k] = v`) or look one up in a loop, by a text key
pub(super) fn keys(body: &Node, name: &str) -> bool {
	let is_name = |part: &Node| matches!(part.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let key_variables = key_variables(body);
	let by_key = |index: &Node| looks_up_a_key(index, &key_variables);
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		found |= matches!(target.drop_meta(), Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index));
		found |= is_name(target) && field_update(value, name);
	});
	body.visit(&mut |part| if let Node::Key(_, Op::While | Op::Do, _) = part {
		part.visit(&mut |inner| found |= matches!(inner, Node::Key(map, Op::Hash, index) if is_name(map) && by_key(index)));
	});
	found
}

/// `field_with(m, "k", v)` of the variable m with a text key: the lowered `m["k"] = v`
pub(super) fn field_update(value: &Node, name: &str) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, map, key, _]
		if word.name() == crate::library_words::FIELD_WITH && matches!(map.drop_meta(), Node::Symbol(map) if map == name) && is_text(key)))
}

/// An index by a text key: `m["k"]`, `m["k\(i)"]` (arriving as `"k" + text_form(i)`), not a position
pub(super) fn is_text_key(index: &Node) -> bool {
	is_text(crate::wasp_parser::subscript_key(index).unwrap_or(index))
}

pub(super) fn is_text(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Text(_) | Node::Char(_) => true,
		Node::Key(left, Op::Add, right) => is_text(left) || is_text(right),
		_ => false,
	}
}

/// Is the variable itself assigned in the body (`m = …`, `m += …`), other than by setting an entry
pub(super) fn assigns(body: &Node, name: &str) -> bool {
	let mut found = false;
	body.visit(&mut |part| if let Node::Key(target, op, value) = part {
		let is_variable = matches!(target.drop_meta(), Node::Symbol(symbol) if symbol == name);
		found |= is_variable && (op.is_compound_assign() || (*op == Op::Assign && !field_update(value, name)));
	});
	found
}

/// `name·list = name` (or another copy suffix) first, then the body with every `name` read as the copy
pub(super) fn with_copy(body: Node, name: &str, suffix: &str) -> Node {
	let copy = format!("{name}{suffix}");
	let renamed = rename_symbol(body, name, &copy);
	let start = Node::Key(Box::new(Node::Symbol(copy)), Op::Assign, Box::new(Node::Symbol(name.to_string())));
	match renamed {
		Node::List(items, Bracket::Curly, separator) => Node::List(std::iter::once(start).chain(items).collect(), Bracket::Curly, match separator {
			Separator::Semicolon | Separator::Newline => separator,
			_ => Separator::Semicolon,
		}),
		other => Node::List(vec![start, other], Bracket::Curly, Separator::Semicolon),
	}
}

pub(super) fn rename_symbol(node: Node, from: &str, to: &str) -> Node {
	match node {
		Node::Symbol(name) if name == from => Node::Symbol(to.to_string()),
		Node::Key(left, op, right) => Node::Key(Box::new(rename_symbol(*left, from, to)), op, Box::new(rename_symbol(*right, from, to))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| rename_symbol(item, from, to)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rename_symbol(*node, from, to)), data },
		other => other,
	}
}

/// The names of a program: `variables` it assigns (`return count` returns the variable, it is no `return.count`), and
/// every word it uses as a value or binds (a parameter, a loop variable): `chars[i]` then indexes, `int[3]` makes zeros
pub(super) struct Names {
	variables: HashSet<String>,
	values: HashSet<String>,
	/// Variables only ever given a text: `add "c" to x` appends to the text
	texts: HashSet<String>,
}

/// Variables whose every assigned value is a text literal or an update of the variable itself (`x = x + "c"`)
pub(super) fn text_variables(program: &Node) -> HashSet<String> {
	let mut texts: HashMap<String, bool> = HashMap::new();
	program.visit(&mut |part| {
		let Node::Key(target, Op::Assign | Op::Define, value) = part else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let is_text = match value.drop_meta() {
			Node::Text(_) | Node::Char(_) => true,
			Node::Key(left, _, _) => matches!(left.drop_meta(), Node::Symbol(updated) if updated == name),
			_ => false,
		};
		*texts.entry(name.clone()).or_insert(true) &= is_text;
	});
	texts.into_iter().filter(|(_, only_texts)| *only_texts).map(|(name, _)| name).collect()
}

/// Words that stand as values somewhere: not as the head of a call, a type annotation (`x:int`, `as int`) or the
/// element type of `int[3]`
pub(super) fn names_used_as_values(program: &Node) -> HashSet<String> {
	fn visit(node: &Node, names: &mut HashSet<String>) {
		match node.drop_meta() {
			Node::Symbol(name) => {
				names.insert(name.clone());
			}
			Node::Key(left, Op::Colon | Op::As, _) => visit(left, names),
			Node::Key(element, Op::Hash, index) if matches!(element.drop_meta(), Node::Symbol(_)) => visit(index, names),
			Node::Key(left, _, right) => {
				visit(left, names);
				visit(right, names);
			}
			Node::List(items, _, _) => {
				let called = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) && items.len() > 1;
				items.iter().skip(called as usize).for_each(|item| visit(item, names))
			}
			_ => {}
		}
	}
	let mut names = HashSet::new();
	visit(program, &mut names);
	names
}

pub(super) fn lower_declarations_among(node: Node, names: &Names) -> Node {
	let variables = &names.variables;
	let lower = |node| lower_declarations_among(node, names);
	let node = match crate::for_loop::lower(node) {
		Ok(loop_as_while) => return lower(loop_as_while),
		Err(node) => node,
	};
	match node {
		Node::List(items, _, _) if print_walk(&items, variables).is_some() => lower(print_walk(&items, variables).expect("guarded")),
		Node::List(items, bracket, separator) if counting_phrase(&items, &bracket, &separator, variables).is_some() => {
			lower(counting_phrase(&items, &bracket, &separator, variables).expect("guarded"))
		}
		Node::List(items, bracket, separator) if of_type_declaration(&items, &bracket, &separator).is_some() => {
			lower(of_type_declaration(&items, &bracket, &separator).expect("guarded"))
		}
		Node::List(items, bracket, separator) if postfix_list_declaration(&items, &bracket, &separator).is_some() => {
			lower(postfix_list_declaration(&items, &bracket, &separator).expect("guarded"))
		}
		Node::List(items, _, _) if hashed_unit_count(&items).is_some() => {
			lower(hashed_unit_count(&items).expect("guarded"))
		}
		Node::Key(empty, Op::Hash, counted) if matches!(empty.drop_meta(), Node::Empty) && unit_count(&counted).is_some() => {
			lower(unit_count(&counted).expect("guarded"))
		}
		// `x : 100 int` and `x:int[100]` declare x as a zero-filled list of 100 ints
		declaration if typed_array_declaration(&declaration).is_some() => {
			let (name, zeros) = typed_array_declaration(&declaration).expect("guarded");
			Node::Key(Box::new(name), Op::Assign, Box::new(zeros))
		}
		// `letters = char[3]` and `upcases = 26 * char` are zero-filled typed arrays like `x : 100 int`
		Node::Key(target, Op::Assign, value) if typed_array_value(&value).is_some() => {
			Node::Key(target, Op::Assign, Box::new(typed_array_value(&value).expect("guarded")))
		}
		// `x as number = 9` declares `x:number=9`
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Key(name, Op::As, type_node)
			if matches!(name.drop_meta(), Node::Symbol(_)) && is_declaration_type(type_node)) => {
			let Node::Key(name, Op::As, type_node) = target.drop_meta().clone() else { unreachable!("guarded") };
			lower(Node::Key(Box::new(Node::Key(name, Op::Colon, type_node)), Op::Assign, value))
		}
		// `x:[number]=v` is `x:list of number=v`
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Key(_, Op::Colon, type_node) if bracketed_list_type(type_node).is_some()) => {
			let Node::Key(name, Op::Colon, type_node) = target.drop_meta().clone() else { unreachable!("guarded") };
			let typed = Node::Key(name, Op::Colon, Box::new(bracketed_list_type(&type_node).expect("guarded")));
			lower(Node::Key(Box::new(typed), Op::Assign, value))
		}
		// `x:[number]` is `x:list of number`
		Node::Key(name, Op::Colon, type_node) if bracketed_list_type(&type_node).is_some() => {
			Node::Key(name, Op::Colon, Box::new(bracketed_list_type(&type_node).expect("guarded")))
		}
		// `r=1…3` stores the list [1 2 3]; the range itself only lives in a `for` header
		Node::Key(target, Op::Assign, value) if range_elements(&value).is_some() => {
			Node::Key(target, Op::Assign, Box::new(range_elements(&value).expect("guarded")))
		}
		// `xs = a..b` of computed bounds: the list a loop over the range collects
		Node::Key(target, Op::Assign, value) if computed_range(&target, &value).is_some() => {
			lower(Node::Key(target.clone(), Op::Assign, Box::new(computed_range(&target, &value).expect("guarded"))))
		}
		// a range anywhere else a value is wanted (`print 1..5`, `str(a..b)`, `(1..5)`) is the same list: the ranges of
		// `for` headers are loops by now (for_loop above)
		range if range_elements(&range).is_some() => range_elements(&range).expect("guarded"),
		range if computed_range(&Node::Symbol(RANGE_VALUE.to_string()), &range).is_some() => {
			lower(computed_range(&Node::Symbol(RANGE_VALUE.to_string()), &range).expect("guarded"))
		}
		// `fast x=v` → `x:fast=v`, parsed either as `(fast x)=v` or as the statement pair `fast (x=v)`;
		// `double(x) := x+x` and `double x := x+x` stay function definitions
		Node::Key(target, Op::Assign, value) if number_type_prefix(&target).is_some() => {
			let (type_name, name) = number_type_prefix(&target).expect("guarded");
			lower(Node::Key(Box::new(Node::Key(Box::new(name), Op::Colon, Box::new(type_name))), Op::Assign, value))
		}
		Node::List(items, bracket, separator) if items.windows(2).any(|pair| paired_declaration(&pair[0], &pair[1]).is_some()) => {
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
			if lowered.len() == 1 {
				lowered.remove(0)
			} else {
				Node::List(lowered, bracket, separator)
			}
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			let value = Box::new(lower(*value));
			match target.drop_meta() {
				Node::Key(name, Op::Colon, type_name) if matches!((name.drop_meta(), type_name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
					let value = match (builtin_type_kind(&type_name.name()), value.drop_meta()) {
						(Some(Kind::Float), Node::Number(number @ (Number::Int(_) | Number::BigInt(_)))) => Box::new(Node::Number(Number::Float((*number).into()))),
						(Some(Kind::Text), Node::Char(character)) => Box::new(Node::Text(character.to_string())),
						_ => value,
					};
					Node::Key(Box::new(Node::meta(name.drop_meta().clone(), type_name.drop_meta().clone())), op, value)
				}
				_ => Node::Key(target, op, value),
			}
		}
		Node::Key(list, Op::Dot, call) if inserted_element(&list, &call).is_some() => lowered_insert(list, &call),
		Node::Key(list, Op::Dot, call) if appended_element(&list, &call).is_some() => {
			let element = lower(appended_element(&list, &call).expect("guarded").clone());
			// `add "c" to x` of a text: the text grows (wiki row 29); of a list: the list gets the element
			let added = match names.texts.contains(&list.name()) {
				true => element,
				false => Node::List(vec![element], Bracket::Square, Separator::Space),
			};
			let appended = Node::Key(list.clone(), Op::Add, Box::new(added));
			Node::Key(list, Op::Assign, Box::new(appended))
		}
		Node::Key(list, Op::Dot, call) if popped_list(&list, &call).is_some() => popped_list(&list, &call).expect("guarded"),
		Node::Key(map, Op::Dot, call) if removed_key(&map, &call).is_some() => lower(removed_key(&map, &call).expect("guarded")),
		// `int[n]`, `#(int[n])`: n zeros of the type, wherever it stands, unless the word is a variable (`chars[i]`)
		Node::Key(element, Op::Hash, one_based) if zero_filled_subscript(&element, &one_based, &names.values).is_some() => {
			zero_filled_subscript(&element, &one_based, &names.values).expect("guarded")
		}
		// x² and x³ are x^2 and x^3 for emission; the parse keeps the suffix operators
		Node::Key(base, op, _) if op.suffix_exponent().is_some() => {
			let exponent = op.suffix_exponent().expect("guarded");
			Node::Key(Box::new(lower(*base)), Op::Pow, Box::new(Node::int(exponent)))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		// `const x=v` → `x=v`; check_constants already enforced the single assignment; `let x=v` and `var x=v` → `x=v`
		Node::List(items, bracket, separator) if items.len() >= 2 && is_declaration_keyword(&items[0]) => {
			let mut declaration = items.into_iter().skip(1).map(lower).collect::<Vec<_>>();
			if declaration.len() == 1 {
				declaration.remove(0)
			} else {
				Node::List(declaration, bracket, separator)
			}
		}
		Node::List(items, Bracket::None, _) if applied_object(&items).is_some() => {
			let (object, key) = applied_object(&items).expect("guarded");
			lower(crate::wasp_parser::subscript(object, key))
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// `xs:float list = [0.5]` (parsed as the items `xs:float`, `list = [0.5]`) is `xs:"list of float" = [0.5]`
fn postfix_list_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let [declaration, list, rest @ ..] = items else { return None };
	let Node::Key(name, Op::Colon, element) = declaration.drop_meta() else { return None };
	let Node::Symbol(element) = element.drop_meta() else { return None };
	let typed_name = |name: &Node| Node::Key(Box::new(name.clone()), Op::Colon, Box::new(Node::Symbol(format!("{LIST_OF_PREFIX}{element}"))));
	let declared = match list.drop_meta() {
		Node::Symbol(word) if word == LIST_WORD => typed_name(name),
		Node::Key(word, op @ (Op::Assign | Op::Define), value) if is_word(word, LIST_WORD) => Node::Key(Box::new(typed_name(name)), *op, value.clone()),
		_ => return None,
	};
	Some(match rest {
		[] => declared,
		rest => Node::List([vec![declared], rest.to_vec()].concat(), bracket.clone(), separator.clone()),
	})
}

/// `x:list of int=[1 2]` (parsed as the items `x:list`, `of`, `int=[1 2]`) is `x:"list of int"=[1 2]`, the same type as `x:list<int>`;
/// nested applications chain: `list of list of int`
pub(super) fn of_type_declaration(items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
	let [declaration, of, ..] = items else { return None };
	let Node::Key(name, Op::Colon, head) = declaration.drop_meta() else { return None };
	let Node::Symbol(mut type_name) = head.drop_meta().clone() else { return None };
	if !is_word(of, "of") {
		return None;
	}
	let mut next = 2;
	let mut assignment = None;
	while let Some(item) = items.get(next) {
		next += 1;
		match item.drop_meta() {
			Node::Symbol(word) => type_name = format!("{type_name} of {word}"),
			Node::Key(word, op, value) => {
				let Node::Symbol(word) = word.drop_meta() else { return None };
				type_name = format!("{type_name} of {word}");
				assignment = Some((*op, value.clone()));
				break;
			}
			_ => return None,
		}
		match items.get(next) {
			Some(of) if is_word(of, "of") => next += 1,
			_ => break,
		}
	}
	let typed_name = Node::Key(name.clone(), Op::Colon, Box::new(Node::Symbol(type_name)));
	let declared = match assignment {
		Some((op, value)) => Node::Key(Box::new(typed_name), op, value),
		None => typed_name,
	};
	match &items[next..] {
		[] => Some(declared),
		rest => Some(Node::List([vec![declared], rest.to_vec()].concat(), bracket.clone(), separator.clone())),
	}
}

/// `{a:1 b:2}(key)`: applying an object to a key looks it up, like `{a:1 b:2}[key]`
pub(super) fn applied_object(items: &[Node]) -> Option<(Node, Node)> {
	let [object, argument] = items else { return None };
	let is_object = matches!(object.drop_meta(), Node::List(entries, Bracket::Curly, _)
		if !entries.is_empty() && entries.iter().all(|entry| matches!(entry.drop_meta(), Node::Key(_, Op::Colon, _))));
	match argument.drop_meta() {
		Node::List(key, Bracket::Round, _) if is_object && key.len() == 1 => Some((object.drop_meta().clone(), key[0].drop_meta().clone())),
		_ => None,
	}
}

/// Methods that append one element; with value semantics `x.add(v)` rebinds `x = x + [v]`
pub(super) const APPEND_METHODS: [&str; 4] = ["add", "append", "push", "insert"];
pub(super) const POP_METHOD: &str = "pop";
/// The list a pop template takes from, replaced by the variable or field popped
const POP_PLACE: &str = "pop_place";
pub(super) const REMOVE_METHOD: &str = "remove";
/// Pseudo-call `removed_value(collection, k)`: what `collection.remove(k)` gives, by the collection's kind: of a map the
/// value of the key (P35, Python's dict.pop), of a list the list without the first element equal to k
pub const REMOVED_VALUE_CALL: &str = "removed_value";
pub(super) const POP_TEMPORARY: &str = "pop_tmp";
pub(super) const INSERT_METHOD: &str = "insert";

/// Pseudo-call `list_insert_at(list, position, value)`: the list with value inserted at the 0-based position
pub const INSERT_AT_CALL: &str = "list_insert_at";
/// Pseudo-call `insert_in_either_order(list, a, b)`: `xs.insert(a, b)` before the kinds decide which is the position
pub const INSERT_EITHER_CALL: &str = "insert_in_either_order";
pub(super) const AT_WORD: &str = "at";

/// The name of a constant field key (`p.x`, `p["x"]`) when it is spelled with letters, digits, `_` and `-` (`phone-number`, P164):
/// the runtime error for its miss is a function named after it (`no_field_x`), and the trace of the trap carries the name.
/// A symbol key `p[k]` is evaluated when `k` is a variable, so it is not a constant.
pub fn constant_field_name(key: &Node) -> Option<String> {
	let name = match key.drop_meta() {
		Node::Text(name) => name.clone(),
		Node::Char(letter) => letter.to_string(),
		_ => return None,
	};
	(!name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')).then_some(name)
}

/// Methods that change the list variable they are called on: `xs.add(v)`, `xs.insert(v, at:1)`, `xs.pop()`
pub fn is_list_mutating_method(name: &str) -> bool {
	APPEND_METHODS.contains(&name) || name == POP_METHOD || name == REMOVE_METHOD
}

/// A place a list update can assign: a variable `xs` or a field of one `p.items`, `self.stack.items` (by now the
/// field index `p#…` of a declared type)
fn is_place(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(object, Op::Dot, field) => matches!(field.drop_meta(), Node::Symbol(_)) && is_place(object),
		Node::Key(object, Op::Hash, _) => is_place(object),
		_ => false,
	}
}

/// The element of `x.add(v)` when x is a variable or a field of one
pub(super) fn appended_element<'a>(list: &Node, call: &'a Node) -> Option<&'a Node> {
	if !is_place(list) {
		return None;
	}
	match call.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(method) if APPEND_METHODS.contains(&method.as_str())) => Some(&items[1]),
		_ => None,
	}
}

/// `xs.pop()` when xs is a variable or a field of one (`s.items`): the last item, removed from xs (Python's list.pop())
pub(super) fn popped_list(list: &Node, call: &Node) -> Option<Node> {
	if !is_place(list) {
		return None;
	}
	match call.drop_meta() {
		Node::List(items, _, _) if matches!(items.as_slice(), [method] if is_word(method, POP_METHOD)) => {
			let template = crate::wasp_parser::parse(&format!(
				"({POP_TEMPORARY} = {POP_PLACE}#count({POP_PLACE}); {POP_PLACE} = slice({POP_PLACE}, 0, count({POP_PLACE})-1); {POP_TEMPORARY})"));
			Some(crate::law::substitute(&template, &std::collections::HashMap::from([(POP_PLACE.to_string(), list.clone())])))
		}
		_ => None,
	}
}

/// `m.remove(k)` when m is a variable: the value of k, its entry removed from m (P35 default, Python's dict.pop); of a
/// list variable the first element equal to k removed, and the list it gives (REMOVED_VALUE_CALL)
pub(super) fn removed_key(map: &Node, call: &Node) -> Option<Node> {
	let Node::Symbol(name) = map.drop_meta() else { return None };
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [method, key] = items.as_slice() else { return None };
	if !is_word(method, REMOVE_METHOD) {
		return None;
	}
	let call = |word: &str, arguments: Vec<Node>| Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None);
	let removed = Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}removed"));
	let value = call(REMOVED_VALUE_CALL, vec![map.clone(), key.clone()]);
	let without = call(crate::library_words::MAP_WITHOUT, vec![map.clone(), key.clone()]);
	Some(Node::List(vec![
		Node::Key(Box::new(removed.clone()), Op::Assign, Box::new(value)),
		Node::Key(Box::new(map.clone()), Op::Assign, Box::new(without)),
		removed,
	], Bracket::Round, Separator::Semicolon))
}

/// `xs.insert(a, b)` when xs is a variable. Wasp writes `insert(value, position)`, Python `insert(position, value)`:
/// the order is never guessed (wiki/Footguns.md "Guessing intent"), `at:` or the kinds decide
pub(super) fn inserted_element(list: &Node, call: &Node) -> Option<Inserted> {
	if !is_place(list) {
		return None;
	}
	let Node::List(items, _, _) = call.drop_meta() else { return None };
	let [method, first, second] = items.as_slice() else { return None };
	if !is_word(method, INSERT_METHOD) {
		return None;
	}
	let position_of = |node: &Node| match node.drop_meta() {
		Node::Key(word, Op::Colon, position) if is_word(word, AT_WORD) => Some(position.as_ref().clone()),
		_ => None,
	};
	Some(match (position_of(first), position_of(second)) {
		(None, Some(position)) => Inserted::At(position, first.clone()),
		(Some(position), None) => Inserted::At(position, second.clone()),
		_ => Inserted::EitherOrder(first.clone(), second.clone()),
	})
}

/// The arguments of `xs.insert(…)`: `insert(x, at: i)` names the position, `insert(a, b)` leaves the order to the kinds
pub(super) enum Inserted {
	At(Node, Node),
	EitherOrder(Node, Node),
}

/// `xs = list_insert_at(xs, position, value)`, or `insert_in_either_order(xs, a, b)` that the emitter resolves by the
/// kinds of a and b: the one Int is the position, two Ints are ambiguous (Python and wasp order differ)
pub(super) fn lowered_insert(list: Box<Node>, call: &Node) -> Node {
	let (pseudo_call, first, second) = match inserted_element(&list, call).expect("guarded") {
		Inserted::At(position, value) => (INSERT_AT_CALL, position, value),
		Inserted::EitherOrder(first, second) => (INSERT_EITHER_CALL, first, second),
	};
	let arguments = vec![Node::Symbol(pseudo_call.to_string()), *list.clone(), lower_declarations(first), lower_declarations(second)];
	Node::Key(list, Op::Assign, Box::new(Node::List(arguments, Bracket::Round, Separator::None)))
}

/// `xs.add(v)`, `xs.insert(i, v)`: an update of the list variable that ø (the empty list) allows
pub(super) fn updates_list(list: &Node, call: &Node) -> bool {
	appended_element(list, call).is_some() || inserted_element(list, call).is_some()
}

/// The zero value of an element type word (`int`, `ints`, `float`, `text` …)
pub(super) fn zero_element(type_word: &str) -> Option<Node> {
	let element = plural_element_type(type_word).unwrap_or(type_word);
	Some(match type_word_kind(element)? {
		Kind::Int => Node::int(0),
		// `0.0f`: a bare 0.0 is an exact decimal, so an Int
		Kind::Float => Node::Key(Box::new(Node::float(0.0)), Op::As, Box::new(Node::Symbol(FLOAT_WORD.to_string()))),
		Kind::Text => Node::Text(String::new()),
		Kind::Codepoint => Node::Char('\0'),
		_ => return None,
	})
}

/// Pseudo-call `zero_fill(count, zero)`: the emitter builds the zero-filled list of `count` elements with a runtime loop,
/// so neither the program nor the compiler grows with the size of the array
pub const ZERO_FILL_CALL: &str = "zero_fill";

/// The zero-filled list of `count` (any number expression) elements of the type word
pub fn zero_list(count: Node, type_word: &str) -> Option<Node> {
	let zero = zero_element(type_word)?;
	Some(Node::List(vec![Node::Symbol(ZERO_FILL_CALL.to_string()), count, zero], Bracket::Round, Separator::None))
}

/// `[x]*n` and `n*[x]` with a list literal: Python repeats the list, NumPy multiplies each element (wiki/Footguns.md
/// "Lists and arithmetic"), so the user is asked; unanswered it is an error naming both explicit forms.
/// Likewise `[1 2 3]+4`: append or add to each element?
pub fn lower_list_times(node: Node) -> Node {
	let list_arithmetic = |key: &Node, positioned: &Node| list_times(key, positioned).or_else(|| list_plus(key, positioned));
	match node {
		Node::Meta { node: inner, data } => {
			let positioned = Node::Meta { node: inner.clone(), data: data.clone() };
			list_arithmetic(&inner, &positioned).unwrap_or_else(|| Node::Meta { node: Box::new(lower_list_times(*inner)), data })
		}
		Node::Key(left, op, right) => {
			let key = Node::Key(Box::new(lower_list_times(*left)), op, Box::new(lower_list_times(*right)));
			list_arithmetic(&key, &key).unwrap_or(key)
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_list_times).collect(), bracket, separator),
		other => other,
	}
}

pub(super) const LIST_TIMES_TOPIC: &str = "list-times";

pub(super) fn list_times(key: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Mul, right) = key.drop_meta() else { return None };
	let (list, count) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) => (left.as_ref(), right.as_ref()),
		(false, true) => (right.as_ref(), left.as_ref()),
		_ => return None,
	};
	let written = key.drop_meta().serialize();
	let (list_text, count_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(count));
	let readings = vec![
		crate::diagnostic::reading("repeat the list", &format!("{count_text} times {list_text}")),
		crate::diagnostic::reading("multiply each element", &format!("{list_text}.map(x => x*{count_text})")),
	];
	let question = crate::diagnostic::Ask::new(LIST_TIMES_TOPIC, format!("type error: list * number: does `{written}` repeat the list or multiply each element?"),
		readings, crate::diagnostic::Fallback::Error).written(&written).at_node(positioned);
	Some(match crate::diagnostic::ask(&question) {
		Ok(0) => filled_list(count.clone(), list).unwrap_or_else(|| crate::node::error("`n times [x]` repeats one element: `3 times [0]`")),
		Ok(_) => {
			let mapped = crate::wasp_parser::parse(&format!("({}).map(item => item * ({}))", list.serialize(), count.serialize()));
			lower_list_times(mapped)
		}
		Err(error) => error,
	})
}

pub(super) const LIST_PLUS_TOPIC: &str = "list-plus";

pub(super) fn is_list_literal(side: &Node) -> bool {
	matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))
}

/// `[1 2 3]+4` and `4+[1 2 3]` with a list literal and a number: append (prepend) or add to each element?
/// Unanswered it is an error naming `[1 2 3] + [4]` and `[1 2 3] .+ 4`
pub(super) fn list_plus(key: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Add, right) = key.drop_meta() else { return None };
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let (list, number, list_first) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) if is_number(right) => (left.as_ref(), right.as_ref(), true),
		(false, true) if is_number(left) => (right.as_ref(), left.as_ref(), false),
		_ => return None,
	};
	let written = key.drop_meta().serialize();
	let (list_text, number_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(number));
	let (joining, joined) = match list_first {
		true => ("append", format!("{list_text} + [{number_text}]")),
		false => ("prepend", format!("[{number_text}] + {list_text}")),
	};
	let readings = vec![
		crate::diagnostic::reading(joining, &joined),
		crate::diagnostic::reading("add to each element", &format!("{list_text} .+ {number_text}")),
	];
	let question = crate::diagnostic::Ask::new(LIST_PLUS_TOPIC, format!("type error: list + number: does `{written}` {joining} {number_text} or add it to each element?"),
		readings, crate::diagnostic::Fallback::Error).written(&written).at_node(positioned);
	let singleton = Node::List(vec![number.clone()], Bracket::Square, Separator::Space);
	Some(match crate::diagnostic::ask(&question) {
		Ok(0) if list_first => Node::Key(Box::new(list.clone()), Op::Add, Box::new(singleton)),
		Ok(0) => Node::Key(Box::new(singleton), Op::Add, Box::new(list.clone())),
		Ok(_) => element_wise(list.clone(), Op::Add, number.clone()),
		Err(error) => error,
	})
}

/// The element name of the lambda an element-wise operator maps with: no wasp program writes it
pub const EACH_ELEMENT: &str = "each_element";

/// `xs .+ n` (also `.-`, `.*`, `./`): the operator applied to each element, `xs.map(each_element => each_element + n)`
pub fn element_wise(list: Node, op: Op, operand: Node) -> Node {
	let element = || Box::new(Node::Symbol(EACH_ELEMENT.to_string()));
	let lambda = Node::Key(element(), Op::FatArrow, Box::new(Node::Key(element(), op, Box::new(operand))));
	let call = Node::List(vec![Node::Symbol("map".to_string()), lambda], Bracket::Round, Separator::None);
	Node::Key(Box::new(list), Op::Dot, Box::new(call))
}

/// `n times [x]`: the list of n copies of x (`zero_fill(n, x)`); `[x]*n` stays ambiguous (Python repeats, NumPy multiplies)
pub fn filled_list(count: Node, list: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Square, _) = list.drop_meta() else { return None };
	let [element] = items.as_slice() else { return None };
	Some(Node::List(vec![Node::Symbol(ZERO_FILL_CALL.to_string()), count, element.clone()], Bracket::Round, Separator::None))
}

/// The zero-filled list of the array type written `int[100]` (a 1-based subscript, see `subscript`)
pub(super) fn subscripted_array_type(type_node: &Node) -> Option<Node> {
	match type_node.drop_meta() {
		Node::Key(element, Op::Hash, one_based) => match (element.drop_meta(), one_based.drop_meta()) {
			(Node::Symbol(word), Node::Number(Number::Int(one_based))) => zero_list(Node::int(one_based - 1), word),
			_ => None,
		},
		_ => None,
	}
}

/// `int[n]` parsed as the subscript `int#(n+1)`: the list of n zeros when the type word is no variable
pub(super) fn zero_filled_subscript(element: &Node, one_based: &Node, variables: &HashSet<String>) -> Option<Node> {
	let Node::Symbol(word) = element.drop_meta() else { return None };
	if variables.contains(word) {
		return None;
	}
	let count = match one_based.drop_meta() {
		Node::Number(Number::Int(one_based)) => Node::int(one_based - 1),
		other => crate::wasp_parser::subscript_key(other)?.clone(),
	};
	zero_list(count, word)
}

/// `int[100]` or `100 * int`: the zero-filled list of that many elements (the parser reads `int[n]` as one already)
pub(super) fn typed_array_value(value: &Node) -> Option<Node> {
	match value.drop_meta() {
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(call)) if call == ZERO_FILL_CALL) => Some(value.clone()),
		Node::Key(count, Op::Mul, element) => match element.drop_meta() {
			Node::Symbol(word) => zero_list(count.as_ref().clone(), word),
			_ => None,
		},
		other => subscripted_array_type(other),
	}
}

/// The type `[number]`: `list of number` for one element type word
pub(super) fn bracketed_list_type(type_node: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Square, _) = type_node.drop_meta() else { return None };
	let [element] = items.as_slice() else { return None };
	let Node::Symbol(word) = element.drop_meta() else { return None };
	type_word_kind(word)?;
	Some(Node::Symbol(format!("list of {word}")))
}

/// `x:int[100]` as the variable and its zero-filled list
pub(super) fn typed_array_declaration(node: &Node) -> Option<(Node, Node)> {
	let Node::Key(name, Op::Colon, type_node) = node.drop_meta() else { return None };
	let Node::Symbol(_) = name.drop_meta() else { return None };
	Some((name.drop_meta().clone(), typed_array_value(type_node)?))
}

/// `x : 100` followed by the element type `int`: the declaration `x = [0 … 0]`
pub(super) fn counted_array_declaration(declared_count: &Node, type_word: &Node) -> Option<Node> {
	let Node::Key(name, Op::Colon, count) = declared_count.drop_meta() else { return None };
	match (name.drop_meta(), count.drop_meta(), type_word.drop_meta()) {
		(Node::Symbol(_), Node::Number(Number::Int(count)), Node::Symbol(word)) => {
			Some(Node::Key(Box::new(name.drop_meta().clone()), Op::Assign, Box::new(zero_list(Node::int(*count), word)?)))
		}
		_ => None,
	}
}

/// Two neighbouring statements that together form one declaration
pub(super) fn paired_declaration(first: &Node, second: &Node) -> Option<Node> {
	prefixed_declaration(first, second).or_else(|| counted_array_declaration(first, second))
}

/// The type a lowered declaration `x:T = v` attached to its target x
pub(super) fn declared_type(target: &Node) -> Option<&Node> {
	match target {
		Node::Meta { data, .. } if matches!(data.as_ref(), Node::Symbol(_)) => Some(data),
		_ => None,
	}
}

/// A builtin type written before a declaration: `int`, `string`, `float`, `real`, `fast` … (see `canonical_type_name`)
pub(super) fn is_declaration_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if builtin_type_kind(name).is_some() || plural_element_type(name).is_some())
}

/// `(fast x)` as the target of `fast x=v`: the type and the name
pub(super) fn number_type_prefix(target: &Node) -> Option<(Node, Node)> {
	match target.drop_meta() {
		Node::List(items, bracket, _) if *bracket != Bracket::Round && items.len() == 2 && is_declaration_type(&items[0]) && matches!(items[1].drop_meta(), Node::Symbol(_)) => {
			Some((items[0].drop_meta().clone(), items[1].drop_meta().clone()))
		}
		_ => None,
	}
}

/// The statement pair `fast`, `x=v` as the declaration `x:fast=v`
pub(super) fn prefixed_declaration(type_name: &Node, next: &Node) -> Option<Node> {
	match next.drop_meta() {
		Node::Key(name, op @ (Op::Assign | Op::Define), value) if is_declaration_type(type_name) && matches!(name.drop_meta(), Node::Symbol(_)) => {
			let target = Node::Key(Box::new(name.drop_meta().clone()), Op::Colon, Box::new(type_name.drop_meta().clone()));
			Some(Node::Key(Box::new(target), *op, value.clone()))
		}
		_ => None,
	}
}
