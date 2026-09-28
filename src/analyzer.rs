use crate::context::{Context, UserFunctionDef};
use crate::diagnostic::Diagnostic;
use crate::extensions::numbers::Number;
use crate::function::{Function, FunctionRegistry, Signature};
use crate::local::Local;
use crate::node::{Bracket, Node, Separator};
use crate::normalize::hints as norm;
use crate::operators::{is_function_keyword, Op};
use crate::type_kinds::{canonical_type_name, Kind};
use std::collections::{HashMap, HashSet};

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
/// Result kind of `left op right`: exact (Int, which includes ratios like `1/4`) unless an f64 is involved
pub fn arithmetic_kind(left: Kind, op: &Op, right: Kind) -> Kind {
	if *op == Op::Add && [left, right].iter().all(|kind| matches!(kind, Kind::List | Kind::Empty)) && [left, right].contains(&Kind::List) {
		Kind::List // concatenation
	} else if [left, right].iter().any(|kind| matches!(kind, Kind::Text | Kind::Codepoint | Kind::List)) {
		Kind::Error // no implicit conversion (DESIGN.md "Dangerous implicitness")
	} else if left == Kind::Float || right == Kind::Float {
		Kind::Float
	} else {
		Kind::Int
	}
}

/// Kind as written: a literal keeps its data kind (`3.14` is a float literal even though its value
/// computes exactly), anything else is inferred
pub fn written_kind(node: &Node, scope: &Scope) -> Kind {
	match node.drop_meta() {
		literal @ Node::Number(_) => literal.kind(),
		other => infer_type(other, scope),
	}
}

