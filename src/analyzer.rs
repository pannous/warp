use crate::context::{Context, UserFunctionDef};
use crate::extensions::numbers::Number;
use crate::function::{Function, FunctionRegistry, Signature};
use crate::local::Local;
use crate::node::{Bracket, Node};
use crate::normalize::hints as norm;
use crate::operators::{is_function_keyword, Op};
use crate::type_kinds::Kind;
use std::collections::HashMap;

/// Check if a node is pure data (not a statement/function call)
fn is_data_node(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False | Node::Empty => true,
		Node::Symbol(s) => !is_function_keyword(s),
		Node::List(items, _, _) => items.iter().all(is_data_node),
		Node::Key(_, Op::Colon, _) => true,  // Key-value pairs are data
		_ => false,
	}
}

/// Infer the Kind for an expression
/// Returns Int, Float, Text, etc. based on the expression's result type
pub fn infer_type(node: &Node, scope: &Scope) -> Kind {
	let node = node.drop_meta();
	match node {
		// Float literals and derived types
		Node::Number(Number::Float(_)) => Kind::Float,
		Node::Number(Number::Quotient(_, _)) => Kind::Float,
		Node::Number(Number::Complex(_, _)) => Kind::Float,
		// Integer literals
		Node::Number(_) => Kind::Int,
		// Text and char
		Node::Text(_) => Kind::Text,
		Node::Char(_) => Kind::Codepoint,
		// Symbol (identifier)
		Node::Symbol(name) => {
			if let Some(local) = scope.lookup(name) {
				local.kind
			} else {
				Kind::Symbol  // Unknown symbol defaults to Symbol
			}
		}
		// List handling: distinguish data lists from statement sequences and function calls
		Node::List(items, bracket, _) if !items.is_empty() => {
			// Check for function calls: (funcname args...) where first item is a symbol
			if items.len() >= 2 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					if s == "fetch" { return Kind::Text; }
					// FFI/builtin function calls return Int by default
					// This handles strcmp, strlen, abs, etc.
					if crate::ffi::is_ffi_function(s) {
						// Get actual return type from FFI signature if available
						if let Some(sig) = crate::ffi::get_ffi_signature(s) {
							if !sig.results.is_empty() {
								return match sig.results[0] {
									wasm_encoder::ValType::F64 | wasm_encoder::ValType::F32 => Kind::Float,
									_ => Kind::Int,
								};
							}
						}
						return Kind::Int;
					}
				}
			}
			// Function call with parentheses: assume Int result
			if *bracket == Bracket::Round && items.len() >= 2 {
				if let Node::Symbol(_) = items[0].drop_meta() {
					return Kind::Int;
				}
			}
			// Zero-arg function call: (funcname) with no args
			if *bracket == Bracket::Round && items.len() == 1 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					if crate::ffi::is_ffi_function(s) {
						if let Some(sig) = crate::ffi::get_ffi_signature(s) {
							if !sig.results.is_empty() {
								return match sig.results[0] {
									wasm_encoder::ValType::F64 | wasm_encoder::ValType::F32 => Kind::Float,
									_ => Kind::Int,
								};
							}
						}
						return Kind::Int;
					}
					// Assume zero-arg user function returns Int
					return Kind::Int;
				}
			}
			// Curly braces {} are code blocks: return type of last expression
			if *bracket == Bracket::Curly {
				if let Some(last) = items.last() {
					return infer_type(last, scope);
				} else {
					return Kind::Empty;
				}
			}
			// Data list: all items are pure data → Kind::List
			if items.iter().all(is_data_node) {
				return Kind::List;
			}
			// Statement sequence: return type of last item
			if let Some(last) = items.last() {
				infer_type(last, scope)
			} else {
				Kind::Empty
			}
		}
		// Arithmetic: upgrade to Float if either operand is Float
		Node::Key(left, op, right) if op.is_arithmetic() => {
			let left_kind = infer_type(left, scope);
			let right_kind = infer_type(right, scope);
			if left_kind == Kind::Float || right_kind == Kind::Float {
				Kind::Float
			} else {
				Kind::Int
			}
		}
		// Assignment/definition: type comes from value
		Node::Key(_left, Op::Define | Op::Assign, right) => {
			infer_type(right, scope)
		}
		// Compound assignment: upgrade if either side is Float
		Node::Key(left, op, right) if op.is_compound_assign() => {
			let left_kind = infer_type(left, scope);
			let right_kind = infer_type(right, scope);
			if left_kind == Kind::Float || right_kind == Kind::Float {
				Kind::Float
			} else {
				Kind::Int
			}
		}
		// global:value -> type comes from value
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					return infer_type(right, scope);
				}
			}
			// Tag structures like html:body are Key
			Kind::Key
		}
		// Comparison operators return Int (boolean as 0/1)
		Node::Key(_, op, _) if op.is_comparison() => Kind::Int,
		// Prefix operators (neg, abs, sqrt): inherit type from operand
		Node::Key(left, op, right) if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) => {
			infer_type(right, scope)
		}
		// Ternary operator: condition ? then : else
		Node::Key(_cond, Op::Question, then_else) => {
			// Extract then and else branches from the Key(then, Colon, else) structure
			if let Node::Key(then_expr, Op::Colon, else_expr) = then_else.drop_meta() {
				let then_kind = infer_type(then_expr, scope);
				let else_kind = infer_type(else_expr, scope);
				// If either branch returns a reference type (Text, Symbol, etc.), return Text
				if then_kind == Kind::Text || else_kind == Kind::Text {
					Kind::Text
				} else if then_kind.is_ref() || else_kind.is_ref() {
					Kind::Text  // Default reference return type
				} else if then_kind == Kind::Float || else_kind == Kind::Float {
					Kind::Float
				} else {
					Kind::Int
				}
			} else {
				Kind::Int
			}
		}
		// Default to Int for other cases
		_ => Kind::Int,
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
					if scope.lookup(name).is_none() {
						let kind = infer_type(right, scope);
						scope.define(name.clone(), None, kind);
					}
				}
			}
			collect_variables_inner(right, scope, false, in_structure)
		}
		// Assign creates variables only at top level (not inside structures)
		Node::Key(left, Op::Assign, right) => {
			if !skip_first_assign && !in_structure {
				// Check for typed declaration: Key(Key(name, Colon, type), Assign, value)
				match left.drop_meta() {
					Node::Symbol(name) => {
						if scope.lookup(name).is_none() {
							let kind = infer_type(right, scope);
							scope.define(name.clone(), None, kind);
						}
					}
					// Typed variable: x:int = 1 parses as Key(Key(x, Colon, int), Assign, 1)
					Node::Key(var_name, Op::Colon, type_node) => {
						if let Node::Symbol(name) = var_name.drop_meta() {
							if scope.lookup(name).is_none() {
								// Get kind from type annotation
								let type_str = type_node.drop_meta().to_string();
								let kind = type_name_to_kind(&type_str);
								scope.define(name.clone(), Some(type_node.clone()), kind);
							}
						}
					}
					_ => {}
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
		Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
			// Integer abs needs a temp local for the if-then-else pattern
			1 + collect_variables_inner(right, scope, false, in_structure)
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
	pub fn define(&mut self, name: String, type_node: Option<Box<Node>>, kind: Kind) -> Local {
		let position = self.locals.len() as u32;
		let local = Local {
			name: name.clone(),
			type_node,
			position,
			is_param: false,
			kind,
			data_pointer: 0,
			data_length: 0,
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
			kind: Kind::Int,  // Default to Int for params
			data_pointer: 0,
			data_length: 0,
		};
		self.locals.insert(name, local.clone());
		local
	}

	/// Update a local's data pointer and length (for string assignments)
	pub fn set_local_data(&mut self, name: &str, pointer: u32, length: u32) {
		if let Some(local) = self.locals.get_mut(name) {
			local.data_pointer = pointer;
			local.data_length = length;
		}
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
	let mut scope = Scope::new();
	if let Some(err) = check_type_errors(&raw, &mut scope) {
		return err;
	}
	raw
}

/// Check for type errors in the AST, returns Some(Node::Error) if found
fn check_type_errors(node: &Node, scope: &mut Scope) -> Option<Node> {
	check_type_errors_inner(node, scope, false)
}

fn check_type_errors_inner(node: &Node, scope: &mut Scope, in_structure: bool) -> Option<Node> {
	let node = node.drop_meta();
	match node {
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					return check_type_errors_inner(right, scope, false);
				}
				// Tag structure: check both sides, right is structure context
				if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
					return Some(err);
				}
				return check_type_errors_inner(right, scope, true);
			}
			if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
				return Some(err);
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, Op::Define, right) => {
			if let Node::Symbol(name) = left.drop_meta() {
				let new_kind = infer_type(right, scope);
				if let Some(existing) = scope.lookup(name) {
					if !types_compatible(existing.kind, new_kind) {
						return Some(type_error(name, existing.kind, new_kind));
					}
				} else {
					scope.define(name.clone(), None, new_kind);
				}
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, Op::Assign, right) => {
			if !in_structure {
				if let Node::Symbol(name) = left.drop_meta() {
					let new_kind = infer_type(right, scope);
					if let Some(existing) = scope.lookup(name) {
						if !types_compatible(existing.kind, new_kind) {
							return Some(type_error(name, existing.kind, new_kind));
						}
					} else {
						scope.define(name.clone(), None, new_kind);
					}
				}
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, _, right) => {
			if let Some(err) = check_type_errors_inner(left, scope, in_structure) {
				return Some(err);
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::List(items, _, _) => {
			for item in items {
				if let Some(err) = check_type_errors_inner(item, scope, in_structure) {
					return Some(err);
				}
			}
			None
		}
		_ => None,
	}
}

fn type_error(name: &str, existing: Kind, new: Kind) -> Node {
	Node::Error(Box::new(Node::Text(format!(
		"type mismatch: cannot assign {} to variable '{}' of type {}",
		new, name, existing
	))))
}

/// Check if two types are compatible for assignment
fn types_compatible(existing: Kind, new: Kind) -> bool {
	match (existing, new) {
		// Same type is always compatible
		(a, b) if a == b => true,
		// Int and Float are NOT compatible (x=1; x=1.0 should fail)
		(Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => false,
		// Text and other types are NOT compatible
		(Kind::Text, _) | (_, Kind::Text) => false,
		// Codepoint and Int may be compatible (char as number)
		(Kind::Int, Kind::Codepoint) | (Kind::Codepoint, Kind::Int) => true,
		// Default: incompatible
		_ => false,
	}
}

/// Collect all function declarations from the AST into a FunctionRegistry
pub fn collect_functions(node: &Node) -> FunctionRegistry {
	let mut registry = FunctionRegistry::new();
	collect_functions_inner(node, &mut registry);
	registry
}

fn collect_functions_inner(node: &Node, registry: &mut FunctionRegistry) {
	let node = node.drop_meta();
	match node {
		// Pattern: fun/fn/def/define/function name(params...) body
		Node::List(items, _, _) if items.len() >= 2 => {
			if let Node::Symbol(keyword) = items[0].drop_meta() {
				if is_function_keyword(keyword) {
					if let Some(func) = parse_function_declaration(items, keyword) {
						// Emit normalization hint for function keyword style
						let params_str = func.signature.parameters.iter()
							.map(|p| p.name.clone())
							.collect::<Vec<_>>()
							.join(", ");
						norm::function_keyword(keyword, &func.name, &params_str);
						registry.register(func);
						return;
					}
				}
			}
			// Recurse into list items
			for item in items {
				collect_functions_inner(item, registry);
			}
		}
		Node::Key(left, _, right) => {
			collect_functions_inner(left, registry);
			collect_functions_inner(right, registry);
		}
		_ => {}
	}
}

/// Parse a function declaration from a list starting with fun/fn/def/define/function
fn parse_function_declaration(items: &[Node], _keyword: &str) -> Option<Function> {
	// Structure: [keyword, ((name (type param)...) body)]
	// or: [keyword, ((name params...) body)]
	if items.len() < 2 {
		return None;
	}

	let decl = items[1].drop_meta();

	// Get function name and params from the declaration structure
	let (name, params, body) = match decl {
		// Pattern: ((name params...) body)
		Node::List(decl_items, _, _) if !decl_items.is_empty() => {
			let first = decl_items[0].drop_meta();
			match first {
				// (name params...)
				Node::List(name_params, _, _) if !name_params.is_empty() => {
					let func_name = name_params[0].name();
					let params = &name_params[1..];
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(func_name, params.to_vec(), body)
				}
				// Just a name symbol
				Node::Symbol(name) => {
					let body = if decl_items.len() > 1 {
						Some(Box::new(decl_items[1].clone()))
					} else {
						None
					};
					(name.clone(), Vec::new(), body)
				}
				_ => return None,
			}
		}
		_ => return None,
	};

	if name.is_empty() {
		return None;
	}

	let mut func = Function::new(&name);
	func.body = body;

	// Parse parameters
	for param in &params {
		let (param_name, param_kind) = parse_param(param);
		func.signature.add(&param_name, param_kind);
	}

	Some(func)
}

/// Parse a parameter node into (name, kind)
fn parse_param(param: &Node) -> (String, Kind) {
	let param = param.drop_meta();
	match param {
		// Pattern: (name:type) - single item list containing a Key
		Node::List(items, _, _) if items.len() == 1 => {
			parse_param(&items[0])
		}
		// Pattern: (type name) e.g., (float a)
		Node::List(items, _, _) if items.len() >= 2 => {
			let type_name = items[0].name();
			let param_name = items[1].name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Pattern: name:type
		Node::Key(left, Op::Colon, right) => {
			let param_name = left.name();
			let type_name = right.name();
			let kind = type_name_to_kind(&type_name);
			(param_name, kind)
		}
		// Just a name (infer type later)
		Node::Symbol(name) => (name.clone(), Kind::Int),
		_ => (String::new(), Kind::Int),
	}
}

/// Convert type name string to Kind
fn type_name_to_kind(name: &str) -> Kind {
	match name.to_lowercase().as_str() {
		"int" | "i32" | "i64" | "integer" | "long" => Kind::Int,
		"float" | "f32" | "f64" | "double" | "real" | "number" => Kind::Float,
		"string" | "str" | "text" => Kind::Text,
		"bool" | "boolean" => Kind::Int, // Booleans are i32/i64
		"char" | "codepoint" => Kind::Codepoint,
		_ => Kind::Int, // Default to Int
	}
}

/// Extract user-defined functions from the AST into context
/// Infer return type of a function body given its parameters
fn infer_function_return_kind(params: &[(String, Option<Node>)], body: &Node) -> Kind {
	let mut scope = Scope::new();
	// Add parameters to scope (all assumed Int for now)
	for (name, _default) in params {
		scope.define(name.clone(), None, Kind::Int);
	}
	infer_type(body, &scope)
}

/// Recognizes patterns:
/// - `name(param) = body` → Key(List[name, param], Assign, body)
/// - `name := body` → Key(Symbol(name), Define, body) (uses implicit `it`)
pub fn extract_user_functions(ctx: &mut Context, node: &Node) {
	extract_user_functions_inner(ctx, node);
}

fn extract_user_functions_inner(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		// Pattern: name(param1, param2, ...) = body
		Node::Key(left, Op::Assign, body) => {
			if let Node::List(items, _, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						let params: Vec<(String, Option<Node>)> =
							items.iter().skip(1).filter_map(extract_param).collect();
						let return_kind = infer_function_return_kind(&params, body);
						let func_def = UserFunctionDef {
							name: name.clone(),
							params,
							body: body.clone(),
							return_kind,
							func_index: None,
						};
						ctx.user_functions.insert(name.clone(), func_def);
						return;
					}
				}
			}
			extract_user_functions_inner(ctx, left);
			extract_user_functions_inner(ctx, body);
		}
		// Pattern: name x := body (with explicit parameter x using $0 or `it`)
		Node::Key(left, Op::Define, body) => {
			if let Node::List(items, _, _) = left.drop_meta() {
				if !items.is_empty() {
					if let Node::Symbol(name) = items[0].drop_meta() {
						let params: Vec<(String, Option<Node>)> =
							items.iter().skip(1).filter_map(extract_param).collect();
						if !params.is_empty() || uses_dollar_param(body) || uses_it(body) {
							let actual_params = if params.is_empty() {
								vec![("it".to_string(), None)]
							} else {
								params
							};
							let return_kind = infer_function_return_kind(&actual_params, body);
							let func_def = UserFunctionDef {
								name: name.clone(),
								params: actual_params,
								body: body.clone(),
								return_kind,
								func_index: None,
							};
							ctx.user_functions.insert(name.clone(), func_def);
							return;
						}
					}
				}
			}
			// Pattern: name := body (uses implicit `it` parameter)
			if let Node::Symbol(name) = left.drop_meta() {
				if uses_it(body) || uses_dollar_param(body) {
					let params = vec![("it".to_string(), None)];
					let return_kind = infer_function_return_kind(&params, body);
					let func_def = UserFunctionDef {
						name: name.clone(),
						params,
						body: body.clone(),
						return_kind,
						func_index: None,
					};
					ctx.user_functions.insert(name.clone(), func_def);
					return;
				}
			}
			extract_user_functions_inner(ctx, left);
			extract_user_functions_inner(ctx, body);
		}
		// Check for def/fun/fn syntax
		Node::List(items, _, _) => {
			if items.len() >= 2 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					if is_function_keyword(s) {
						if let Some(func_def) = extract_def_function(&items[1..]) {
							ctx.user_functions.insert(func_def.name.clone(), func_def);
							return;
						}
					}
				}
			}
			for item in items {
				extract_user_functions_inner(ctx, item);
			}
		}
		Node::Key(left, _, right) => {
			extract_user_functions_inner(ctx, left);
			extract_user_functions_inner(ctx, right);
		}
		_ => {}
	}
}

