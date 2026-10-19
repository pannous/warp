//! Class bodies as their members: fields as Java, C#, Go and Python write them, properties and accessors, constructors as `init`, methods with their result types
use super::*;
use crate::lowering::nodes::{assign, block, word_headed};
use crate::node::parens;
use crate::words::SELF_WORD;

/// The fields and methods of a class body; the groups a `;` makes (`{name age:int; greet() := …}`) flattened
pub(super) fn class_items(body: &Node) -> Vec<Node> {
	let items = match body.drop_meta() {
		// a block of one statement `{fn sum() …}` is that statement's words
		Node::List(words, Bracket::Curly, Separator::Space)
			if words.first().is_some_and(
				|first| matches!(first.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)),
			) || auto_property(words).is_some()
				|| typed_field(words).is_some() =>
		{
			vec![Node::List(words.clone(), Bracket::None, Separator::Space)]
		}
		_ => body.as_items(),
	};
	items
		.into_iter()
		.map(without_modifiers)
		.map(with_body_words)
		.flat_map(|item| match item.drop_meta() {
			Node::List(words, _, _) if keyword_method(words).is_some() => keyword_method(words).into_iter().collect(),
			Node::List(words, _, _) if braced_method(words).is_some() => braced_method(words).into_iter().collect(),
			Node::List(words, _, _) if value_block(words).is_some() => value_block(words).into_iter().collect(),
			Node::List(words, _, _) if accessors(words).is_some() => accessors(words).unwrap_or_default(),
			Node::List(..) if included_mixin(&item).is_some() => vec![item],
			Node::List(words, _, _) if nested_fields(words).is_some() => nested_fields(words).into_iter().collect(),
			// Java's `int x`: the field x of type int; C#'s auto-property `int X { get; set; }` the field X
			Node::List(words, _, _) if typed_field(words).is_some() => typed_field(words).into_iter().collect(),
			Node::List(words, _, _) if auto_property(words).is_some() => auto_property(words).into_iter().collect(),
			// TypeScript's `twice(): number { … }`
			Node::Key(call, Op::Colon, typed_block) if typed_method(call, typed_block).is_some() => {
				typed_method(call, typed_block).into_iter().collect()
			}
			Node::List(group, Bracket::None, _) => group.clone(),
			_ => vec![item],
		})
		.map(as_member)
		.collect()
}

/// `fn f() := 3 squared`: the parser ends the definition before a suffix word, `(f() := 3) squared`; the words after it
/// are its body's, as ambiguous_forms regroups them outside a class
pub(super) fn with_body_words(item: Node) -> Node {
	let Node::List(words, Bracket::None, Separator::Space) = item.drop_meta() else {
		return item;
	};
	let Some(at) = words.iter().position(|word| matches!(word.drop_meta(), Node::Key(head, Op::Define, _) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)))) else { return item };
	let (before, rest) = words.split_at(at + 1);
	if rest.is_empty() {
		return item;
	}
	let Node::Key(head, op, value) = before[at].drop_meta() else {
		return item;
	};
	let body = Node::List([vec![value.as_ref().clone()], rest.to_vec()].concat(), Bracket::None, Separator::Space);
	let definition = Node::Key(head.clone(), *op, Box::new(body));
	Node::List([before[..at].to_vec(), vec![definition]].concat(), Bracket::None, Separator::Space)
}

/// `def area() -> int {…}`, `fun area(): Int {…}`, `func area() {…}`: the method `area() := …` (with its result type)
pub(super) fn keyword_method(words: &[Node]) -> Option<Node> {
	// Kotlin's expression body `fun sum() = x + y` defines as `:=` does; Java's `int sum() {…}` as C's
	match crate::declarations::keyword_definition(words)
		.or_else(|| crate::declarations::c_function(words))
		.or_else(|| python_method(words))?
	{
		Node::Key(head, Op::Assign, body) => Some(Node::Key(head, Op::Define, body)),
		definition => Some(definition),
	}
}

