//! Page ids as names (card page-ids, user 2026-10-15, notes/page_elements.md): an id in the program's own markup
//! (`input{ id: "who" }`) is a name, plain `who` the element (`document.getElementById("who")`) and `$who` its value:
//! the text of an input, textarea or select, the number of a number input, yes/no of a checkbox, else the text content;
//! `$who = "Ann"` writes it. `element(x)` finds any other id; `document` and `window` are plain names. Each brings
//! its `use js` along, which foreign_modules then lowers, so `who.focuss()` is a compile-time did-you-mean.
//! An id clashing with a variable or function of the program, or with a word the program uses plainly, is a
//! compile-time error naming both. The forms of card browser-interaction (`$who.value`, `$who.focus()`,
//! `$document.title`, `$xyz.innerHTML`) keep working one release with a hint naming the new form.
//! Untouched: texts (markup text, single-quoted strings), `$0` (a closure's first parameter), `$a` naming an
//! enclosing node of a data literal (references.rs), a bare `$main` of no id, and everything inside `data …`; a
//! shader's holes, a route's form fields and a code pattern's holes are taken before this pass or sealed.

use crate::errors::diagnostic::Diagnostic;
use crate::lowering::blocks::DATA_WORD;
use crate::modules::needed_definitions::with_statements_first;
use crate::node::Node;
use crate::operators::Op;
use std::collections::{BTreeMap, BTreeSet, HashSet};

const SIGIL: char = '$';
/// The global the elements are found in, and how
const DOCUMENT: &str = "document";
const FIND_BY_ID: &str = "getElementById";
/// The browser's globals a program names without `use js`
const PLAIN_GLOBALS: [&str; 2] = [DOCUMENT, "window"];
/// `element("user-name")`: the element of a computed id or one the program does not create
const ELEMENT_WORD: &str = "element";
/// JavaScript's own globals, which WebIDL (the browser's) does not declare
const LANGUAGE_GLOBALS: [&str; 14] = [
	"Math", "JSON", "Date", "Intl", "Reflect", "Object", "Array", "Map", "Set", "Number", "String", "Promise",
	"RegExp", "BigInt",
];
const ID_ATTRIBUTE: &str = "id";
const TYPE_ATTRIBUTE: &str = "type";
/// The elements whose value is what was typed or chosen in them
const FIELD_TAGS: [&str; 3] = ["input", "textarea", "select"];
const TICKED_TYPES: [&str; 2] = ["checkbox", "radio"];
const NUMBER_TYPES: [&str; 2] = ["number", "range"];
const VALUE_MEMBER: &str = "value";
const TICKED_MEMBER: &str = "checked";
const NUMBER_MEMBER: &str = "valueAsNumber";
const TEXT_MEMBER: &str = "textContent";
/// What a clash of an id with a variable offers, and the first `$who` of a field hints at: `input{ bind: who }`
const BIND_ATTRIBUTE: &str = "bind";
/// jQuery's `$('#who')`
const JQUERY_WORD: &str = "$";
const ID_SELECTOR: char = '#';
const OLD_FORM_REASON: &str = "page ids are names: `who` is the element, `$who` its value (card page-ids)";

/// An element of the program's markup that has an id
struct PageElement {
	tag: String,
	input_type: Option<String>,
	/// `input{ id: "who" }` as written, for the clash error
	written: String,
}

impl PageElement {
	/// An input, textarea or select: `bind:` keeps a variable in sync with it
	fn is_field(&self) -> bool {
		FIELD_TAGS.contains(&self.tag.as_str())
	}

	/// The member `$id` reads and writes
	fn value_member(&self) -> &'static str {
		let input_type = self.input_type.as_deref().unwrap_or_default();
		match self.tag.as_str() {
			"input" if TICKED_TYPES.contains(&input_type) => TICKED_MEMBER,
			"input" if NUMBER_TYPES.contains(&input_type) => NUMBER_MEMBER,
			tag if FIELD_TAGS.contains(&tag) => VALUE_MEMBER,
			_ => TEXT_MEMBER,
		}
	}
}

struct PageNames {
	elements: BTreeMap<String, PageElement>,
	defined: HashSet<String>,
	/// The globals whose `use js` the program needs
	globals: BTreeSet<String>,
	error: Option<Node>,
	/// The last name met with its position, where a clash with a word or the bind hint is reported
	last_name: Option<Node>,
	bind_hinted: bool,
}

