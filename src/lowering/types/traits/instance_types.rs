//! Static instance types: which variables and expressions hold instances of which declared type
use super::*;

/// What the compiler knows about a value of a declared type
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
	Instance(String),
	ListOf(String),
}

/// The shapes of the program's variables: a variable only ever bound to values of one shape has it (flow-insensitive,
/// like library_words' object variables); parameters `p:person` are instances
pub struct InstanceTypes {
	variables: HashMap<String, Option<Shape>>,
	/// The shapes user functions return, from their bodies: `make(t) := pdf(t)` returns a pdf
	results: HashMap<String, Option<Shape>>,
	pub(super) registry: TypeRegistry,
	/// The last inference round: a value whose shape is still unknown is unknown for good
	final_round: bool,
	/// The parameters of the definitions being rewritten, innermost last (in_definition)
	parameters: RefCell<Vec<HashMap<String, Option<Shape>>>>,
}

/// Fixpoint rounds over the assignments, at most: a variable can be assigned from another one defined later in the
/// text, a result from a chain of calls (`room = make()` of `make() := bus() |> delay(…)`, four deep)
pub(super) const INFERENCE_ROUNDS: usize = 8;

impl InstanceTypes {
	pub fn of(node: &Node) -> Self {
		let mut registry = TypeRegistry::new();
		collect_all_types(&mut registry, node);
		let mut types = InstanceTypes {
			variables: HashMap::new(),
			results: HashMap::new(),
			registry,
			final_round: false,
			parameters: RefCell::default(),
		};
		if types.registry.types().is_empty() {
			return types;
		}
		for _ in 1..INFERENCE_ROUNDS {
			if !types.inferred_round(node) {
				break;
			}
		}
		types.final_round = true;
		types.inferred_round(node);
		types
	}

	/// One round over the program from the shapes of the round before: whether it learned anything
	fn inferred_round(&mut self, node: &Node) -> bool {
		let (mut variables, mut results) = (HashMap::new(), HashMap::new());
		self.collect(node, &mut variables, &mut results);
		let changed = variables != self.variables || results != self.results;
		(self.variables, self.results) = (variables, results);
		changed
	}

	/// A value of an unknown shape adds nothing (it may be an instance the next round recognizes) until the final round,
	/// where it is unknown for good: like a plain value or another shape it makes the variable shapeless (None), so
	/// `s = square(1); s = xs#2` dispatches at run time
	pub(super) fn bind(
		variables: &mut HashMap<String, Option<Shape>>,
		name: &str,
		shape: Option<Shape>,
		is_plain: bool,
	) {
		let agreed = match (variables.get(name), shape) {
			(_, None) if is_plain => None,
			(None, None) => return,
			(Some(earlier), None) => earlier.clone(),
			(None, Some(shape)) => Some(shape),
			(Some(earlier), Some(shape)) => earlier.clone().filter(|earlier| *earlier == shape),
		};
		variables.insert(name.to_string(), agreed);
	}

	pub(super) fn collect(
		&self,
		node: &Node,
		variables: &mut HashMap<String, Option<Shape>>,
		results: &mut HashMap<String, Option<Shape>>,
	) {
		match node.drop_meta() {
			Node::Key(target, Op::Assign | Op::Define, value) => {
				let is_plain = crate::min_max::is_plain(value) || value.is_text_literal();
				match (target.drop_meta(), definition_head(target)) {
					(Node::Symbol(name), _) => {
						// `p = field_with(p, …)` (a field assignment) keeps the shape of p
						let keeps_shape = is_field_update(name, value);
						let unknown_for_good = self.final_round && !keeps_shape && self.shape(value).is_none();
						Self::bind(variables, name, self.shape(value), is_plain || unknown_for_good)
					}
					// `xs: [C] = …` of a declared list type (before the instance case: the name of `[C]` is C)
					(Node::Key(name, Op::Colon, type_node), _) if self.declared_element(type_node).is_some() => {
						Self::bind(variables, &name.name(), self.declared_element(type_node).map(Shape::ListOf), false)
					}
					// `d:docx = …` of a declared type
					(Node::Key(name, Op::Colon, type_node), _)
						if self.registry.get_by_name(&type_node.drop_meta().name()).is_some() =>
					{
						Self::bind(variables, &name.name(), Some(Shape::Instance(type_node.drop_meta().name())), false)
					}
					(_, Some(head)) => Self::bind(results, &head[0].name(), self.shape(value), is_plain),
					// `bus() := Bus{…}`: a function without parameters returns an instance too
					(head, None) if is_parameterless_head(target) => {
						Self::bind(results, &crate::node::leading_name(head), self.shape(value), is_plain)
					}
					_ => {}
				}
				self.collect(value, variables, results);
			}
			Node::Key(left, _, right) => {
				self.collect(left, variables, results);
				self.collect(right, variables, results);
			}
			Node::List(items, _, _) => {
				// `for x in xs`: x is an item of xs; of other items too it is shapeless (filters share one element name)
				if let [keyword, variable, in_word, list, ..] = items.as_slice() {
					if keyword.name() == FOR_WORD && in_word.name() == IN_WORD {
						if let Node::Symbol(name) = variable.drop_meta() {
							match self.shape(list) {
								Some(Shape::ListOf(type_name)) => {
									Self::bind(variables, name, Some(Shape::Instance(type_name)), false)
								}
								_ => Self::bind(variables, name, None, self.final_round),
							}
						}
					}
				}
				items.iter().for_each(|item| self.collect(item, variables, results))
			}
			_ => {}
		}
	}