/// Python's `def f(): x + 1` without parameters, which keyword_definition leaves to late_binding outside a class (P71):
/// in a class body it is the method `f() := x + 1`
pub(super) fn python_method(words: &[Node]) -> Option<Node> {
	let [keyword, definition] = words else { return None };
	let is_keyword = matches!(keyword.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word));
	match definition.drop_meta() {
		Node::Key(head, Op::Colon, body)
			if is_keyword
				&& matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if word_headed(items).is_some()) =>
		{
			Some(Node::Key(head.clone(), Op::Define, body.clone()))
		}
		_ => None,
	}
}

/// A property's getter and setter as methods: `get age() {…}` is the getter `age := …`, `set age(v) {…}` the setter
/// `age·set(v) := …`; the warp form `age:{getter} set{setter}` has the new value as `it`
pub(super) fn accessors(words: &[Node]) -> Option<Vec<Node>> {
	let [first, second] = words else { return None };
	let method = |name: &str, parameters: Vec<Node>, body: &Node| {
		let head = match parameters.is_empty() {
			true => symbol(name),
			false => call(name, parameters),
		};
		key(head, Op::Define, body.clone())
	};
	match (first.drop_meta(), second.drop_meta()) {
		// `get age() {…}`, `set age(v) {…}`
		(Node::Symbol(word), Node::List(parts, _, _))
			if crate::warp_parser::ACCESSOR_WORDS.contains(&word.as_str()) =>
		{
			let [call, body] = parts.as_slice() else { return None };
			let Node::List(call, Bracket::Round, _) = call.drop_meta() else {
				return None;
			};
			let name = call.first()?.drop_meta().name();
			let parameters: Vec<Node> = call[1..].iter().flat_map(grouped_parameters).collect();
			match word == "get" {
				true => Some(vec![method(&name, vec![], body)]),
				false => Some(vec![method(&format!("{name}{SETTER_SUFFIX}"), parameters, body)]),
			}
		}
		// `age:{2026 - birthday} set{birthday = 2026 - it}`
		(Node::Key(name, Op::Colon | Op::None, getter), Node::Key(set, Op::Colon | Op::None, setter))
			if set.drop_meta().name() == "set" && getter.is_block() && setter.is_block() =>
		{
			let name = name.drop_meta().name();
			// `it` is the new value; named apart, so no pass reads the block as a lambda of `it`
			let new_value = Node::Symbol(format!("{}{SETTER_SUFFIX}", crate::words::IMPLICIT_PARAMETER));
			let setter = renamed_symbol(setter.as_ref().clone(), crate::words::IMPLICIT_PARAMETER, &new_value);
			Some(vec![
				method(&name, vec![], getter),
				method(&format!("{name}{SETTER_SUFFIX}"), vec![new_value], &setter),
			])
		}
		_ => None,
	}
}

/// The properties the classes of the program give setters
pub(super) fn property_setters(node: &Node) -> Vec<String> {
	let mut setters = vec![];
	node.visit(&mut |part| {
		if let Node::Type { body, .. } = part {
			setters.extend(
				class_items(body)
					.iter()
					.filter_map(method_parts)
					.filter_map(|(name, _, _)| name.strip_suffix(SETTER_SUFFIX).map(str::to_string)),
			);
		}
	});
	setters
}

pub(super) fn renamed_symbol(node: Node, name: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(symbol) if symbol == name => replacement.clone(),
		other => other.map_children(|child| renamed_symbol(child, name, replacement)),
	}
}