pub fn infer_type(node: &Node, scope: &Scope) -> Kind {
	let node = node.drop_meta();
	match node {
		// Decimal literals are exact numbers, see wasm_emitter/exact.rs
		Node::Number(Number::Float(f)) if Number::is_exact_decimal(*f) => Kind::Int,
		Node::Number(Number::Float(_)) => Kind::Float,
		Node::Number(Number::Complex(_, _)) => Kind::Float,
		// Integer and rational literals
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
			// Type constructor: int("5"), float("1.5"), str(3)
			if items.len() == 2 {
				if let Node::Symbol(s) = items[0].drop_meta() {
					match s.as_str() {
						"int" | "integer" => return Kind::Int,
						"float" => return Kind::Float,
						"str" | "string" | "text" => return Kind::Text,
						_ => {}
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
			// Grouping: (x) has the type of x
			if *bracket == Bracket::Round && items.len() == 1 {
				return infer_type(&items[0], scope);
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
			arithmetic_kind(infer_type(left, scope), op, infer_type(right, scope))
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
		// `v as float` is an f64; `as int`, `as exact` stay exact Ints
		Node::Key(_, Op::As, target) if builtin_type_kind(&target.name()).is_some_and(|kind| kind.is_float()) => Kind::Float,
		// Comparison operators return Int (boolean as 0/1)
		Node::Key(_, op, _) if op.is_comparison() => Kind::Int,
		// √x is irrational in general: an f64
		Node::Key(left, Op::Sqrt, _) if matches!(left.drop_meta(), Node::Empty) => Kind::Float,
		// Prefix operators (neg, abs): inherit type from operand
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
				if then_kind.is_ref() || else_kind.is_ref() {
					Kind::Text
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
						let kind = binding_kind(right, scope);
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
					Node::Symbol(name) if scope.lookup(name).is_none() => {
						let declared = declared_type(left);
						let kind = declared.and_then(|type_name| declared_kind(&type_name.name())).unwrap_or_else(|| binding_kind(right, scope));
						scope.define(name.clone(), declared.map(|type_name| Box::new(type_name.clone())), kind);
					}
					// Typed variable: x:int = 1 parses as Key(Key(x, Colon, int), Assign, 1)
					Node::Key(var_name, Op::Colon, type_node) => {
						if let Node::Symbol(name) = var_name.drop_meta() {
							if scope.lookup(name).is_none() {
								// Get kind from type annotation
								let type_str = type_node.drop_meta().to_string();
								let kind = declared_kind(&type_str).unwrap_or(Kind::Int);
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

/// Kind of a new variable bound to `value`: `x=ø` makes x an optional, held as a Node that is ø until assigned
fn binding_kind(value: &Node, scope: &Scope) -> Kind {
	match value.drop_meta() {
		Node::Empty => Kind::Empty,
		_ => infer_type(value, scope),
	}
}

/// Kind of a variable declared `x:T`; an optional `T?` may hold ø, so it is held as a Node
fn declared_kind(type_name: &str) -> Option<Kind> {
	match type_name.strip_suffix('?') {
		Some(_) => Some(Kind::Empty),
		None => builtin_type_kind(type_name),
	}
}

/// Outer variables a function body reads: bound in `outer`, not a parameter or local of the body.
/// Functions capture these by value when they are defined (DESIGN.md: immutable local bindings).
pub fn captured_variables(function: &UserFunctionDef, outer: &Scope) -> Vec<(String, Kind)> {
	let mut own = Scope::new();
	for (name, _default) in &function.params {
		own.define(name.clone(), None, Kind::Int);
	}
	collect_variables(&function.body, &mut own);
	let mut captured: Vec<(String, Kind)> = vec![];
	function.body.visit(&mut |node| {
		if let Node::Symbol(name) = node {
			let is_new = own.lookup(name).is_none() && !captured.iter().any(|(seen, _)| seen == name);
			if let Some(local) = outer.lookup(name).filter(|_| is_new) {
				captured.push((name.clone(), local.kind));
			}
		}
	});
	captured
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
				if let Some(err) = check_assignment(name, right, scope) {
					return Some(err);
				}
			}
			check_type_errors_inner(right, scope, in_structure)
		}
		Node::Key(left, Op::Assign, right) => {
			if !in_structure {
				if let Node::Symbol(name) = left.drop_meta() {
					if let Some(err) = check_assignment(name, right, scope) {
						return Some(err);
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

/// `name = value` must keep the variable's kind; a new variable takes the value's kind
fn check_assignment(name: &str, value: &Node, scope: &mut Scope) -> Option<Node> {
	match scope.lookup(name) {
		Some(existing) => {
			let new_kind = written_kind(value, scope);
			(!types_compatible(existing.kind, new_kind)).then(|| type_error(name, existing.kind, new_kind))
		}
		None => {
			let kind = infer_type(value, scope);
			scope.define(name.to_string(), None, kind);
			None
		}
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
	builtin_type_kind(name).unwrap_or(Kind::Int)
}

/// Kind of a built-in type name; an exact number (`exact`, `real`) is an Int that may hold a ratio (wasm_emitter/exact.rs)
pub fn builtin_type_kind(name: &str) -> Option<Kind> {
	Some(match canonical_type_name(&name.to_lowercase()) {
		"int" | "i32" | "i64" | "integer" | "long" | "exact" => Kind::Int,
		"float" | "f32" | "number" => Kind::Float,
		"string" | "str" | "text" => Kind::Text,
		"bool" | "boolean" => Kind::Int, // Booleans are i32/i64
		"char" | "codepoint" => Kind::Codepoint,
		_ => return None,
	})
}

/// Semantic checks run before emission; the first violation comes back as an error value
pub fn diagnose(program: &Node) -> Option<Node> {
	check_declared_types(program, &mut HashMap::new())
		.or_else(|| check_constants(program, &mut HashSet::new()))
		.or_else(|| check_null_use(program, &mut HashSet::new()))
		.or_else(|| check_boolean_arithmetic(program))
		.or_else(|| check_ambiguous_calls(program))
		.map(Diagnostic::into_error)
}

/// Warnings that do not stop compilation: well-defined code that likely does not mean what it says
pub fn lint(program: &Node) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	lint_into(program, &mut warnings);
	warnings
}

fn lint_into(node: &Node, warnings: &mut Vec<Diagnostic>) {
	match node.drop_meta() {
		Node::Key(left, op, right) => {
			if let (Op::Or, Node::Key(condition, Op::And, then)) = (op, left.drop_meta()) {
				let (condition, then, otherwise) = (condition.serialize(), then.serialize(), right.serialize());
				warnings.push(Diagnostic::at(node, format!("`{condition} and {then} or {otherwise}` yields {otherwise} whenever {then} is falsy"))
					.fix(format!("if {condition} then {then} else {otherwise}")));
			}
			if *op == Op::Mod && (is_negative(left) || is_negative(right)) {
				warnings.push(Diagnostic::at(node, negative_modulo_warning(left, right)));
			}
			lint_into(left, warnings);
			lint_into(right, warnings);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| lint_into(item, warnings)),
		_ => {}
	}
}

/// A negative literal or a negation: `-7`, `-x`, `(-7)`
fn is_negative(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(Number::Int(n)) => *n < 0,
		Node::Number(Number::Float(f)) => *f < 0.0,
		Node::Number(Number::Quotient(numerator, _)) => *numerator < 0,
		Node::Number(Number::BigInt(big)) => big.sign() == num_bigint::Sign::Minus,
		Node::Key(left, Op::Neg | Op::Sub, _) => matches!(left.drop_meta(), Node::Empty),
		Node::List(items, _, _) if items.len() == 1 => is_negative(&items[0]),
		_ => false,
	}
}

/// `%` is Euclidean (0 ≤ r < |b|); C, Java, JS and Rust truncate, Python floors
fn negative_modulo_warning(left: &Node, right: &Node) -> String {
	let (a, b) = (left.serialize(), right.serialize());
	let (a, b) = (a.trim(), b.trim());
	let values = match (left.drop_meta(), right.drop_meta()) {
		(Node::Number(Number::Int(x)), Node::Number(Number::Int(y))) => x.checked_rem_euclid(*y).zip(x.checked_rem(*y)),
		_ => None,
	};
	match values {
		Some((euclidean, truncated)) if euclidean != truncated => format!(
			"`{a} % {b}` is {euclidean}: % is Euclidean as in mathematics; C/Java/JS give {truncated}; use `rem` for the truncated remainder"
		),
		_ => format!("`{a} % {b}`: % is Euclidean as in mathematics (never negative), unlike C/Java/JS; use `rem` for the truncated remainder"),
	}
}

/// A braceless argument takes arithmetic (`f 3-1` is `f(3-1)`, also in `1 + f 3-1`), so a second braceless call inside it
/// is ambiguous (wiki/precedence.md): `square 3 + square 3` reads as `square(3 + square 3)` or `square(3) + square(3)`.
/// Bad.md's recursive `fib it-1 + fib it-2` would silently mean `fib(it-1 + fib(it-2))`, so it is rejected with both readings.
fn check_ambiguous_calls(node: &Node) -> Option<Diagnostic> {
	const KEYWORDS: [&str; 10] = ["return", "const", "let", "var", "def", "fun", "fn", "use", "import", "include"];
	fn braceless_call(node: &Node) -> Option<(&String, &Node)> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) if items.len() == 2 => match items[0].drop_meta() {
				Node::Symbol(head) if !KEYWORDS.contains(&head.as_str()) => Some((head, &items[1])),
				_ => None,
			},
			_ => None,
		}
	}
	fn holds_call(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => holds_call(left) || holds_call(right),
			other => braceless_call(other).is_some(),
		}
	}
	fn explicit(node: &Node) -> String {
		if let Some((head, argument)) = braceless_call(node) {
			return format!("{head}({})", explicit(argument));
		}
		match node.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => format!("{} {} {}", explicit(left), op.as_str(), explicit(right)),
			other => other.serialize(),
		}
	}
	if let Some((head, argument)) = braceless_call(node) {
		if let Node::Key(left, op, right) = argument.drop_meta() {
			if op.is_arithmetic() && holds_call(argument) {
				let whole = format!("{head}({})", explicit(argument));
				let first = format!("{head}({}) {} {}", explicit(left), op.as_str(), explicit(right));
				return Some(Diagnostic::at(node, format!("ambiguous braceless call: {head} {}", argument.drop_meta().serialize()))
					.fix(format!("{first} or {whole}")));
			}
		}
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => check_ambiguous_calls(left).or_else(|| check_ambiguous_calls(right)),
		Node::List(items, _, _) => items.iter().find_map(check_ambiguous_calls),
		_ => None,
	}
}

/// Booleans are not numbers: `true + true` is rejected, not 2 (the runtime still encodes them as Int 1/0)
fn check_boolean_arithmetic(node: &Node) -> Option<Diagnostic> {
	fn is_boolean(operand: &Node) -> bool {
		match operand.drop_meta() {
			Node::True | Node::False => true,
			Node::Key(left, op, _) => op.is_comparison() || (*op == Op::Not && matches!(left.drop_meta(), Node::Empty)),
			Node::List(items, Bracket::Round, _) if items.len() == 1 => is_boolean(&items[0]),
			_ => false,
		}
	}
	match node.drop_meta() {
		Node::Key(left, op, right) if op.is_arithmetic() && (is_boolean(left) || is_boolean(right)) => {
			let expression = node.drop_meta().serialize();
			let converted = |operand: &Node| if is_boolean(operand) { format!("int({})", operand.serialize()) } else { operand.serialize() };
			Some(Diagnostic::at(node, format!("arithmetic on a boolean: {expression}"))
				.fix(format!("{} {} {}", converted(left), op.as_str(), converted(right))))
		}
		Node::Key(left, _, right) => check_boolean_arithmetic(left).or_else(|| check_boolean_arithmetic(right)),
		Node::List(items, _, _) => items.iter().find_map(check_boolean_arithmetic),
		_ => None,
	}
}

/// `x=ø` makes x possibly null: arithmetic or member access on it needs an `if x {…}` check first (wiki/null.md)
fn check_null_use(node: &Node, nullable: &mut HashSet<String>) -> Option<Diagnostic> {
	let possibly_null = |operand: &Node, nullable: &HashSet<String>| match operand.drop_meta() {
		Node::Symbol(name) if nullable.contains(name) => Some(name.clone()),
		_ => None,
	};
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			let found = check_null_use(value, nullable);
			let target = match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => &**name, // x:int?=ø
				other => other,
			};
			if let Node::Symbol(name) = target.drop_meta() {
				if matches!(value.drop_meta(), Node::Empty) {
					nullable.insert(name.clone());
				} else {
					nullable.remove(name);
				}
			}
			found
		}
		Node::Key(if_condition, Op::Then, then) => {
			let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else {
				return check_null_use(if_condition, nullable).or_else(|| check_null_use(then, nullable));
			};
			let mut narrowed = nullable.clone();
			if let Some(name) = possibly_null(condition, nullable) {
				narrowed.remove(&name);
			}
			check_null_use(condition, nullable).or_else(|| check_null_use(then, &mut narrowed))
		}
		// ø is the empty list: appending to it or concatenating a list needs no check (`a=(); a.add(1)`)
		Node::Key(list, Op::Dot, call) if appended_element(list, call).is_some() => {
			let found = check_null_use(call, nullable);
			nullable.remove(&list.name()); // no longer empty
			found
		}
		Node::Key(left, Op::Add, right) if [left, right].iter().any(|side| matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))) => {
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, op, right) if op.is_arithmetic() || *op == Op::Dot => {
			let operand = possibly_null(left, nullable).or_else(|| if *op == Op::Dot { None } else { possibly_null(right, nullable) });
			if let Some(name) = operand {
				let expression = node.drop_meta().serialize();
				return Some(Diagnostic::at(node, format!("{name} may be ø (null) in {expression}")).fix(format!("if {name} {{ {expression} }}")));
			}
			check_null_use(left, nullable).or_else(|| check_null_use(right, nullable))
		}
		Node::Key(left, _, right) => check_null_use(left, nullable).or_else(|| check_null_use(right, nullable)),
		Node::List(items, _, _) => items.iter().find_map(|item| check_null_use(item, nullable)),
		_ => None,
	}
}

/// `const x=…` binds x once; any later assignment, compound assignment, increment or element assignment is rejected
fn check_constants(node: &Node, constants: &mut HashSet<String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::List(items, _, _) => {
			let statements = match items.as_slice() {
				[keyword, declaration, rest @ ..] if matches!(keyword.drop_meta(), Node::Symbol(s) if s == "const") => {
					let Node::Key(target, Op::Assign | Op::Define, value) = declaration.drop_meta() else {
						return Some(Diagnostic::at(declaration, format!("const needs a value: {}", declaration.serialize())).fix("const x = 5".to_string()));
					};
					if let Some(found) = check_constants(value, constants) {
						return Some(found);
					}
					constants.insert(target.name());
					rest
				}
				_ => items.as_slice(),
			};
			statements.iter().find_map(|statement| check_constants(statement, constants))
		}
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec) || op.is_compound_assign() => {
			let place = match target.drop_meta() {
				Node::Key(name, Op::Hash, _) => name.name(),
				other => other.name(),
			};
			if constants.contains(&place) {
				let message = format!("{place} is const, cannot assign it again: {}", node.drop_meta().serialize());
				return Some(Diagnostic::at(node, message).fix(format!("use a new name instead of {place}, or declare it without const")));
			}
			check_constants(value, constants)
		}
		Node::Key(left, _, right) => check_constants(left, constants).or_else(|| check_constants(right, constants)),
		_ => None,
	}
}