pub fn lower(program: Node) -> Node {
	let defined = crate::library_words::defined_names(&program);
	let elements = markup_elements(&program, &defined);
	if let Some(error) = clash_with_definition(&program, &elements) {
		return error;
	}
	let mut names =
		PageNames { elements, defined, globals: BTreeSet::new(), error: None, last_name: None, bind_hinted: false };
	let program = names.rewritten(program, &mut Vec::new());
	if let Some(error) = names.error {
		return error;
	}
	let uses = names.globals.iter().map(|global| crate::warp_parser::parse(&format!("use js {global}"))).collect();
	with_statements_first(program, uses)
}

/// The elements of the markup by their id: `input{ id: "who" }`, also `input{ id: who }` of a name the program does
/// not define (a defined one computes the id: `element(name)` finds it)
fn markup_elements(program: &Node, defined: &HashSet<String>) -> BTreeMap<String, PageElement> {
	let mut elements = BTreeMap::new();
	program.visit(&mut |node| {
		let Node::Key(tag, Op::Colon, body) = node else {
			return;
		};
		let (Node::Symbol(tag), Node::List(attributes, crate::node::Bracket::Curly, _)) =
			(tag.drop_meta(), body.drop_meta())
		else {
			return;
		};
		if !crate::web::markup::is_element_tag(tag) {
			return;
		}
		let attribute = |name: &str| attributes.iter().find_map(|item| attribute_value(item, name, defined));
		if let Some(id) = attribute(ID_ATTRIBUTE) {
			let input_type = attribute(TYPE_ATTRIBUTE);
			let written = format!("{tag}{{ {ID_ATTRIBUTE}: {} }}", Node::Text(id.clone()).serialize());
			elements.insert(id, PageElement { tag: tag.clone(), input_type, written });
		}
	});
	elements
}

/// The literal value of the attribute `name: "value"` (or `name: value`)
fn attribute_value(item: &Node, name: &str, defined: &HashSet<String>) -> Option<String> {
	let Node::Key(key, Op::Colon, value) = item.drop_meta() else {
		return None;
	};
	if !key.drop_meta().is_symbol(name) {
		return None;
	}
	match value.drop_meta() {
		Node::Text(text) => Some(text.clone()),
		Node::Symbol(word) if !defined.contains(word) => Some(word.clone()),
		_ => None,
	}
}

/// An id that is also a variable or function of the program: the error naming both; a variable of a field's id
/// is what `bind:` is for (`input{ id: "name" }` + `name = $name`, the most common clash)
fn clash_with_definition(program: &Node, elements: &BTreeMap<String, PageElement>) -> Option<Node> {
	let functions = crate::analyzer::function_definitions(program).user_functions;
	let assigned: HashSet<String> = program.assigned_names(|op| matches!(op, Op::Assign | Op::Define));
	let (id, element) = elements.iter().find(|(id, _)| functions.contains_key(*id) || assigned.contains(*id))?;
	let error = match functions.contains_key(id) {
		true => clash_error(program, id, element, &format!("the function {id}"), false),
		false => clash_error(&assignment_of(program, id), id, element, &format!("the variable {id}"), true),
	};
	Some(error.into_error())
}

/// "name is two things here: the page element of id name (`input{ id: "name" }`) and the variable name", with the
/// way out: `bind:` for a variable of a field, else a rename
fn clash_error(at: &Node, id: &str, element: &PageElement, other: &str, is_variable: bool) -> Diagnostic {
	let way_out = match is_variable && element.is_field() {
		true => format!(
			"To keep a variable in sync with the field, write `{}{{ {BIND_ATTRIBUTE}: {id} }}` and drop the id",
			element.tag
		),
		false => format!("Rename one of them; `element(\"{id}\")` finds the element by its id"),
	};
	Diagnostic::at(
		at,
		format!("{id} is two things here: the page element of id {id} (`{}`) and {other}. {way_out}", element.written),
	)
}