/// `p.age = v` of a property with a setter: `p = age·set(p, v)`
pub(super) fn setter_calls(node: Node, setters: &[String]) -> Node {
	let recurse = |child: Node| setter_calls(child, setters);
	// inside the setter `age·set(self, v)` writing age stores the field, as `p.age` inside `age(p)` reads it: a call
	// would set it again forever
	if let Node::Key(head, op @ (Op::Define | Op::Assign), body) = &node {
		let property = match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().and_then(Node::symbol_name),
			_ => None,
		}
		.and_then(|name| name.strip_suffix(SETTER_SUFFIX));
		if let Some(property) = property {
			let others: Vec<String> = setters.iter().filter(|setter| *setter != property).cloned().collect();
			return Node::Key(head.clone(), *op, Box::new(setter_calls(body.as_ref().clone(), &others)));
		}
	}
	match node {
		Node::Key(target, Op::Assign, value) => match target.drop_meta() {
			Node::Key(receiver, Op::Dot, property)
				if matches!(receiver.drop_meta(), Node::Symbol(_))
					&& setters.contains(&property.drop_meta().name()) =>
			{
				let setter = Node::Symbol(format!("{}{SETTER_SUFFIX}", property.drop_meta().name()));
				let call = parens(vec![setter, receiver.as_ref().clone(), recurse(*value)]);
				Node::Key(receiver.clone(), Op::Assign, Box::new(call))
			}
			_ => assign(recurse(*target), recurse(*value)),
		},
		other => other.map_children(recurse),
	}
}

/// A member without the words that change nothing in warp: `mutating func f() {…}` is `func f() {…}`, Swift's
/// `var count = 0` the field `count = 0`
pub(super) fn without_modifiers(item: Node) -> Node {
	let Node::List(words, bracket, separator) = item.drop_meta().clone() else {
		return item;
	};
	let is_modifier = |word: &Node| matches!(word.drop_meta(), Node::Symbol(word) if crate::warp_parser::MEMBER_MODIFIERS.contains(&word.as_str()) || crate::warp_parser::FIELD_KEYWORDS.contains(&word.as_str()));
	let kept: Vec<Node> = words.iter().skip_while(|word| is_modifier(word)).cloned().collect();
	if let Some(kept_word) = kept.first().map(leading_name).filter(|_| kept.len() < words.len()) {
		let modifiers: Vec<String> =
			words[..words.len() - kept.len()].iter().map(|word| word.drop_meta().name()).collect();
		if !modifiers.iter().any(|word| SILENT_MODIFIERS.contains(&word.as_str())) {
			crate::normalize::set_position_of(&item);
			crate::errors::diagnostic::note_alias(&format!("{} {kept_word}", modifiers.join(" ")), &kept_word);
		}
	}
	match kept.len() {
		length if length == words.len() => item,
		1 => kept.into_iter().next().expect("one"),
		_ => Node::List(kept, bracket, separator),
	}
}

/// wiki/class.md's `address { street; city; zip? }`: the field address holding a block of fields, `address:{…}`
pub(super) fn nested_fields(words: &[Node]) -> Option<Node> {
	let [name, block] = words else { return None };
	let word = name.symbol_name()?;
	let Node::List(fields, Bracket::Curly, _) = block.drop_meta() else {
		return None;
	};
	let is_field = |field: &Node| match field.drop_meta() {
		Node::Symbol(_) => true,
		Node::Key(_, Op::Colon, _) => true,
		Node::List(group, Bracket::None, _) => {
			group.iter().all(|field| matches!(field.drop_meta(), Node::Symbol(_) | Node::Key(_, Op::Colon, _)))
		}
		_ => false,
	};
	let is_member_word = crate::operators::is_function_keyword(word)
		|| is_constructor_word(word)
		|| crate::warp_parser::ACCESSOR_WORDS.contains(&word);
	(!is_member_word && !fields.is_empty() && fields.iter().all(is_field))
		.then(|| key(name.clone(), Op::Colon, block.clone()))
}

/// Java's and C#'s field `int x`, Go's `x int`: the field `x:int`
pub(super) fn typed_field(words: &[Node]) -> Option<Node> {
	let is_type = |word: &Node| crate::analyzer::type_word_kind(&word.drop_meta().name()).is_some();
	let field = |name: &Node, field_type: &Node| key(name.clone(), Op::Colon, field_type.clone());
	match words {
		[field_type, name] if is_type(field_type) && matches!(name.drop_meta(), Node::Symbol(_)) => {
			Some(field(name, field_type))
		}
		[name, field_type] if is_type(field_type) && matches!(name.drop_meta(), Node::Symbol(_)) => {
			Some(field(name, field_type))
		}
		_ => None,
	}
}

