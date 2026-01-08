use crate::node::{Local, Node, Op};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use wasm_ast::Function;

pub static FUNCTIONS: Lazy<HashMap<String, Function>> = Lazy::new(|| HashMap::new());

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
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, is_float: bool) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			position,
			is_param: false,
			is_float,
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