/// Extract parameter name and optional default value from a parameter node
fn extract_param(item: &Node) -> Option<(String, Option<Node>)> {
	match item.drop_meta() {
		Node::Symbol(s) => Some((s.clone(), None)),
		Node::Key(n, Op::Colon, _) => {
			if let Node::Symbol(s) = n.drop_meta() {
				Some((s.clone(), None))
			} else {
				None
			}
		}
		Node::Key(n, Op::Assign, default) => {
			if let Node::Symbol(s) = n.drop_meta() {
				Some((s.clone(), Some(default.as_ref().clone())))
			} else {
				None
			}
		}
		_ => None,
	}
}

/// Check if a node uses the implicit `it` parameter
fn uses_it(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s == "it" => true,
		Node::Key(left, _, right) => uses_it(left) || uses_it(right),
		Node::List(items, _, _) => items.iter().any(uses_it),
		_ => false,
	}
}

/// Check if a node uses $n parameter references (e.g., $0, $1)
fn uses_dollar_param(node: &Node) -> bool {
	let node = node.drop_meta();
	match node {
		Node::Symbol(s) if s.starts_with('$') && s[1..].parse::<u32>().is_ok() => true,
		Node::Key(left, _, right) => uses_dollar_param(left) || uses_dollar_param(right),
		Node::List(items, _, _) => items.iter().any(uses_dollar_param),
		_ => false,
	}
}