/// C#'s auto-property `int X { get; set; }` (or `{ get; init; }`): the field `X:int`
pub(super) fn auto_property(words: &[Node]) -> Option<Node> {
	let [field_type, name, accessors] = words else {
		return None;
	};
	let Node::List(accessors, Bracket::Curly, _) = accessors.drop_meta() else {
		return None;
	};
	let only_accessors = !accessors.is_empty()
		&& accessors.iter().all(|accessor| matches!(accessor.drop_meta().name().as_str(), "get" | "set" | "init"));
	only_accessors.then(|| typed_field(&[field_type.clone(), name.clone()])).flatten()
}

/// TypeScript's method with its result type, `twice(): number { … }`: `twice():number := {…}`
pub(super) fn typed_method(call: &Node, typed_block: &Node) -> Option<Node> {
	let is_call = matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if word_headed(items).is_some());
	let Node::List(parts, _, _) = typed_block.drop_meta() else {
		return None;
	};
	let [result_type, block] = parts.as_slice() else {
		return None;
	};
	let is_block = block.is_block();
	(is_call && is_block).then(|| key(key(call.clone(), Op::Colon, result_type.clone()), Op::Define, block.clone()))
}

/// Python's class attribute: a field `count = 0` read as `Counter.count` is the class's own, shared by its instances
/// (the static `static count = 0`)
pub(super) fn with_class_attributes(node: Node) -> Node {
	let mut qualified: Vec<(String, String)> = vec![];
	node.visit(&mut |part| {
		if let Node::Key(class, Op::Dot, member) = part {
			if let (Node::Symbol(class), Node::Symbol(member)) = (class.drop_meta(), member.drop_meta()) {
				qualified.push((class.clone(), member.clone()));
			}
		}
	});
	if qualified.is_empty() {
		return node;
	}
	class_attributes(node, &qualified)
}

pub(super) fn class_attributes(node: Node, qualified: &[(String, String)]) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			// a field with its value `count = 0` (a parameter named like the class reads `photo.width` of an instance)
			let has_value = |item: &Node| matches!(item.drop_meta(), Node::Key(_, Op::Assign, _));
			let is_attribute = |item: &Node| {
				method_parts(item).is_none()
					&& has_value(item)
					&& declared_field_name(item).is_some_and(|field| qualified.contains(&(class.clone(), field)))
			};
			let items = class_items(&body);
			if !items.iter().any(|item| is_attribute(item) && !is_static(item)) {
				return Node::Type { name, body };
			}
			let items = items
				.into_iter()
				.map(|item| match is_attribute(&item) {
					true => item.with_attribute(crate::warp_parser::STATIC_KEYWORD, Node::True),
					false => item,
				})
				.collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(|child| class_attributes(child, qualified)),
	}
}

/// The names of the classes the program declares whose body `keep` accepts
pub(super) fn declared_classes(node: &Node, keep: impl Fn(&Node) -> bool) -> Vec<String> {
	let mut classes = vec![];
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			if keep(body) {
				classes.push(name.drop_meta().name());
			}
		}
	});
	classes
}

/// Go's positional braces `Point{1, 2}` of a declared class: the construction `Point(1, 2)`, with a note (P167); of
/// an unknown name they stay tagged data
pub(super) fn positional_braces(node: Node) -> Node {
	let classes = declared_classes(&node, |_| true);
	if classes.is_empty() {
		return node;
	}
	constructed_from_braces(node, &classes)
}