/// The first `id = …` of the program, with its position
fn assignment_of(program: &Node, id: &str) -> Node {
	let mut found = None;
	program.visit(&mut |node| {
		let is_assignment = matches!(node.drop_meta(), Node::Key(target, Op::Assign | Op::Define, _) if target.drop_meta().is_symbol(id));
		if found.is_none() && is_assignment {
			found = Some(node.clone());
		}
	});
	found.unwrap_or_else(|| program.clone())
}

impl PageNames {
	/// `enclosing`: the names of the data literal's nodes around the node (`a{ c{ parent=$a } }`)
	fn rewritten(&mut self, node: Node, enclosing: &mut Vec<String>) -> Node {
		if self.error.is_some() {
			return node;
		}
		match node {
			// a code pattern's holes (`match c { data($x.warp) => x }`, grouped `(data $x.warp)` there), not always sealed yet
			quoted if is_quoted(&quoted) => quoted,
			Node::Meta { .. } => {
				if matches!(node.drop_meta(), Node::Symbol(_)) {
					self.last_name = Some(node.clone());
				}
				node.under_meta(|inner| self.rewritten(inner, enclosing))
			}
			Node::Key(tag, Op::Colon, body) if is_element(&tag, &body) => {
				let body = body.under_meta(|body| match body {
					Node::List(items, bracket, separator) => {
						let items = items.into_iter().map(|item| self.element_item(item, enclosing)).collect();
						Node::List(items, bracket, separator)
					}
					other => other,
				});
				Node::Key(tag, Op::Colon, Box::new(body))
			}
			// an attribute's or data key's name is no use of the name: `title: "x"`, `on click { … }`
			Node::Key(name, Op::Colon, value) => {
				Node::Key(name, Op::Colon, Box::new(self.rewritten(*value, enclosing)))
			}
			Node::Key(receiver, Op::Dot, member) => {
				let receiver = match self.jquery(&receiver, Some(&member)) {
					Some(found) => found,
					None => match self.old_receiver(&receiver, &member, enclosing) {
						Some(element) => element,
						None => self.rewritten(*receiver, enclosing),
					},
				};
				Node::Key(Box::new(receiver), Op::Dot, Box::new(self.member_rewritten(*member, enclosing)))
			}
			Node::Key(target, op, value) if op == Op::Assign || op.is_compound_assign() => {
				let target = match self.dollar_element(&target, enclosing) {
					Some((id, member)) => self.value_of(&id, member),
					None => self.rewritten(*target, enclosing),
				};
				Node::Key(Box::new(target), op, Box::new(self.rewritten(*value, enclosing)))
			}
			Node::Symbol(name) => self.symbol(name, enclosing),
			jquery if self.jquery(&jquery, None).is_some() => self.jquery(&jquery, None).expect("checked"),
			Node::List(items, ..) if self.is_element_call(&items) => {
				let id = items.into_iter().nth(1).expect("element(x) has its id");
				let id = self.rewritten(id, enclosing);
				self.globals.insert(DOCUMENT.to_string());
				element_of(id)
			}
			other => {
				let named = crate::lowering::references::named_body(&other).map(|(name, _)| name);
				enclosing.extend(named.clone());
				let other = other.map_children(|child| self.rewritten(child, enclosing));
				if named.is_some() {
					enclosing.pop();
				}
				other
			}
		}
	}

	/// An item of an element: an attribute's bare value names, it uses no name (`label{ for: email }`, the id email)
	fn element_item(&mut self, item: Node, enclosing: &mut Vec<String>) -> Node {
		let names_an_id = matches!(item.drop_meta(), Node::Key(_, Op::Colon, value)
			if matches!(value.drop_meta(), Node::Symbol(word) if self.elements.contains_key(word)));
		match names_an_id {
			true => item,
			false => self.rewritten(item, enclosing),
		}
	}

