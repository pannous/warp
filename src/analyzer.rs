use crate::extensions::numbers::Number;
use crate::node::{Local, Node, Op};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use wasm_ast::Function;

pub static FUNCTIONS: Lazy<HashMap<String, Function>> = Lazy::new(|| HashMap::new());

/// Check if a node requires float operations (type upgrading)
pub fn needs_float(node: &Node, scope: &Scope) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Number(Number::Float(_)) => true,
		Node::Number(Number::Quotient(_, _)) => true,
		Node::Number(Number::Complex(_, _)) => true,
		Node::Key(left, op, right) if op.is_arithmetic() => {
			needs_float(left, scope) || needs_float(right, scope)
		}
		Node::Key(_left, Op::Define | Op::Assign, right) => {
			needs_float(right, scope)
		}
		Node::Key(left, op, right) if op.is_compound_assign() => {
			needs_float(left, scope) || needs_float(right, scope)
		}
		// Handle global:Key(name, =, value) -> check value
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					return needs_float(right, scope);
				}
			}
			needs_float(left, scope) || needs_float(right, scope)
		}
		Node::List(items, _, _) => items.iter().any(|item| needs_float(item, scope)),
		Node::Symbol(name) => {
			if let Some(local) = scope.lookup(name) {
				local.is_float
			} else {
				false
			}
		}
		_ => false,
	}
}

/// Check if an expression returns a Node reference (not a primitive)
/// Used to determine if a variable should be ref-typed
fn returns_node_ref(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		// fetch returns a Node (Text)
		Node::List(items, _, _) if items.len() == 2 => {
			if let Node::Symbol(s) = items[0].drop_meta() {
				return s == "fetch";
			}
			false
		}
		// Text and Symbol literals return Node refs
		Node::Text(_) | Node::Symbol(_) => true,
		// Numbers return primitives (i64 or f64)
		Node::Number(_) => false,
		// Arithmetic returns primitives
		Node::Key(_, op, _) if op.is_arithmetic() => false,
		// Other cases default to ref for safety
		_ => false,
	}
}

/// Collect variables defined in node and populate scope
/// Returns count of temp locals needed (e.g., for while loops)
pub fn collect_variables(node: &Node, scope: &mut Scope) -> u32 {
	collect_variables_inner(node, scope, false, false)
}

fn collect_variables_inner(node: &Node, scope: &mut Scope, skip_first_assign: bool, in_structure: bool) -> u32 {
	let node = node.drop_meta();
	match node {
		// Global declarations: global:Key(name, =, value) - don't create local
		// Tag structures: html:body - body is structure context (attributes, not variables)
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					// Don't define local for global variable
					// But still count any variables in the value expression
					return collect_variables_inner(right, scope, true, false);
				}
				// Symbol:body is a tag/structure - right side is structure context
				// Inside structures, Op::Assign is attribute, not variable
				return collect_variables_inner(left, scope, false, in_structure)
					+ collect_variables_inner(right, scope, false, true);
			}
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		// Define (:=) always creates a variable, Assign (=) only outside structure context
		Node::Key(left, Op::Define, right) => {
			if !skip_first_assign {
				if let Node::Symbol(name) = left.drop_meta() {
					let is_float = needs_float(right, scope);
					let is_ref = returns_node_ref(right);
					scope.define(name.clone(), None, is_float, is_ref);
				}
			}
			collect_variables_inner(right, scope, false, in_structure)
		}
		// Assign creates variables only at top level (not inside structures)
		Node::Key(left, Op::Assign, right) => {
			if !skip_first_assign && !in_structure {
				if let Node::Symbol(name) = left.drop_meta() {
					let is_float = needs_float(right, scope);
					let is_ref = returns_node_ref(right);
					scope.define(name.clone(), None, is_float, is_ref);
				}
			}
			collect_variables_inner(right, scope, false, in_structure)
		}
		// Compound assignments don't create new variables
		Node::Key(left, op, right) if op.is_compound_assign() => {
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, Op::Do, right) => {
			// While loop needs a temp local for result
			1 + collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::Key(left, _, right) => {
			collect_variables_inner(left, scope, false, in_structure) + collect_variables_inner(right, scope, false, in_structure)
		}
		Node::List(items, _, _) => {
			items.iter().map(|item| collect_variables_inner(item, scope, false, in_structure)).sum()
		}
		_ => 0,
	}
}

/// Scope for tracking variable bindings
#[derive(Clone, Debug, Default)]
pub struct Scope {
	pub locals: HashMap<String, Local>,
	pub types: HashMap<String, Node>,  // User-defined types
	pub parent: Option<Box<Scope>>,
}

impl Scope {
	pub fn new() -> Self {
		Scope::default()
	}

	pub fn child(&self) -> Self {
		Scope {
			locals: HashMap::new(),
			types: HashMap::new(),
			parent: Some(Box::new(self.clone())),
		}
	}

	/// Look up a variable by name, checking parent scopes
	pub fn lookup(&self, name: &str) -> Option<&Local> {
		self.locals.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup(name)))
	}

	/// Define a new variable in current scope
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, is_float: bool, is_ref: bool) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			position,
			is_param: false,
			is_float,
			is_ref,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Define a function parameter
	pub fn define_param(&mut self, name: String, type_node: Option<Box<Node>>) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			position,
			is_param: true,
			is_float: false,
			is_ref: false,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Define a type in current scope
	pub fn define_type(&mut self, name: String, def: Node) {
		self.types.insert(name, def);
	}

	/// Look up a type by name
	pub fn lookup_type(&self, name: &str) -> Option<&Node> {
		self.types.get(name).or_else(||
			self.parent.as_ref().and_then(|p| p.lookup_type(name)))
	}

	/// Get total number of locals (for WASM local declaration)
	pub fn local_count(&self) -> u32 {
		self.locals.len() as u32
	}
}

pub fn analyze(raw: Node) -> Node {
	raw // Pass-through for now; analysis happens in emitter
}