/// Extract function from def/fun/fn syntax
fn extract_def_function(items: &[Node]) -> Option<UserFunctionDef> {
	if items.is_empty() {
		return None;
	}
	let first = items[0].drop_meta();

	// Pattern 1: def (name params...): body
	if let Node::Key(sig, Op::Colon, body) = first {
		if let Node::List(sig_items, _, _) = sig.drop_meta() {
			if !sig_items.is_empty() {
				if let Node::Symbol(name) = sig_items[0].drop_meta() {
					let params: Vec<(String, Option<Node>)> =
						sig_items.iter().skip(1).filter_map(extract_param).collect();
					let return_kind = infer_function_return_kind(&params, body);
					return Some(UserFunctionDef {
						name: name.clone(),
						params,
						body: body.clone(),
						return_kind,
						func_index: None,
					});
				}
			}
		}
	}

	// Pattern 2: def ((name params...) {body})
	if let Node::List(inner_items, _, _) = first {
		if inner_items.len() >= 2 {
			if let Node::List(sig_items, _, _) = inner_items[0].drop_meta() {
				if !sig_items.is_empty() {
					if let Node::Symbol(name) = sig_items[0].drop_meta() {
						let params: Vec<(String, Option<Node>)> = sig_items
							.iter()
							.skip(1)
							.flat_map(|item| {
								match item.drop_meta() {
									Node::List(param_items, _, _) => {
										param_items.iter().filter_map(extract_param).collect::<Vec<_>>()
									}
									_ => extract_param(item).into_iter().collect(),
								}
							})
							.collect();
						let body = inner_items[1].clone();
						let return_kind = infer_function_return_kind(&params, &body);
						return Some(UserFunctionDef {
							name: name.clone(),
							params,
							body: Box::new(body),
							return_kind,
							func_index: None,
						});
					}
				}
			}
		}
	}
	None
}