	/// jQuery's `$('#who')` (`member`: what is read of it): the element, with a hint naming `who` and `$who`; a
	/// selector other than an id is the error naming document.querySelector
	fn jquery(&mut self, node: &Node, member: Option<&Node>) -> Option<Node> {
		let Node::List(items, _, _) = node.drop_meta() else {
			return None;
		};
		let [word, selector] = items.as_slice() else {
			return None;
		};
		let Node::Text(selector) = selector.drop_meta() else {
			return None;
		};
		if !word.drop_meta().is_symbol(JQUERY_WORD) {
			return None;
		}
		let written = format!("{JQUERY_WORD}({})", Node::Text(selector.clone()).serialize());
		let id = selector.strip_prefix(ID_SELECTOR).unwrap_or(selector);
		if !is_plain_id(id) {
			let query = format!("{DOCUMENT}.querySelector({})", Node::Text(selector.clone()).serialize());
			return Some(
				Diagnostic::at(node, format!("{written} is jQuery's; warp writes `{query}` for a selector"))
					.offer("the page's selector query", written, query)
					.into_error(),
			);
		}
		crate::normalize::set_position_of(node);
		let reason = format!("no jQuery needed: ${id} is the element's value, {id} the element itself");
		match self.elements.get(id) {
			Some(element) if member.is_some_and(|member| member.drop_meta().is_symbol(element.value_member())) => {
				let read = format!("{written}.{}", element.value_member());
				crate::normalize::advise(&read, &format!("${id}"), &reason, None);
			}
			Some(_) => crate::normalize::advise(&written, id, &reason, None),
			None => {
				let reason = format!("no jQuery needed: element(\"{id}\") finds an element the program doesn't create");
				crate::normalize::advise(&written, &format!("{ELEMENT_WORD}(\"{id}\")"), &reason, None);
			}
		}
		self.globals.insert(DOCUMENT.to_string());
		Some(element_of_id(id))
	}

	/// A member's name stays (`x.who` is no element); a method call's arguments are code
	fn member_rewritten(&mut self, member: Node, enclosing: &mut Vec<String>) -> Node {
		match member {
			Node::Symbol(_) => member,
			Node::Meta { .. } => member.under_meta(|inner| self.member_rewritten(inner, enclosing)),
			Node::List(items, bracket, separator)
				if items.first().is_some_and(|first| first.drop_meta().symbol_name().is_some()) =>
			{
				let mut items = items.into_iter();
				let name = items.next().into_iter();
				let arguments: Vec<Node> = items.map(|argument| self.rewritten(argument, enclosing)).collect();
				Node::List(name.chain(arguments).collect(), bracket, separator)
			}
			other => self.rewritten(other, enclosing),
		}
	}

	/// A name in code: an id is its element, `$id` its value, document and window the page's
	fn symbol(&mut self, name: String, enclosing: &[String]) -> Node {
		if let Some((id, member)) = self.dollar_element(&Node::Symbol(name.clone()), enclosing) {
			self.hint_bind(&id);
			return self.value_of(&id, member);
		}
		if self.elements.contains_key(&name) {
			if is_word(&name) {
				let at = self.last_name.clone().unwrap_or_else(|| Node::Symbol(name.clone()));
				let error = clash_error(&at, &name, &self.elements[&name], &format!("the word {name}"), false);
				self.error = Some(error.into_error());
				return Node::Symbol(name);
			}
			self.globals.insert(DOCUMENT.to_string());
			return element_of_id(&name);
		}
		if PLAIN_GLOBALS.contains(&name.as_str()) && !self.defined.contains(&name) {
			self.globals.insert(name.clone());
		}
		Node::Symbol(name)
	}

	/// Once per program, at the first `$who` read of a field: `bind:` keeps a variable in sync instead
	fn hint_bind(&mut self, id: &str) {
		if self.bind_hinted || !self.elements[id].is_field() {
			return;
		}
		self.bind_hinted = true;
		if let Some(at) = &self.last_name {
			crate::normalize::set_position_of(at);
		}
		let tag = &self.elements[id].tag;
		let reason =
			format!("${id} reads the field's value: {tag}{{ {BIND_ATTRIBUTE}: {id} }} keeps a variable {id} in sync");
		crate::normalize::advise(&format!("${id}"), &format!("{tag}{{ {BIND_ATTRIBUTE}: {id} }}"), &reason, None);
	}

	/// `$who` of an id of the markup: the id and the member of its value
	fn dollar_element(&self, node: &Node, enclosing: &[String]) -> Option<(String, &'static str)> {
		let Node::Symbol(name) = node.drop_meta() else {
			return None;
		};
		let id = name.strip_prefix(SIGIL)?;
		if enclosing.iter().any(|node| node == id) {
			return None;
		}
		self.elements.get(id).map(|element| (id.to_string(), element.value_member()))
	}

