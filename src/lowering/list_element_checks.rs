//! A declared list holds only items of its element type (card list-element-types). A literal item of another type is a
//! compile-time error (analyzer check_declared_types); an item known only at run time is checked at the store:
//! `names.add(v)` of `names: texts` is `if not (v is text) { raise "…" }; names.add(v)`, a call's item is held in a
//! temporary first, and a whole list from a call (`names = f()`) or another list (`names = other + [v]`) has each item
//! checked. A list field (`b.items.add(v)`, `b.items = f()`) is checked the same way: by its class when the pass knows
//! b's class (`b = bag(…)`), else guarded by each class with a list field of that name, `if (b is bag) and not (v is text)`.
//! The check is a statement before the store, so the store keeps its form (a typed int list stays an array).

use super::nodes::key;
use crate::analyzer::{added_items, appended_items, builtin_type_kind, computed_literal_kind, declaring_name_and_type, indexed_list, list_element_type, of_type_declaration};
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use std::collections::HashMap;

const ITEM_PLACEHOLDER: &str = "checked_item_placeholder";
const LIST_PLACEHOLDER: &str = "checked_list_placeholder";
const INSTANCE_PLACEHOLDER: &str = "checked_instance_placeholder";
const CHECK_PLACEHOLDER: &str = "item_check_placeholder";
/// The temporaries holding an item or a list while it is checked: `checked·1`
const CHECKED_PREFIX: &str = "checked·";
/// A float or number list takes ints too, as a declared float does
const NUMBER_WORD: &str = "number";

pub fn lower(node: Node) -> Node {
	let class_fields = crate::class_methods::class_fields(&node);
	let classes: Vec<String> = class_fields.keys().cloned().collect();
	let instances = crate::class_methods::instance_classes(&node, &classes);
	ElementChecks { declared: HashMap::new(), class_fields, instances, temporaries: 0 }.lower(node)
}

struct ElementChecks {
	/// the declared type of each variable so far, as check_declared_types reads them
	declared: HashMap<String, String>,
	/// each class's fields with their type names, for `b.items.add(v)` of a field `items: texts`
	class_fields: HashMap<String, Vec<(String, String)>>,
	/// the class of each variable known to hold an instance
	instances: HashMap<String, String>,
	temporaries: usize,
}

/// The run-time test and the error message of a declared list `name: type_name`; None for a type it does not check
struct ElementTest {
	type_word: String,
	message: String,
	/// the instance `b` and the class `bag` of `b is bag`: the test applies only to an instance of that class, for a field
	/// of an instance whose class is unknown
	guard: Option<(Node, String)>,
}

impl ElementTest {
	/// `if not (item is text) { raise "…" }`, `if (b is bag) and not (item is text) { … }`: the instance is put in
	/// as a node, its name (`nested·0`) is no source text
	fn check(&self) -> Node {
		let misfit = format!("not ({ITEM_PLACEHOLDER} is {})", self.type_word);
		let failure = match &self.guard {
			Some((_, class)) => format!("({INSTANCE_PLACEHOLDER} is {class}) and {misfit}"),
			None => misfit,
		};
		let template = crate::warp_parser::parse(&format!("if {failure} {{ raise {:?} }}", self.message));
		match &self.guard {
			Some((instance, _)) => substitute(template, INSTANCE_PLACEHOLDER, instance),
			None => template,
		}
	}
}

