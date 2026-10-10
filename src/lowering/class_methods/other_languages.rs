//! Classes as other languages write them: Rust's `impl Point {…}`, Go's `func (p Point) Sum() int {…}`, Ruby's
//! `Point.new(1, 2)`, and the text, equality and order methods by their other names (`toString`, `__eq__`, `compareTo`)
use super::*;

/// Rust's `impl Point { fn sum(&self) -> i32 {…} }` (and `impl Trait for Point {…}`): its functions are methods of
/// the class Point, the impl block itself goes
pub(super) fn with_impls(node: Node) -> Node {
	let mut impls: Vec<(String, Vec<Node>)> = vec![];
	node.visit(&mut |part| if let Some((class, items)) = impl_block(part) {
		impls.push((class, items));
	});
	if impls.is_empty() {
		return node;
	}
	taking_in_impls(node, &impls)
}

pub(super) fn taking_in_impls(node: Node, impls: &[(String, Vec<Node>)]) -> Node {
	match node {
		_ if impl_block(&node).is_some() => Node::Empty,
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let added: Vec<Node> = impls.iter().filter(|(owner, _)| *owner == class).flat_map(|(_, items)| items.clone()).collect();
			match added.is_empty() {
				true => Node::Type { name, body },
				false => Node::Type { name, body: Box::new(Node::List([class_items(&body), added].concat(), Bracket::Curly, Separator::Semicolon)) },
			}
		}
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| taking_in_impls(item, impls)).filter(|item| !matches!(item, Node::Empty)).collect();
			Node::List(items, bracket, separator)
		}
		Node::Meta { node, data } => match taking_in_impls(*node, impls) {
			Node::Empty => Node::Empty,
			node => Node::Meta { node: Box::new(node), data },
		},
		other => other,
	}
}

/// `impl Point {…}`, `impl Display for Point {…}`: the class and the items of the block
pub(super) fn impl_block(node: &Node) -> Option<(String, Vec<Node>)> {
	let Node::List(words, _, _) = node.drop_meta() else { return None };
	let (class, block) = match words.as_slice() {
		[word, class, block] if word.drop_meta().name() == IMPL_WORD => (class, block),
		// `impl Point {…}` of a declared Point reads like the spaced construction `Point {…}`
		[word, construction] if word.drop_meta().name() == IMPL_WORD => match construction.drop_meta() {
			Node::Key(class, Op::None | Op::Colon, block) => (class.as_ref(), block.as_ref()),
			_ => return None,
		},
		[word, _, for_word, class, block] if word.drop_meta().name() == IMPL_WORD && for_word.drop_meta().name() == "for" => (class, block),
		_ => return go_method(words),
	};
	let Node::Symbol(class) = class.drop_meta() else { return None };
	match block.drop_meta() {
		Node::List(_, Bracket::Curly, _) => Some((class.clone(), class_items(block))),
		_ => None,
	}
}

/// Go's method `func (p Point) Sum() int {…}` (or of the pointer `(p *Point)`): the class and the method `func Sum() int
/// {…}`, its receiver p read as self
pub(super) fn go_method(words: &[Node]) -> Option<(String, Vec<Node>)> {
	let [keyword, receiver, rest @ ..] = words else { return None };
	if keyword.drop_meta().name() != GO_FUNCTION_WORD || rest.is_empty() {
		return None;
	}
	let Node::List(receiver, Bracket::Round, _) = receiver.drop_meta() else { return None };
	let (variable, class) = match receiver.as_slice() {
		[variable, class] => (variable.drop_meta(), class.drop_meta()),
		[single] => match single.drop_meta() {
			Node::Key(variable, Op::Mul, class) => (variable.drop_meta(), class.drop_meta()),
			_ => return None,
		},
		_ => return None,
	};
	let class = match class {
		Node::Key(empty, Op::Mul, class) if matches!(empty.drop_meta(), Node::Empty) => class.drop_meta(),
		class => class,
	};
	let (Node::Symbol(variable), Node::Symbol(class)) = (variable, class) else { return None };
	let rest: Vec<Node> = match rest {
		[Node::List(group, Bracket::None, _)] => group.clone(),
		_ => rest.to_vec(),
	};
	let bindings = [(variable.clone(), symbol(RECEIVER))].into_iter().collect();
	let method = crate::law::substitute(&Node::List([vec![keyword.clone()], rest].concat(), Bracket::None, Separator::Space), &bindings);
	Some((class.clone(), class_items(&Node::List(vec![method], Bracket::Curly, Separator::Semicolon))))
}