	/// `document.getElementById("who").value`, the member the element's kind keeps its value in
	fn value_of(&mut self, id: &str, member: &str) -> Node {
		self.globals.insert(DOCUMENT.to_string());
		Node::Key(Box::new(element_of_id(id)), Op::Dot, Box::new(Node::Symbol(member.to_string())))
	}

	/// `element(x)` of a program that defines no element
	fn is_element_call(&self, items: &[Node]) -> bool {
		matches!(items, [word, _] if word.drop_meta().is_symbol(ELEMENT_WORD)) && !self.defined.contains(ELEMENT_WORD)
	}

	/// The receivers of card browser-interaction, still read for one release with a hint naming the new form:
	/// `$who.value` → `$who`, `$who.focus()` → `who.focus()`, `$document.title` → `document.title`, `$xyz.x` →
	/// `element("xyz").x`
	fn old_receiver(&mut self, receiver: &Node, member: &Node, enclosing: &[String]) -> Option<Node> {
		let written = receiver.drop_meta().symbol_name()?;
		let name = browser_name(written).filter(|name| !enclosing.iter().any(|node| node == name))?;
		crate::normalize::set_position_of(receiver);
		if is_global(name) {
			crate::normalize::hint(written, name, OLD_FORM_REASON);
			self.globals.insert(name.to_string());
			return Some(Node::Symbol(name.to_string()));
		}
		match self.elements.get(name) {
			Some(element) if member.drop_meta().is_symbol(element.value_member()) => {
				let read = format!("{written}.{}", element.value_member());
				crate::normalize::hint(&read, written, OLD_FORM_REASON);
			}
			Some(_) => crate::normalize::hint(written, name, OLD_FORM_REASON),
			None => {
				let call = format!("{ELEMENT_WORD}({})", Node::Text(name.to_string()).serialize());
				crate::normalize::hint(written, &call, OLD_FORM_REASON);
			}
		}
		self.globals.insert(DOCUMENT.to_string());
		Some(element_of_id(name))
	}
}

/// `input{ … }`
fn is_element(tag: &Node, body: &Node) -> bool {
	matches!((tag.drop_meta(), body.drop_meta()), (Node::Symbol(tag), Node::List(_, crate::node::Bracket::Curly, _)) if crate::web::markup::is_element_tag(tag))
}

/// `who`, `user-name`: an id a name can be, no CSS selector
fn is_plain_id(id: &str) -> bool {
	id.starts_with(|first: char| first.is_alphabetic() || first == '_')
		&& id.chars().all(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-')
}

/// `data …`, also grouped as a code pattern is (`(data $x.warp)`)
fn is_quoted(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().is_symbol(DATA_WORD)))
}

/// `$xyz` → xyz; none for `$0` or a name without the sigil
fn browser_name(name: &str) -> Option<&str> {
	name.strip_prefix(SIGIL).filter(|rest| rest.starts_with(|first: char| first.is_alphabetic() || first == '_'))
}

/// A global of JavaScript or of the browser (WebIDL), also one only a page has (document)
fn is_global(name: &str) -> bool {
	LANGUAGE_GLOBALS.contains(&name) || !matches!(crate::formats::web_idl::global_interface(name), Ok(None))
}

/// A word every program has: a library function or a type
fn is_word(name: &str) -> bool {
	crate::library_words::is_standard_function(name) || crate::type_tests::is_builtin_type_word(name)
}

/// `document.getElementById("xyz")`
fn element_of_id(id: &str) -> Node {
	element_of(Node::Text(id.to_string()))
}

/// `document.getElementById(id)`, shaped as the parser reads it
fn element_of(id: Node) -> Node {
	let template = crate::warp_parser::parse(&format!("{DOCUMENT}.{FIND_BY_ID}(0)"));
	let Node::Key(document, Op::Dot, find) = template.drop_meta() else {
		unreachable!("a member call parses as a member");
	};
	let Node::List(items, bracket, separator) = find.drop_meta() else {
		unreachable!("a call parses as a list");
	};
	let find = Node::List(vec![items[0].clone(), id], bracket.clone(), separator.clone());
	Node::Key(document.clone(), Op::Dot, Box::new(find))
}