/// `x:int=…` declares x's type; every value assigned to x later must fit it
fn check_declared_types(node: &Node, declared: &mut HashMap<String, String>) -> Option<Diagnostic> {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			let declaration = match target.drop_meta() {
				Node::Key(name, Op::Colon, type_name) => match (name.drop_meta(), type_name.drop_meta()) {
					(Node::Symbol(name), Node::Symbol(type_name)) => {
						declared.insert(name.clone(), type_name.clone());
						Some((name, type_name))
					}
					_ => None,
				},
				Node::Symbol(name) => declared.get_key_value(name),
				_ => None,
			};
			if let Some((name, type_name)) = declaration {
				if let Some(mismatch) = assignment_mismatch(node, name, type_name, value) {
					return Some(mismatch);
				}
			}
			check_declared_types(value, declared)
		}
		Node::Key(left, _, right) => check_declared_types(left, declared).or_else(|| check_declared_types(right, declared)),
		Node::List(items, _, _) => items.iter().find_map(|item| check_declared_types(item, declared)),
		_ => None,
	}
}

fn assignment_mismatch(assignment: &Node, name: &str, type_name: &str, value: &Node) -> Option<Diagnostic> {
	if let Some(base) = type_name.strip_suffix('?') {
		return match value.drop_meta() {
			Node::Empty => None,
			_ => assignment_mismatch(assignment, name, base, value),
		};
	}
	if matches!(value.drop_meta(), Node::Empty) && builtin_type_kind(type_name).is_some() {
		let message = format!("type mismatch: {name} is declared {type_name}, cannot assign ø");
		return Some(Diagnostic::at(assignment, message).fix(format!("declare {name}:{type_name}? to allow ø")));
	}
	let expected = builtin_type_kind(type_name)?;
	let actual = literal_kind(value)?;
	let exact_decimal = canonical_type_name(type_name) == "exact" && actual == Kind::Float;
	if expected == actual || (expected == Kind::Float && actual == Kind::Int) || exact_decimal {
		return None;
	}
	let value_text = value.serialize();
	let message = format!("type mismatch: {name} is declared {type_name}, cannot assign {} {value_text}", format!("{actual:?}").to_lowercase());
	Some(Diagnostic::at(assignment, message).fix(format!("{name}={type_name}({value_text}) or declare {name}:{}", format!("{actual:?}").to_lowercase())))
}