impl ElementChecks {
	fn lower(&mut self, node: Node) -> Node {
		match node {
			// `names: list of text = f()` still arrives as the items `names:list`, `of`, `text=f()`: normalized to
			// `names:"list of text"=f()` here (as declaration_lowering would later), else type_tests reads the `text` of a
			// check `v is text` as the variable this form seems to assign
			Node::List(items, bracket, separator) => match of_type_declaration(&items, &bracket, &separator) {
				Some(normalized) => self.lower(normalized),
				None => Node::List(items, bracket, separator).map_children(|child| self.lower(child)),
			},
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let value = self.lower(*value);
				// `names#1 = v`
				if let Some(test) = indexed_list(&target).and_then(|name| self.declared_test(name)) {
					let mut checks = vec![];
					let item = self.checked_items(vec![value], &[test], &mut checks).remove(0);
					return self.with_checks(checks, Node::Key(target, op, Box::new(item)));
				}
				// `b.items = f()`
				let field_tests = self.field_tests(&target);
				if !field_tests.is_empty() {
					return self.checked_store(&field_tests, &target.clone(), *target, op, value);
				}
				let declaration = match target.drop_meta() {
					Node::Symbol(name) => self.declared.get(name).map(|type_name| (name.clone(), type_name.clone())),
					declaring => declaring_name_and_type(declaring),
				};
				let Some((name, type_name)) = declaration else { return Node::Key(target, op, Box::new(value)) };
				self.declared.insert(name.clone(), type_name.clone());
				match element_test(&name, &type_name) {
					Some(test) => self.checked_store(&[test], &Node::Symbol(name), *target, op, value),
					None => Node::Key(target, op, Box::new(value)),
				}
			}
			// `xs += ys` of a declared list stores `xs + ys`, checked as that assignment
			Node::Key(target, Op::AddAssign, value) if matches!(target.drop_meta(), Node::Symbol(name) if self.declared_test(name).is_some()) => {
				let sum = Node::Key(target.clone(), Op::Add, value);
				self.lower(Node::Key(target, Op::Assign, Box::new(sum)))
			}
			Node::Key(list, Op::Dot, call) => {
				let append = Node::Key(list, Op::Dot, call);
				if appended_items(&append).is_none() {
					return append.map_children(|child| self.lower(child));
				}
				let Node::Key(list, _, _) = append.drop_meta() else { unreachable!("an append call") };
				let tests = match list.drop_meta() {
					Node::Symbol(name) => self.declared_test(name).into_iter().collect(),
					field => self.field_tests(field),
				};
				match tests.is_empty() {
					true => append,
					false => self.checked_append(&tests, append),
				}
			}
			other => other.map_children(|child| self.lower(child)),
		}
	}

	/// `names = names + [v]`, `names = other + [v]`, `names = [v w]`, `names = f()`, into the list `stored`
	fn checked_store(&mut self, tests: &[ElementTest], stored: &Node, target: Node, op: Op, value: Node) -> Node {
		let mut checks = vec![];
		let value = match added_items(&value) {
			Some(items) => {
				let items = self.checked_items(items.to_vec(), tests, &mut checks);
				let value = with_added_items(&value, items);
				match value {
					Node::Key(list, Op::Add, appended) if !self.holds_checked_items(&list, stored) => {
						let held = self.temporary();
						checks.push(assignment(held.clone(), *list));
						checks.extend(tests.iter().map(|test| each_item_check(&held, test)));
						Node::Key(Box::new(held), Op::Add, appended)
					}
					value => value,
				}
			}
			None if matches!(value.drop_meta(), Node::Empty) || checked_at_compile_time(&value, tests) => value,
			None => {
				let list = self.temporary();
				checks.push(assignment(list.clone(), value));
				checks.extend(tests.iter().map(|test| each_item_check(&list, test)));
				list
			}
		};
		self.with_checks(checks, key(target, op, value))
	}

	/// `names.add(v)`
	fn checked_append(&mut self, tests: &[ElementTest], append: Node) -> Node {
		let Node::Key(list, Op::Dot, call) = append.drop_meta() else { return append };
		let Node::List(items, bracket, separator) = call.drop_meta() else { return append };
		let (method, arguments) = items.split_first().expect("an append call has its method");
		let mut checks = vec![];
		let arguments = self.checked_items(arguments.to_vec(), tests, &mut checks);
		let call = Node::List([vec![method.clone()], arguments].concat(), bracket.clone(), separator.clone());
		self.with_checks(checks, Node::Key(list.clone(), Op::Dot, Box::new(call)))
	}

	/// The items as stored, each run-time one checked first; a call's item is held in a temporary so it runs once
	fn checked_items(&mut self, items: Vec<Node>, tests: &[ElementTest], checks: &mut Vec<Node>) -> Vec<Node> {
		items.into_iter().map(|item| {
			if checked_at_compile_time(&item, tests) {
				return item;
			}
			let held = match item.drop_meta() {
				Node::Symbol(_) => item,
				_ => {
					let temporary = self.temporary();
					checks.push(assignment(temporary.clone(), item));
					temporary
				}
			};
			checks.extend(tests.iter().map(|test| item_check(&held, test)));
			held
		}).collect()
	}

	fn declared_test(&self, name: &str) -> Option<ElementTest> {
		element_test(name, self.declared.get(name)?)
	}

	/// The list `xs` of `xs + [v]` stored in the list `stored` holds checked items: stored itself or a list declared
	/// with the same element type
	fn holds_checked_items(&self, list: &Node, stored: &Node) -> bool {
		if list.drop_meta() == stored.drop_meta() {
			return true;
		}
		let element = |variable: &Node| match variable.drop_meta() {
			Node::Symbol(name) => self.declared.get(name).and_then(|type_name| list_element_type(type_name)),
			_ => None,
		};
		element(list).is_some_and(|element_type| element(stored) == Some(element_type))
	}

	/// The tests of a list field `b.items`: by b's class when the pass knows it, else one guarded test per class with a
	/// checked list field of that name
	fn field_tests(&self, list: &Node) -> Vec<ElementTest> {
		let Node::Key(instance, Op::Dot, field) = list.drop_meta() else { return vec![] };
		let Node::Symbol(name) = instance.drop_meta() else { return vec![] };
		let field = field.drop_meta().name();
		let test_of = |class: &String| {
			let (_, type_name) = self.class_fields.get(class)?.iter().find(|(name, _)| *name == field)?;
			element_test(&format!("{field} of {class}"), type_name)
		};
		match self.instances.get(name) {
			Some(class) => test_of(class).into_iter().collect(),
			None => self.class_fields.keys().collect::<std::collections::BTreeSet<_>>().into_iter().filter_map(|class| {
				let test = test_of(class)?;
				Some(ElementTest { guard: Some((instance.drop_meta().clone(), class.clone())), ..test })
			}).collect(),
		}
	}

	fn with_checks(&mut self, checks: Vec<Node>, store: Node) -> Node {
		if checks.is_empty() {
			return store;
		}
		Node::List([checks, vec![store]].concat(), Bracket::None, Separator::Semicolon)
	}

	fn temporary(&mut self) -> Node {
		self.temporaries += 1;
		Node::Symbol(format!("{CHECKED_PREFIX}{}", self.temporaries))
	}
}