	/// `rewrite` of the body of the definition `head := …` while its parameters hide the variables of their names: `q`
	/// of `f(q:V)` is an instance of the declared type V, an untyped one of no known shape (card param-text)
	pub fn in_definition<T>(&self, head: &Node, rewrite: impl FnOnce() -> T) -> T {
		let Some(items) = definition_head(head) else {
			return rewrite();
		};
		let shape_of = |parameter: &Node| match parameter.drop_meta() {
			Node::Key(name, Op::Colon, type_node) => {
				let type_name = type_node.drop_meta().name();
				Some((name.name(), self.registry.get_by_name(&type_name).map(|_| Shape::Instance(type_name))))
			}
			Node::Key(name, Op::Assign, _) => Some((name.name(), None)),
			Node::Symbol(name) => Some((name.clone(), None)),
			_ => None,
		};
		let scope = items[1..].iter().filter_map(shape_of).collect();
		self.parameters.borrow_mut().push(scope);
		let rewritten = rewrite();
		self.parameters.borrow_mut().pop();
		rewritten
	}

	/// Is `name` a parameter of a definition being rewritten
	pub(super) fn is_parameter(&self, name: &str) -> bool {
		self.parameters.borrow().iter().any(|scope| scope.contains_key(name))
	}

	/// The declared type of the elements of `[C]`
	pub(super) fn declared_element(&self, type_node: &Node) -> Option<String> {
		let Node::List(items, Bracket::Square, _) = type_node.drop_meta() else {
			return None;
		};
		let [element] = items.as_slice() else { return None };
		let name = element.drop_meta().name();
		self.registry.get_by_name(&name).is_some().then_some(name)
	}

	/// Is `name` a field of any declared type: `v.x` then reads the field at run time, whatever v holds
	pub fn is_declared_field(&self, name: &str) -> bool {
		self.registry.types().iter().any(|type_def| type_def.fields.iter().any(|field| field.name == name))
	}

	/// Does `list` hold instances of a declared type with the field `name`: `people.name` then reads it of each
	pub fn is_element_field(&self, list: &Node, name: &str) -> bool {
		let Some(Shape::ListOf(type_name)) = self.shape(list) else {
			return false;
		};
		self.has_field(&type_name, name)
	}

	/// Is `node` known to be an instance of a declared type with the field `name`
	pub fn is_instance_field(&self, node: &Node, name: &str) -> bool {
		matches!(self.shape(node), Some(Shape::Instance(type_name)) if self.has_field(&type_name, name))
	}

	fn has_field(&self, type_name: &str, name: &str) -> bool {
		self.registry
			.get_by_name(type_name)
			.is_some_and(|type_def| type_def.fields.iter().any(|field| field.name == name))
	}