/// Ruby's `Point.new(1, 2)` of a declared class that defines no `new`: the construction `Point(1, 2)`, with a note
pub(super) fn ruby_constructions(node: Node) -> Node {
	let classes = declared_classes(&node, |body| !class_items(body).iter().filter_map(method_parts).any(|(method, _, _)| method == RUBY_NEW_WORD));
	if classes.is_empty() {
		return node;
	}
	constructed_by_new(node, &classes)
}

/// Ruby's `Point.new(1, 2)` and Java's `new Set(xs)` of a class the parser did not see (a standard module's): the
/// construction, with a note
pub(super) fn constructed_by_new(node: Node, classes: &[String]) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [new, call] if new.drop_meta().name() == RUBY_NEW_WORD && classes.contains(&leading_name(call))) => {
			let call = items[1].clone();
			crate::diagnostic::note_alias(&format!("{RUBY_NEW_WORD} {}", leading_name(&call)), &leading_name(&call));
			constructed_by_new(call, classes)
		}
		Node::Key(class, Op::Dot, member) if classes.contains(&class.drop_meta().name()) && matches!(class.drop_meta(), Node::Symbol(_)) && leading_name(&member) == RUBY_NEW_WORD => {
			let class_name = class.drop_meta().name();
			crate::normalize::set_position_of(&class);
			crate::diagnostic::note_alias(&format!("{class_name}.{RUBY_NEW_WORD}"), &class_name);
			let arguments = match member.drop_meta().clone() {
				Node::List(items, _, _) => items[1..].iter().cloned().map(|argument| constructed_by_new(argument, classes)).collect(),
				_ => vec![],
			};
			Node::List([vec![*class], arguments].concat(), Bracket::Round, Separator::None)
		}
		other => other.map_children(|child| constructed_by_new(child, classes)),
	}
}

pub(super) fn is_witness_method(name: &str) -> bool {
	WITNESS_METHODS.iter().any(|(witness, _)| *witness == name)
}

/// A class's `text()`, `equals(o)` and `compare(o)` (Java's `toString()`, `compareTo(o)`, Python's `__str__`, `__eq__`…,
/// with a note) as the witnesses of its text, equality and order: the other instance typed by the class,
/// `equals(o:P)`, so the method is `equals(self:P, o:P)`, which traits makes equals·P
pub(super) fn with_witness_methods(node: Node) -> Node {
	match node {
		Node::Type { name, body } => {
			let class = name.drop_meta().name();
			let items: Vec<Node> = class_items(&body);
			if !items.iter().filter_map(method_parts).any(|(method, _, _)| WITNESS_METHODS.iter().any(|(witness, aliases)| *witness == method || aliases.contains(&method.as_str()))) {
				return Node::Type { name, body };
			}
			let items = items.into_iter().map(|item| witness_method(item, &class)).collect();
			Node::Type { name, body: Box::new(Node::List(items, Bracket::Curly, Separator::Semicolon)) }
		}
		other => other.map_children(with_witness_methods),
	}
}

pub(super) fn witness_method(item: Node, class: &str) -> Node {
	let Some((method, parameters, _)) = method_parts(&item) else { return item };
	let Some((witness, _)) = WITNESS_METHODS.iter().find(|(witness, aliases)| *witness == method || aliases.contains(&method.as_str())) else { return item };
	if *witness != method {
		crate::diagnostic::note_alias(&method, witness);
	}
	let typed = |parameter: Node| match parameter.drop_meta() {
		Node::Symbol(_) if *witness != "text" => key(parameter, Op::Colon, symbol(class)),
		_ => parameter,
	};
	let head = call(witness, parameters.into_iter().map(typed).collect());
	match item.drop_meta().clone() {
		Node::Key(old_head, Op::Define, body) => match old_head.drop_meta() {
			Node::Key(_, Op::Colon, result) if result_type(&item).is_some() => Node::Key(Box::new(Node::Key(Box::new(head), Op::Colon, result.clone())), Op::Define, body),
			_ => Node::Key(Box::new(head), Op::Define, body),
		},
		_ => item,
	}
}