/// `x:T = v` → `x = v` with T kept as metadata on x (see `declared_type`);
/// the widening of an Int literal assigned to a float becomes an explicit Float literal
pub fn lower_declarations(node: Node) -> Node {
	match node {
		// `fast x=v` → `x:fast=v`, parsed either as `(fast x)=v` or as the statement pair `fast (x=v)`;
		// `double(x) := x+x` and `double x := x+x` stay function definitions
		Node::Key(target, Op::Assign, value) if number_type_prefix(&target).is_some() => {
			let (type_name, name) = number_type_prefix(&target).expect("guarded");
			lower_declarations(Node::Key(Box::new(Node::Key(Box::new(name), Op::Colon, Box::new(type_name))), Op::Assign, value))
		}
		Node::List(items, bracket, separator) if items.windows(2).any(|pair| prefixed_declaration(&pair[0], &pair[1]).is_some()) => {
			let mut lowered = Vec::with_capacity(items.len());
			let mut items = items.into_iter().peekable();
			while let Some(item) = items.next() {
				match items.peek().and_then(|next| prefixed_declaration(&item, next)) {
					Some(declaration) => {
						items.next();
						lowered.push(lower_declarations(declaration));
					}
					None => lowered.push(lower_declarations(item)),
				}
			}
			if lowered.len() == 1 {
				lowered.remove(0)
			} else {
				Node::List(lowered, bracket, separator)
			}
		}
		Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
			let value = Box::new(lower_declarations(*value));
			match target.drop_meta() {
				Node::Key(name, Op::Colon, type_name) if matches!((name.drop_meta(), type_name.drop_meta()), (Node::Symbol(_), Node::Symbol(_))) => {
					let value = match (builtin_type_kind(&type_name.name()), value.drop_meta()) {
						(Some(Kind::Float), Node::Number(number @ (Number::Int(_) | Number::BigInt(_)))) => Box::new(Node::Number(Number::Float(number.clone().into()))),
						_ => value,
					};
					Node::Key(Box::new(Node::meta(name.drop_meta().clone(), type_name.drop_meta().clone())), op, value)
				}
				_ => Node::Key(target, op, value),
			}
		}
		Node::Key(list, Op::Dot, call) if appended_element(&list, &call).is_some() => {
			let element = lower_declarations(appended_element(&list, &call).expect("guarded").clone());
			let appended = Node::Key(list.clone(), Op::Add, Box::new(Node::List(vec![element], Bracket::Square, Separator::Space)));
			Node::Key(list, Op::Assign, Box::new(appended))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(lower_declarations(*left)), op, Box::new(lower_declarations(*right))),
		// `const x=v` → `x=v`; check_constants already enforced the single assignment
		Node::List(items, bracket, separator) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(s) if s == "const") => {
			let mut declaration = items.into_iter().skip(1).map(lower_declarations).collect::<Vec<_>>();
			if declaration.len() == 1 {
				declaration.remove(0)
			} else {
				Node::List(declaration, bracket, separator)
			}
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower_declarations).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower_declarations(*node)), data },
		other => other,
	}
}