/// Analyze node tree for non-default required functions.
/// Default functions (new_empty, new_int, new_float, new_text, new_symbol, new_codepoint, new_key, new_list)
/// are always included and don't need to be inserted here.
pub fn analyze_required_functions(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::Empty | Node::Number(_) | Node::Symbol(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False => {}
		Node::Key(key, op, value) => {
			if *op == Op::Assign {
				if let Node::Key(_, Op::Hash, _) = key.drop_meta() {
					ctx.required_functions.insert("node_set_at");
					ctx.required_functions.insert("string_set_char_at");
					ctx.required_functions.insert("list_set_at");
					analyze_required_functions(ctx, key);
					analyze_required_functions(ctx, value);
					return;
				}
			}
			if *op == Op::Pow {
				ctx.required_functions.insert("i64_pow");
			} else if *op == Op::Square || *op == Op::Cube {
				analyze_required_functions(ctx, key);
				return;
			} else if op.is_prefix() && matches!(key.drop_meta(), Node::Empty) {
				analyze_required_functions(ctx, value);
				return;
			} else if *op == Op::Hash {
				if matches!(key.drop_meta(), Node::Empty) {
					ctx.required_functions.insert("node_count");
				} else {
					ctx.required_functions.insert("node_index_at");
					ctx.required_functions.insert("string_char_at");
					ctx.required_functions.insert("list_node_at");
					ctx.required_functions.insert("list_at");
				}
			} else if *op == Op::Dot {
				let method_name = match value.drop_meta() {
					Node::Symbol(s) => Some(s.clone()),
					Node::List(items, _, _) if items.len() == 1 => {
						if let Node::Symbol(s) = items[0].drop_meta() {
							Some(s.clone())
						} else {
							None
						}
					}
					_ => None,
				};
				if let Some(method) = method_name {
					if matches!(method.as_str(), "count" | "number" | "size") {
						ctx.required_functions.insert("node_count");
						return;
					}
				}
			}
			analyze_required_functions(ctx, key);
			analyze_required_functions(ctx, value);
		}
		Node::List(items, _, _) => {
			if items.is_empty() {
				return;
			}
			if let Node::Symbol(fn_name) = items[0].drop_meta() {
				if ctx.ffi_imports.contains_key(fn_name.as_str()) {
					for item in items.iter().skip(1) {
						analyze_required_functions(ctx, item);
					}
					return;
				}
				if items.len() == 2 && matches!(fn_name.as_str(), "count" | "size") {
					ctx.required_functions.insert("node_count");
					return;
				}
			}
			for item in items {
				analyze_required_functions(ctx, item);
			}
		}
		Node::Data(_) => {
			ctx.required_functions.insert("new_data");
		}
		Node::Meta { node, .. } => {
			analyze_required_functions(ctx, node);
		}
		Node::Type { name, body } => {
			ctx.required_functions.insert("new_type");
			ctx.type_registry.register_from_node(node);
			analyze_required_functions(ctx, name);
			analyze_required_functions(ctx, body);
		}
		Node::Error(inner) => {
			analyze_required_functions(ctx, inner);
		}
	}
}