pub(super) fn constructed_from_braces(node: Node, classes: &[String]) -> Node {
	let is_positional =
		|items: &[Node]| !items.is_empty() && items.iter().all(|item| !matches!(item.drop_meta(), Node::Key(..)));
	match node {
		Node::Key(class, Op::None, braces)
			if matches!(class.drop_meta(), Node::Symbol(name) if classes.contains(name))
				&& matches!(braces.drop_meta(), Node::List(items, Bracket::Curly, _) if is_positional(items)) =>
		{
			let Node::List(items, _, _) = braces.drop_meta().clone() else { unreachable!("guarded") };
			let class_name = class.drop_meta().name();
			let written: Vec<String> = items.iter().map(Node::serialize).collect();
			crate::normalize::set_position_of(&class);
			crate::errors::diagnostic::note_alias(
				&format!("{class_name}{{{}}}", written.join(", ")),
				&format!("{class_name}({})", written.join(", ")),
			);
			let arguments = items.into_iter().map(|item| constructed_from_braces(item, classes));
			parens([vec![*class]].into_iter().flatten().chain(arguments).collect())
		}
		// a match arm's `Point{x, y} =>` is a pattern (destructurings), no construction
		Node::Key(pattern, Op::Arrow, body) => {
			Node::Key(pattern, Op::Arrow, Box::new(constructed_from_braces(*body, classes)))
		}
		other => other.map_children(|child| constructed_from_braces(child, classes)),
	}
}

/// Every class's constructor as `init(…){…}` (P162): one named by another language (`constructor(x)`, `__init__`,
/// CONSTRUCTOR_ALIASES) or like its class (Java's `Point(int x, int y) {…}`), with a got-it note; an alias the program
/// also calls as a method (`p.new(2)`) stays a method
pub(super) fn with_init_constructors(node: Node, called: &std::collections::HashSet<String>) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let is_alias = |item: &Node| {
				constructor_alias(item).filter(|alias| {
					*alias == class
						|| ((CONSTRUCTOR_ALIASES.contains(&alias.as_str()) || *alias == CONSTRUCTOR_WORD)
							&& !called.contains(alias))
				})
			};
			let items: Vec<Node> = class_items(&body);
			if !items.iter().any(|item| is_alias(item).is_some()) {
				return Node::Type { name, body };
			}
			let items = items
				.into_iter()
				.map(|item| match is_alias(&item) {
					// `init(xs) := {…}` is the constructor too, without a note
					Some(alias) if alias == CONSTRUCTOR_WORD => as_init(item),
					Some(alias) => {
						crate::normalize::set_position_of(&item);
						crate::errors::diagnostic::note_alias(&alias, CONSTRUCTOR_WORD);
						as_init(item)
					}
					None => item,
				})
				.collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(|child| with_init_constructors(child, called)),
	}
}

/// The name a constructor-like member is written with: `constructor` of `constructor(x){…}`, `Point` of `Point(x) := …`
pub(super) fn constructor_alias(item: &Node) -> Option<String> {
	match item.drop_meta() {
		Node::Key(word, Op::None | Op::Colon, block)
			if block.is_block()
				&& constructor_parameters(word).is_some() =>
		{
			Some(leading_name(word))
		}
		_ => method_parts(item).map(|(method, _, _)| method),
	}
}

/// A constructor-like member as the constructor `init(…){…}`
pub(super) fn as_init(item: Node) -> Node {
	let (parameters, body) = match (constructor_parts(&item), method_parts(&item)) {
		(Some((parameters, body)), _) => (parameters, body.clone()),
		(None, Some((_, parameters, body))) => (parameters, curly(body)),
		(None, None) => return item,
	};
	let word = call(CONSTRUCTOR_WORD, parameters);
	key(word, Op::None, body)
}

/// The names the program calls as methods, `new` of `p.new(2)` and of Rust's `Point::new(1, 2)`
pub(super) fn method_calls_named(node: &Node) -> std::collections::HashSet<String> {
	let mut names = std::collections::HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(_, Op::Dot | Op::SafeDot | Op::Scope, member) = part {
			names.insert(leading_name(member));
		}
	});
	names
}

/// Every class body as its members (class_items): Java's `int x` fields and C#'s auto-properties are fields also in a
/// class without methods, which no later pass splits
pub(super) fn with_members(node: Node) -> Node {
	match node {
		Node::Type { name, body } if name.attribute(crate::warp_parser::MIXIN_WORD).is_none() => {
			let members = block(class_items(&body));
			let changed = members.serialize() != block(class_items_as_written(&body)).serialize();
			Node::Type { name, body: Box::new(if changed { members } else { *body }) }
		}
		other => other.map_children(with_members),
	}
}