/// A literal is checked at compile time, unless a test is guarded: the compiler doesn't know that instance's class
fn checked_at_compile_time(value: &Node, tests: &[ElementTest]) -> bool {
	computed_literal_kind(value).is_some() && tests.iter().all(|test| test.guard.is_none())
}

fn element_test(name: &str, type_name: &str) -> Option<ElementTest> {
	let element = list_element_type(type_name)?;
	let kind = builtin_type_kind(element)?;
	let type_word = if kind == Kind::Float { NUMBER_WORD.to_string() } else { element.to_string() };
	let message = format!("type mismatch: {name} is declared {type_name}, cannot hold an item that is no {element}");
	Some(ElementTest { type_word, message, guard: None })
}

/// `if not (v is text) { raise "…" }`
fn item_check(item: &Node, test: &ElementTest) -> Node {
	substitute(test.check(), ITEM_PLACEHOLDER, item)
}

/// `for item in list { if not (item is text) { raise "…" } }`
fn each_item_check(list: &Node, test: &ElementTest) -> Node {
	let template = crate::warp_parser::parse(&format!("for {ITEM_PLACEHOLDER} in {LIST_PLACEHOLDER} {{ {CHECK_PLACEHOLDER} }}"));
	let template = substitute(template, CHECK_PLACEHOLDER, &test.check());
	let item = Node::Symbol(format!("{}·item", list.name()));
	substitute(substitute(template, LIST_PLACEHOLDER, list), ITEM_PLACEHOLDER, &item)
}

fn assignment(target: Node, value: Node) -> Node {
	key(target, Op::Assign, value)
}


/// The value `[a b]` or `xs + [a b]` with its added items replaced
fn with_added_items(value: &Node, items: Vec<Node>) -> Node {
	let square = |like: &Node| match like.drop_meta() {
		Node::List(_, _, separator) => Node::List(items.clone(), Bracket::Square, separator.clone()),
		_ => Node::List(items.clone(), Bracket::Square, Separator::Space),
	};
	match value.drop_meta() {
		Node::Key(list, Op::Add, appended) => Node::Key(list.clone(), Op::Add, Box::new(square(appended))),
		_ => square(value),
	}
}