/// Recursively collect all type definitions from the AST into the TypeRegistry
/// This pre-scan enables forward references (use a type before defining it)
pub fn collect_all_types(registry: &mut crate::type_kinds::TypeRegistry, node: &Node) {
	match node.drop_meta() {
		Node::Type { .. } => {
			registry.register_from_node(node);
		}
		Node::Key(l, _, r) => {
			collect_all_types(registry, l);
			collect_all_types(registry, r);
		}
		Node::List(items, _, _) => {
			for item in items {
				collect_all_types(registry, item);
			}
		}
		Node::Meta { node, .. } => collect_all_types(registry, node),
		_ => {}
	}
}

/// Extract FFI imports from "import X from Y" and "use Y" statements
pub fn extract_ffi_imports(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::List(items, _, _) => {
			if !items.is_empty() {
				match items[0].drop_meta() {
					Node::Symbol(first_sym) => {
						if first_sym == "import" && items.len() >= 2 {
							if items.len() == 2 {
								let lib = items[1].name();
								add_ffi_lib(ctx, &lib);
								return;
							}
							let func_name = items[1].name();
							if items.len() >= 3 {
								if let Node::Key(ref key, _, ref value) = items[2].drop_meta() {
									if key.name() == "from" {
										let lib = value.name();
										add_ffi_import(ctx, &func_name, &lib);
										return;
									}
								}
							}
							if items.len() >= 4 && items[2].name() == "from" {
								let lib = items[3].name();
								add_ffi_import(ctx, &func_name, &lib);
								return;
							}
						} else if first_sym == "use" && items.len() >= 2 {
							let lib = items[1].name();
							add_ffi_lib(ctx, &lib);
							return;
						}
					}
					Node::List(inner_items, _, _) => {
						if inner_items.len() >= 2 {
							if let Node::Symbol(inner_first) = inner_items[0].drop_meta() {
								if inner_first == "use" {
									let lib = inner_items[1].name();
									add_ffi_lib(ctx, &lib);
								}
							}
						}
					}
					_ => {}
				}
			}
			for item in items {
				extract_ffi_imports(ctx, item);
			}
		}
		Node::Key(ref key, _, ref value) => {
			if key.name() == "import" {
				if let Node::Key(ref from_key, _, ref lib) = value.drop_meta() {
					if from_key.name() == "from" {
						let func_name = key.name();
						let lib_name = lib.name();
						add_ffi_import(ctx, &func_name, &lib_name);
						return;
					}
				}
			}
			extract_ffi_imports(ctx, key);
			extract_ffi_imports(ctx, value);
		}
		Node::Meta { ref node, .. } => {
			extract_ffi_imports(ctx, node);
		}
		_ => {}
	}
}