/// Methods that append one element; with value semantics `x.add(v)` rebinds `x = x + [v]`
const APPEND_METHODS: [&str; 3] = ["add", "append", "push"];

/// The element of `x.add(v)` when x is a variable
fn appended_element<'a>(list: &Node, call: &'a Node) -> Option<&'a Node> {
	let Node::Symbol(_) = list.drop_meta() else { return None };
	match call.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(method) if APPEND_METHODS.contains(&method.as_str())) => Some(&items[1]),
		_ => None,
	}
}

/// The type a lowered declaration `x:T = v` attached to its target x
fn declared_type(target: &Node) -> Option<&Node> {
	match target {
		Node::Meta { data, .. } if matches!(data.as_ref(), Node::Symbol(_)) => Some(data),
		_ => None,
	}
}

/// A number type written before a declaration: `real`, `exact`, `float`, `fast` … (see `canonical_type_name`)
fn is_number_type(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if matches!(canonical_type_name(name), "exact" | "float"))
}

/// `(fast x)` as the target of `fast x=v`: the type and the name
fn number_type_prefix(target: &Node) -> Option<(Node, Node)> {
	match target.drop_meta() {
		Node::List(items, bracket, _) if *bracket != Bracket::Round && items.len() == 2 && is_number_type(&items[0]) && matches!(items[1].drop_meta(), Node::Symbol(_)) => {
			Some((items[0].drop_meta().clone(), items[1].drop_meta().clone()))
		}
		_ => None,
	}
}