	/// The shape of an expression, when it is known to hold instances of one declared type
	pub fn shape(&self, node: &Node) -> Option<Shape> {
		if let Some((name, _)) = instance_parts(node) {
			return Some(Shape::Instance(name.name()));
		}
		if let Some(type_name) = typed_as(node) {
			return Some(Shape::Instance(type_name));
		}
		match node.drop_meta() {
			Node::Symbol(name) => match self.parameters.borrow().iter().rev().find_map(|scope| scope.get(name)) {
				Some(parameter) => parameter.clone(),
				None => self.variables.get(name).cloned().flatten(),
			},
			Node::List(items, Bracket::Square, _) if !items.is_empty() => {
				let first = self.shape(&items[0])?;
				let Shape::Instance(type_name) = &first else {
					return None;
				};
				items
					.iter()
					.all(|item| self.shape(item).as_ref() == Some(&first))
					.then(|| Shape::ListOf(type_name.clone()))
			}
			// `xs#1`, `xs[0]`: an item of a list of instances
			Node::Key(list, Op::Hash, _) => match self.shape(list)? {
				Shape::ListOf(type_name) => Some(Shape::Instance(type_name)),
				Shape::Instance(_) => None,
			},
			// `xs.sort`
			Node::Key(list, Op::Dot, word) if word.name() == SORT_WORD => {
				self.shape(list).filter(|shape| matches!(shape, Shape::ListOf(_)))
			}
			// `q.to("km")` of an instance: what the method returns (card instance-result)
			Node::Key(receiver, Op::Dot, call) if matches!(self.shape(receiver), Some(Shape::Instance(_))) => {
				let method = crate::node::leading_name(call);
				self.results.get(&method).cloned().flatten()
			}
			// `c ? a : b`, `if c then a else b`: the shape both branches share
			_ if crate::lowering::nodes::branches(node).is_some() => {
				let (_, then, otherwise) = crate::lowering::nodes::branches(node)?;
				self.branches_shape(then, otherwise)
			}
			Node::List(items, _, separator) => match items.as_slice() {
				// `sort xs`, `sort(xs)`
				[word, list] if word.name() == SORT_WORD => {
					self.shape(list).filter(|shape| matches!(shape, Shape::ListOf(_)))
				}
				// `P·init(P{…}, 3)`: a construction passed through its class's constructor (class_methods::constructor_name)
				[function, instance, ..] if matches!(function.drop_meta(), Node::Symbol(name) if crate::class_methods::constructor_name(&instance_parts(instance).map(|(class, _)| class.name()).unwrap_or_default()) == *name) => {
					self.shape(instance)
				}
				// `Bus(id, chain)`: a class called constructs its instance (before type_constructor writes it `Bus{…}`)
				[function, _, ..] if matches!(function.drop_meta(), Node::Symbol(name) if self.registry.get_by_name(name).is_some()) => {
					Some(Shape::Instance(function.name()))
				}
				// `make(t)`: what the user function returns
				[function, ..] if matches!(function.drop_meta(), Node::Symbol(name) if self.results.contains_key(name)) => {
					self.results[&function.name()].clone()
				}
				// `(x)`, and `(t = a; value)` as min_max binds its operands; another call `f(s)` is not its argument
				[single] => self.shape(single),
				[.., last] if matches!(separator, Separator::Semicolon | Separator::Newline) => self.shape(last),
				_ => None,
			},
			_ => None,
		}
	}

	pub(super) fn branches_shape(&self, then: &Node, otherwise: &Node) -> Option<Shape> {
		match (diverges(then), diverges(otherwise)) {
			(true, _) => self.shape(otherwise),
			(_, true) => self.shape(then),
			_ => self.shape(then).filter(|shape| self.shape(otherwise).as_ref() == Some(shape)),
		}
	}

	/// A curly list of the declared fields of the instance a node holds: library_words reads fields of it like of an
	/// object literal
	pub fn fields_template(&self, node: &Node) -> Option<Node> {
		let Some(Shape::Instance(type_name)) = self.shape(node) else {
			return None;
		};
		let type_def = self.registry.get_by_name(&type_name)?;
		let entries = type_def.fields.iter().map(|field| key(Node::Symbol(field.name.clone()), Op::Colon, Node::Empty));
		Some(Node::List(entries.collect(), Bracket::Curly, Separator::Space))
	}
}

/// A branch that never gives a value: the error of `max` over an empty list
pub(super) fn diverges(branch: &Node) -> bool {
	matches!(branch.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|head| head.name() == crate::min_max::EMPTY_EXTREMUM_CALL))
}