/// Add an FFI import by function name
fn add_ffi_import(ctx: &mut Context, name: &str, library: &str) {
	use crate::ffi::{get_ffi_signature, get_ffi_signature_from_lib};

	let sig = get_ffi_signature_from_lib(name, library)
		.or_else(|| get_ffi_signature(name));

	if let Some(sig) = sig {
		ctx.ffi_imports.insert(name.to_string(), sig);
	}
}

/// Add all common functions from a library
fn add_ffi_lib(ctx: &mut Context, lib: &str) {
	let lib_alias = crate::ffi::resolve_library_alias(lib);
	if lib_alias == "m" {
		for name in ["fmin", "fmax", "fabs", "floor", "ceil", "round", "sqrt", "sin", "cos", "tan", "fmod", "pow", "exp", "log", "log10"] {
			add_ffi_import(ctx, name, "m");
		}
	} else if lib_alias == "c" {
		for name in ["strlen", "atoi", "atol", "atof", "strcmp", "strncmp", "rand"] {
			add_ffi_import(ctx, name, "c");
		}
	} else {
		add_ffi_lib_dynamic(ctx, lib);
	}
}

/// Dynamically discover and add all functions from a library via header parsing
fn add_ffi_lib_dynamic(ctx: &mut Context, lib: &str) {
	use crate::ffi::get_signatures_from_headers;

	let signatures = get_signatures_from_headers(lib);
	if signatures.is_empty() {
		eprintln!("[FFI] Warning: No functions found for library '{}'", lib);
		return;
	}

	for (name, sig) in signatures {
		ctx.ffi_imports.insert(name, sig);
	}
}