/// The statement pair `fast`, `x=v` as the declaration `x:fast=v`
fn prefixed_declaration(type_name: &Node, next: &Node) -> Option<Node> {
	match next.drop_meta() {
		Node::Key(name, op @ (Op::Assign | Op::Define), value) if is_number_type(type_name) && matches!(name.drop_meta(), Node::Symbol(_)) => {
			let target = Node::Key(Box::new(name.drop_meta().clone()), Op::Colon, Box::new(type_name.drop_meta().clone()));
			Some(Node::Key(Box::new(target), *op, value.clone()))
		}
		_ => None,
	}
}

/// Kind of a value known before running the program
fn literal_kind(value: &Node) -> Option<Kind> {
	match value.drop_meta() {
		Node::Number(Number::Int(_) | Number::BigInt(_)) | Node::True | Node::False => Some(Kind::Int),
		Node::Number(_) => Some(Kind::Float),
		Node::Text(_) => Some(Kind::Text),
		Node::Char(_) => Some(Kind::Codepoint),
		Node::Key(nothing, Op::Neg, operand) if matches!(nothing.drop_meta(), Node::Empty) => literal_kind(operand),
		_ => None,
	}
}

/// Extract user-defined functions from the AST into context
/// Infer return type of a function body given its parameters
fn infer_function_return_kind(params: &[(String, Option<Node>)], body: &Node) -> Kind {
	let mut scope = Scope::new();
	for (name, default) in params {
		scope.define(name.clone(), None, param_kind(default));
	}
	infer_type(body, &scope)
}