/// The items of a class body as written, its `;` groups flattened, nothing read into them
pub(super) fn class_items_as_written(body: &Node) -> Vec<Node> {
	body.as_items()
		.into_iter()
		.flat_map(|item| match item.drop_meta() {
			Node::List(group, Bracket::None, _) => group.clone(),
			_ => vec![item],
		})
		.collect()
}

/// Java's and C#'s `Point p = new Point(3, 4);`, a variable declared with a class as its type: `p:Point = Point(3, 4)`
/// (the type stays: a call may pick its overload by it, D10)
pub(super) fn class_typed_declarations(node: Node) -> Node {
	let classes = declared_classes(&node, |_| true);
	if classes.is_empty() {
		return node;
	}
	declared_with_classes(node, &classes)
}

pub(super) fn declared_with_classes(node: Node, classes: &[String]) -> Node {
	match node {
		// not the call `P(x = 7)`, a construction with a named argument
		Node::List(words, bracket, _)
			if bracket != Bracket::Round
				&& words.len() == 2
				&& classes.contains(&words[0].drop_meta().name())
				&& matches!(words[1].drop_meta(), Node::Key(variable, Op::Assign, _) if matches!(variable.drop_meta(), Node::Symbol(_))) =>
		{
			let Node::Key(variable, _, value) = words[1].drop_meta().clone() else { unreachable!("guarded") };
			let typed = Node::Key(variable, Op::Colon, Box::new(words[0].clone()));
			assign(typed, declared_with_classes(*value, classes))
		}
		other => other.map_children(|child| declared_with_classes(child, classes)),
	}
}

/// JavaScript's `sum() { return … }`: the method `sum() := {…}`
pub(super) fn braced_method(words: &[Node]) -> Option<Node> {
	let [call, block] = words else { return None };
	let is_call = matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().and_then(Node::symbol_name).is_some_and(|name| !is_constructor_word(name)));
	let is_block = block.is_block();
	(is_call && is_block).then(|| key(call.clone(), Op::Define, block.clone()))
}

/// A method as other languages write it, as a class member: Python's explicit `self` first parameter dropped (the
/// method gets its receiver anyway), a method named by an operator's glyph under the operator's english name
pub(super) fn as_member(definition: Node) -> Node {
	let Node::Key(head, Op::Define, body) = definition.drop_meta().clone() else {
		return definition;
	};
	// `sum(self):i32 := …`: the head under its result type
	if let Node::Key(call, Op::Colon, result_type) = head.drop_meta().clone() {
		if matches!(call.drop_meta(), Node::List(_, Bracket::Round, _)) {
			return match as_member(Node::Key(call, Op::Define, body)) {
				Node::Key(call, Op::Define, body) => {
					Node::Key(Box::new(Node::Key(call, Op::Colon, result_type)), Op::Define, body)
				}
				constructor => constructor,
			};
		}
	}
	let Node::List(call, Bracket::Round, separator) = head.drop_meta().clone() else {
		return definition;
	};
	let Some((name, parameters)) = call.split_first() else {
		return definition;
	};
	let parameters: Vec<Node> = parameters.iter().flat_map(grouped_parameters).collect();
	// `self`, `self: Self`, Rust's `&self`
	let is_self = |parameter: &Node| match parameter.drop_meta() {
		Node::Symbol(word) => word == SELF_WORD,
		Node::Key(word, Op::Colon, _) => word.drop_meta().name() == SELF_WORD,
		Node::Key(empty, _, word) => empty.is_empty_node() && word.drop_meta().name() == SELF_WORD,
		_ => false,
	};
	let parameters = match parameters.first() {
		Some(first) if is_self(first) => parameters[1..].to_vec(),
		_ => parameters,
	};
	let name = glyph_method(&name.drop_meta().name()).map_or(name.clone(), symbol);
	Node::Key(Box::new(Node::List([vec![name], parameters].concat(), Bracket::Round, separator)), Op::Define, body)
}

/// A body as the block `{…}` a constructor has
pub(super) fn curly(body: Node) -> Node {
	match body {
		Node::List(items, Bracket::Curly, separator) => Node::List(items, Bracket::Curly, separator),
		Node::List(items, Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List(items, Bracket::Curly, separator)
		}
		other => block(vec![other]),
	}
}