/// A parameter takes the kind of its default value (a fresh value per call); untyped parameters are Int
pub fn param_kind(default: &Option<Node>) -> Kind {
	match default.as_ref().map(|value| infer_type(value, &Scope::new())) {
		Some(kind @ (Kind::Float | Kind::Text | Kind::List)) => kind,
		_ => Kind::Int,
	}
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
pub(crate) fn extract_def_function(items: &[Node]) -> Option<UserFunctionDef> {
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

/// `count x`, `length x`, `size x`: the runtime function counting x in its unit. A text counts graphemes
/// (user-perceived characters) except for `size`, which counts bytes; a user function of that name wins.
pub fn counting_function(name: &str, ctx: &Context) -> Option<&'static str> {
	if ctx.user_functions.contains_key(name) {
		return None;
	}
	match name {
		"count" | "length" => Some("node_count"),
		"size" => Some("node_size"),
		_ => None,
	}
}

/// `x.count`, `x.length`, `x.size`, and the explicit text units `x.bytes`, `x.chars` (code points), `x.graphemes`
pub fn counting_method(name: &str, ctx: &Context) -> Option<&'static str> {
	match name {
		"number" => Some("node_count"),
		"bytes" => Some("text_byte_count"),
		"chars" | "codepoints" => Some("text_codepoint_count"),
		"graphemes" => Some("text_grapheme_count"),
		_ => counting_function(name, ctx),
	}
}

fn require_counter(ctx: &mut Context, counter: &'static str) {
	if counter == "node_size" {
		ctx.required_functions.insert("node_count");
	}
	ctx.required_functions.insert(counter);
}

/// Analyze node tree for non-default required functions.
/// Default functions (new_empty, new_int, new_float, new_text, new_symbol, new_codepoint, new_key, new_list)
/// are always included and don't need to be inserted here.
pub fn analyze_required_functions(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::Number(number) => {
			if !matches!(number, Number::Int(n) if crate::wasm_emitter::is_fixnum(*n)) {
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
		}
		Node::Text(text) => {
			if let Some(number) = crate::wasp_parser::number_in_text(text) {
				analyze_required_functions(ctx, &Node::Number(number));
			}
		}
		Node::Empty | Node::Symbol(_) | Node::Char(_) | Node::True | Node::False => {}
		Node::Key(key, op, value) => {
			if op.is_arithmetic()
				|| op.is_compound_assign()
				|| matches!(op, Op::Inc | Op::Dec | Op::Neg | Op::Abs | Op::Square | Op::Cube | Op::Xor)
			{
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
			if matches!(op, Op::Eq | Op::Ne) {
				ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
			}
			if matches!(op, Op::If | Op::While | Op::Question) {
				ctx.required_functions.insert(crate::wasm_emitter::IS_TRUTHY);
			}
			if *op == Op::Assign || op.is_compound_assign() {
				if let Node::Key(_, Op::Hash, _) = key.drop_meta() {
					ctx.required_functions.insert("node_with_at");
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
				if let Some(counter) = method_name.and_then(|method| counting_method(&method, ctx)) {
					require_counter(ctx, counter);
					return;
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
				if items.len() == 2 {
					if let Some(counter) = counting_function(fn_name, ctx) {
						require_counter(ctx, counter);
						return;
					}
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
					Node::List(inner_items, _, _) if inner_items.len() >= 2 => {
						if let Node::Symbol(inner_first) = inner_items[0].drop_meta() {
							if inner_first == "use" {
								let lib = inner_items[1].name();
								add_ffi_lib(ctx, &lib);
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
		for (name, _) in crate::ffi::LIBM_F64_FUNCTIONS {
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