/// `init {…}` (spaced) and `init(name) {…}`: the constructor `init{…}`, `init(name):{…}`
pub(super) fn value_block(words: &[Node]) -> Option<Node> {
	let [word, block] = words else { return None };
	let is_block = block.is_block();
	(is_block && constructor_parameters(word).is_some()).then(|| key(word.clone(), Op::None, block.clone()))
}

pub(super) fn is_constructor_word(name: &str) -> bool {
	name == CONSTRUCTOR_WORD || CONSTRUCTOR_ALIASES.contains(&name)
}

/// The parameters `init` or `init(name)` declares (or an alias's, `value(name)`)
pub(super) fn constructor_parameters(word: &Node) -> Option<Vec<Node>> {
	match word.drop_meta() {
		Node::Symbol(name) if is_constructor_word(name) => Some(vec![]),
		// `init(a, b)`, `init (n)`: the parameters may come as one group
		Node::List(call, Bracket::Round, _)
			if call.first().and_then(Node::symbol_name).is_some_and(is_constructor_word) =>
		{
			Some(call[1..].iter().flat_map(grouped_parameters).collect())
		}
		_ => None,
	}
}

/// The parameters and statements of the constructor `init{…}`, `init(name){…}`
pub(super) fn constructor_parts(item: &Node) -> Option<(Vec<Node>, &Node)> {
	match item.drop_meta() {
		// glued `init{…}` arrives as the key `init:{…}`
		Node::Key(word, Op::None | Op::Colon, block)
			if block.is_block() =>
		{
			Some((constructor_parameters(word)?, block))
		}
		_ => None,
	}
}

pub(super) fn constructor_body(item: &Node) -> Option<&Node> {
	constructor_parts(item).map(|(_, body)| body)
}

/// The fields a constructor body sets, `id = …` or `this.id = …`
/// (`this.x = x` sets the field x also when a parameter is called x; a bare `n = …` of a parameter n sets the parameter)
pub(super) fn fields_set(body: &Node, parameters: &[String]) -> Vec<String> {
	let mut names = vec![];
	collect_fields_set(body, parameters, &mut names);
	names
}

/// The fields `fields_set` finds; what a loop sets (`for x in xs { i = … }`) stays local to the constructor
pub(super) fn collect_fields_set(node: &Node, parameters: &[String], names: &mut Vec<String>) {
	if crate::event_signals::is_loop(node) {
		return;
	}
	match node.drop_meta() {
		Node::Key(target, op, value) => {
			let name = match target.drop_meta() {
				_ if *op != Op::Assign => None,
				Node::Symbol(name) if !parameters.contains(name) => Some(name.clone()),
				Node::Key(receiver, Op::Dot, field)
					if RECEIVER_ALIASES.contains(&receiver.drop_meta().name().as_str())
						|| receiver.drop_meta().name() == SELF_WORD =>
				{
					Some(field.drop_meta().name())
				}
				_ => None,
			};
			if let Some(name) = name.filter(|name| !names.contains(name)) {
				names.push(name);
			}
			collect_fields_set(target, parameters, names);
			collect_fields_set(value, parameters, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_fields_set(item, parameters, names)),
		_ => {}
	}
}

/// The result type a method declares: `area():int := …`
pub(super) fn result_type(item: &Node) -> Option<&Node> {
	match item.drop_meta() {
		Node::Key(head, Op::Define, _) => match head.drop_meta() {
			Node::Key(_, Op::Colon, result_type) => Some(result_type),
			_ => None,
		},
		_ => None,
	}
}

/// A definition with the result type on its head
pub(super) fn with_result_type(definition: Node, result_type: Option<&Node>) -> Node {
	match (definition, result_type) {
		(Node::Key(head, Op::Define, body), Some(result_type)) => {
			Node::Key(Box::new(Node::Key(head, Op::Colon, Box::new(result_type.clone()))), Op::Define, body)
		}
		(definition, _) => definition,
	}
}
