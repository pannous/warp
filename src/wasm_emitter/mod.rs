//! WASM GC code emitter - generates WebAssembly modules with GC support

mod big_int;
mod closures;
pub(crate) mod exact;
mod constructors;
mod equality;
mod function_builder;
mod config;
mod ffi_emitter;
mod float_text;
mod import_manager;
mod key_emitter;
mod layout;
mod list_emitter;
mod list_dispatch;
mod library_ops;
mod text_unicode;
mod list_ops;
mod loop_control;
pub(crate) use loop_control::mark_step;
pub(crate) mod text_builtins;
mod reflection;
mod string_table;
mod type_manager;
mod try_guard;
mod tuple_emitter;
pub use try_guard::{CAUGHT_ERROR_TEXT, RAN_WITHOUT_ERROR};
mod witness;
pub(crate) mod wasi_emitter;

pub use big_int::{is_fixnum, INT_RUNTIME};

/// Something emission found it must have that the analysis pass did not request
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Need {
	/// A runtime function, by registry name
	Function(&'static str),
	/// A libm import, keyed like ffi_imports (`m.pow`) so it never shadows a user function
	MathImport(&'static str),
}

/// Builtins that write their argument and give it back: `puti i` as a statement of a loop body
const OUTPUT_CALLS: [&str; 5] = ["print", "puts", "puti", "putl", "putf"];

/// Builtins that round a float to an exact Int
const ROUNDING_FUNCTIONS: [&str; 5] = ["ceil", "floor", "round", "round_half_up", "round_half_even"];

/// 2^63: floats with a magnitude at or beyond it do not fit an i64
const I64_RANGE_LIMIT: f64 = 9223372036854775808.0;
/// The int scratch local that holds the left operand of `and`/`or` between its test and the branch that returns it
const LOGICAL_SCRATCH: u32 = 2;

/// ffi_imports key of libm's pow for float powers; the `m.` prefix keeps it apart from a user function named pow
const LIBM_POW: &str = "m.pow";

/// Traps of the exact runtime with the message a user should read instead of a wasm backtrace
const EXACT_TRAP_MESSAGES: [(&str, &str); 2] = [
	("int_shift_", "a shift needs an integer and a count from 0 to 65536"),
	("exact_pow", "an exact power needs an integer exponent"),
];

/// An index out of range in a program with an exclusive range is often an end the writer meant to include
/// The exported global where a program leaves the value a runtime error is about (the subject of a missed switch)
pub const TRAP_DETAIL: &str = "trap_detail";
/// How a trapped run carries that value into its error: `trap detail: <value>`
pub const TRAP_DETAIL_PREFIX: &str = "trap detail: ";
const INDEX_OUT_OF_RANGE: &str = "index out of range";
const RANGE_END_HINT: &str = "hint: `..` excludes the end; `...` or `to` include it";

const TERNARY_BRANCHES: &str = "`condition ? then : else`";
const IF_THEN: &str = "`if condition then ...`";

/// The condition and the then-branch of `if condition then branch`
/// The call `join(list, separator)`: the items' text forms with the separator between them
pub(super) fn join_call(list: Node, separator: &str) -> Node {
	Node::List(vec![Node::Symbol("join".to_string()), list, Node::Text(separator.to_string())], Bracket::Round, Separator::None)
}

pub(super) fn joined_text(items: &[Node], separator: &str) -> Node {
	join_call(Node::List(items.to_vec(), Bracket::Square, Separator::Space), separator)
}

fn if_then_parts(node: &Node) -> Option<(&Node, &Node)> {
	let Node::Key(if_condition, Op::Then, then_branch) = node.drop_meta() else { return None };
	let Node::Key(_, Op::If, condition) = if_condition.drop_meta() else { return None };
	Some((condition, then_branch))
}

pub use equality::{IS_TRUTHY, VALUES_EQUAL};
pub use config::{EmitterConfig, EmitterConfigBuilder};
pub use import_manager::ImportManager;
pub use string_table::StringTable;
pub use type_manager::TypeManager;

use crate::analyzer::{analyze_required_functions, captured_variables, kind_with_article, param_kind, collect_all_types, collect_variables, extract_ffi_imports, extract_user_functions, infer_type, type_word_kind, Scope};
use crate::context::{Context, UserFunctionDef};
use crate::local::Local;
use crate::extensions::numbers::Number;
use crate::function::{Function as FuncDef, Signature};
#[cfg(feature = "native")]
use crate::gc_traits::GcObject as ErgonomicGcObject;
use crate::node::{Bracket, Node, Separator};
use crate::normalize::hints as norm;
use crate::operators::{is_function_keyword, op_to_code, Op};
use crate::type_kinds::{any_heap_type, field_def_to_val_type, FieldDef, Kind, RawFieldValue, TypeDef, TypeRegistry, KIND_MASK};
#[cfg(feature = "native")]
use crate::util::gc_engine;
#[cfg(feature = "native")]
use crate::wasm_reader::read_bytes;
use crate::wasp_parser::WaspParser;
use log::{trace, warn};
use std::collections::HashMap;
use std::thread::scope;
use wasm_encoder::*;
#[cfg(feature = "validate")]
use wasmparser::{Validator, WasmFeatures};
use Instruction::I32Const;
use StorageType::Val;
use ValType::Ref;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArithmeticWrap {
	Int,
	Float,
}



/// Compact 3-field Node struct for WASM GC:
/// ```wat
/// (type $String (struct (field $ptr i32) (field $len i32)))
/// (type $Node (struct
///   (field $kind i64)              ;; Type tag (0=Empty,1=Int,2=Float,3=Text,...)
///   (field $data (ref null any))   ;; Payload: i31ref, (ref $String), (ref $Node), boxed numbers
///   (field $value (ref null $Node)) ;; Child/value node (for Key, Pair, List, etc.)
/// ))
/// ```
pub struct WasmGcEmitter {
	module: Module,
	functions: FunctionSection,
	code: CodeSection,
	exports: ExportSection,
	names: NameSection,
	memory: MemorySection,
	globals: GlobalSection,
	tags: TagSection,
	guards_errors: bool, // the program has a `try` that catches runtime errors
	error_catching: Option<try_guard::ErrorCatching>, // the tag and globals of that `try`
	extra_global_names: Vec<(u32, &'static str)>,

	// Configuration
	config: EmitterConfig,

	// Managers
	pub(crate) type_manager: TypeManager,
	import_manager: ImportManager,
	string_table: StringTable,

	// Function and global indices
	next_func_idx: u32,
	next_global_idx: u32,
	next_temp_local: u32,

	// Compilation context
	pub(crate) ctx: Context, // module scope: globals, functions, types, etc.

	scope: Scope, // current function scope

	// Unbounded Int (big_int.rs)
	int_scratch: u32,      // first of INT_SCRATCH_LOCALS i64 locals in the current function
	wrapping_ints: bool,   // inside `expr as i64`: machine arithmetic
	int_heap_global: u32,  // $BigInts heap that handles index
	int_count_global: u32, // used slots in that heap
	returns_node: bool, // the function being compiled returns a Node (main does), so `return x` returns x's Node
	returned_tuple: Vec<Kind>, // the value kinds of the tuple function being compiled (`return a, b`), else empty
	tuple_packers: HashMap<String, u32>, // tuple function → its `f$list` packer (tuple_emitter.rs)
	text_heap_global: Option<u32>, // bump pointer for texts built at runtime, in memory grown past the string table
	// Needs the analyzer could not foresee (they depend on inferred types); emission reruns with them
	discovered_needs: std::collections::HashSet<Need>,
	type_errors: Vec<String>,
	loop_labels: Vec<loop_control::LoopLabels>, // enclosing loops of the code being emitted, innermost last
	typed_lists: HashMap<String, list_dispatch::TypedList>, // list variables of the body being emitted held as typed arrays
	compare_witness: Option<witness::WitnessTable>, // runtime dispatch of Comparable to user types (witness.rs)
	closures: closures::ClosureTypes,
}

impl Default for WasmGcEmitter {
	fn default() -> Self {
		Self::new()
	}
}

impl WasmGcEmitter {
	pub fn new() -> Self {
		WasmGcEmitter {
			module: Module::new(),
			functions: FunctionSection::new(),
			code: CodeSection::new(),
			exports: ExportSection::new(),
			names: NameSection::new(),
			memory: MemorySection::new(),
			globals: GlobalSection::new(),
			tags: TagSection::new(),
			guards_errors: false,
			error_catching: None,
			extra_global_names: Vec::new(),
			config: EmitterConfig::default(),
			type_manager: TypeManager::new(),
			import_manager: ImportManager::new(),
			string_table: StringTable::new(),
			next_func_idx: 0,
			next_global_idx: 0,
			next_temp_local: 0,
			ctx: Context::new(),
			scope: Default::default(),
			int_scratch: 0,
			wrapping_ints: false,
			int_heap_global: 0,
			int_count_global: 0,
			returns_node: true,
			returned_tuple: vec![],
			tuple_packers: HashMap::new(),
			text_heap_global: None,
			discovered_needs: Default::default(),
			type_errors: Vec::new(),
			loop_labels: Vec::new(),
			typed_lists: HashMap::new(),
			compare_witness: None,
			closures: closures::ClosureTypes::default(),
		}
	}

	/// Enable/disable emitting Kind globals for documentation
	pub fn set_emit_kind_globals(&mut self, enabled: bool) {
		self.config.emit_kind_globals = enabled;
	}

	pub fn set_tree_shaking(&mut self, enabled: bool) {
		self.config.emit_all_functions = !enabled;
	}

	/// Enable/disable host function imports (fetch, run)
	pub fn set_host_imports(&mut self, enabled: bool) {
		self.config.emit_host_imports = enabled;
	}

	/// Enable/disable WASI imports (fd_write for puts, puti, etc.)
	pub fn set_wasi_imports(&mut self, enabled: bool) {
		self.config.emit_wasi_imports = enabled;
	}

	/// Enable/disable FFI imports (libc, libm functions)
	pub fn set_ffi_imports(&mut self, enabled: bool) {
		self.config.emit_ffi_imports = enabled;
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Type management helpers (delegate to type_manager)
	// ═══════════════════════════════════════════════════════════════════════════

	/// Get a RefType for Node with specified nullability
	fn node_ref(&self, nullable: bool) -> RefType {
		self.type_manager.node_ref(nullable)
	}

	/// Emit user-defined struct types from TypeRegistry
	pub fn emit_user_types(&mut self, registry: &TypeRegistry) {
		self.type_manager.emit_user_types(registry);
		// Sync user type indices back to context
		for type_def in registry.types() {
			if let Some(idx) = self.type_manager.get_user_type_idx(&type_def.name) {
				self.ctx.user_type_indices.insert(type_def.name.clone(), idx);
			}
		}
	}

	/// Get the WASM type index for a user-defined type
	pub fn get_user_type_idx(&self, name: &str) -> Option<u32> {
		self.type_manager.get_user_type_idx(name)
	}

	/// Emit core GC types (String, Node, i64box, f64box)
	fn emit_gc_types(&mut self) {
		self.type_manager.emit_gc_types();
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Import and FFI helpers
	// ═══════════════════════════════════════════════════════════════════════════

	/// Get FFI function call index by name
	fn ffi_func_index(&self, name: &str) -> Option<u32> {
		let ffi_name = format!("ffi_{}", name);
		self.ctx.func_registry.get(&ffi_name).map(|f| f.call_index as u32)
	}

	/// Register a code function
	fn register_func(&mut self, name: &'static str) -> u32 {
		let func = FuncDef::builtin(name);
		let idx = self.ctx.func_registry.register(func);
		self.next_func_idx = self.ctx.func_registry.import_count() + self.ctx.func_registry.code_count();
		idx
	}

	/// Get function call index by name
	fn func_index(&self, name: &str) -> u32 {
		// First check user functions
		if let Some(user_fn) = self.ctx.user_functions.get(name) {
			if let Some(idx) = user_fn.func_index {
				return idx;
			}
		}
		// Then check registry (builtins/imports)
		self.ctx.func_registry
			.get(name)
			.map(|f| f.call_index as u32)
			.unwrap_or_else(|| panic!("Unknown function: {}", name))
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// User-defined function compilation (extraction done in analyzer)
	// ═══════════════════════════════════════════════════════════════════════════

	/// Compile all extracted user functions to WASM
	/// Pre-allocate strings from user function bodies before compiling
	fn collect_user_function_strings(&mut self) {
		let bodies: Vec<Box<Node>> = self.ctx.user_functions.values()
			.map(|f| f.body.clone())
			.collect();
		for body in bodies {
			self.collect_and_allocate_strings(&body);
		}
	}

	/// Give every outer variable a function reads a global, set from the variable where the function is defined
	fn allocate_closure_captures(&mut self, program: &Node) {
		let mut outer = Scope::new();
		collect_variables(program, &mut outer);
		let functions: Vec<UserFunctionDef> = self.ctx.user_functions.values().cloned().collect();
		for function in functions {
			let captured: Vec<(String, Kind)> = captured_variables(&function, &outer)
				.into_iter()
				.filter(|(name, _)| !self.ctx.user_functions.contains_key(name))
				.collect();
			let captures = captured.into_iter()
				.map(|(name, kind)| (name, (self.declare_mutable_global(kind), kind)))
				.collect();
			self.ctx.captures.insert(function.name, captures);
		}
	}

	/// At a function definition: snapshot the captured variables, so later reassignment is not seen by the function
	pub(super) fn emit_closure_capture(&mut self, func: &mut Function, function_name: &str) {
		let captures = self.ctx.captures.get(function_name).cloned().unwrap_or_default();
		for (name, (global, kind)) in captures {
			let stored_alike = |local: &&Local| self.storage_type(local.kind) == self.storage_type(kind);
			if let Some(local) = self.scope.lookup(&name).filter(stored_alike) {
				func.instruction(&Instruction::LocalGet(local.position));
				func.instruction(&Instruction::GlobalSet(global));
			}
		}
	}

	fn compile_user_functions(&mut self) {
		// Clone the function names to avoid borrow issues
		let func_names: Vec<String> = self.ctx.user_functions.keys().cloned().collect();

		// PASS 1: Register all function signatures and indices
		// This allows forward references (e.g., is_prime can call check before check is compiled)
		for name in &func_names {
			self.register_user_function_signature(name);
		}
		self.register_closure_entries();

		// PASS 2: Compile all function bodies
		for name in func_names {
			if self.is_closure_call(&name) {
				self.compile_closure_call(&name);
			} else {
				self.compile_user_function_body(&name);
			}
		}
		self.compile_closure_entries();
	}

	/// Register a user function's signature and assign it an index (PASS 1)
	fn register_user_function_signature(&mut self, name: &str) {
		let user_fn = self.ctx.user_functions.get(name).unwrap().clone();
		let returns_node = user_fn.return_kind.is_ref();  // Text, Symbol, List, etc. return Node refs

		// Create function type: (params...) -> i64 or (ref $Node) depending on return type
		let func_type_idx = self.type_manager.types().len();
		let param_types: Vec<ValType> = user_fn.params.iter().map(|param| self.storage_type(param_kind(param))).collect();
		let result_types = if !user_fn.tuple_kinds.is_empty() {
			self.tuple_result_types(&user_fn.tuple_kinds)
		} else if returns_node {
			vec![Ref(self.node_ref(false))]
		} else {
			vec![self.storage_type(user_fn.return_kind)]
		};
		self.type_manager.types_mut().ty().function(param_types, result_types);

		// Register function in function section
		self.functions.function(func_type_idx);
		let func_idx = self.next_func_idx;
		self.next_func_idx += 1;
		if !user_fn.tuple_kinds.is_empty() {
			self.register_tuple_packer(name, &user_fn.tuple_kinds);
		}

		// Store the function index and whether it returns a Node
		if let Some(fn_def) = self.ctx.user_functions.get_mut(name) {
			fn_def.func_index = Some(func_idx);
		}
	}

	/// Compile a user function's body (PASS 2)
	fn compile_user_function_body(&mut self, name: &str) {
		let user_fn = self.ctx.user_functions.get(name).unwrap().clone();
		let returns_node = user_fn.return_kind.is_ref();  // Text, Symbol, List, etc. return Node refs

		// Create function scope with parameters
		let function_scope = Scope::with_function_kinds(self.user_function_kinds());
		let saved_scope = std::mem::replace(&mut self.scope, function_scope);
		// declared globals are changed in place, never shadowed by a local of the same name
		self.scope.globals = self.ctx.declared_globals.clone();
		for param in user_fn.params.iter() {
			self.scope.define(param.name.clone(), None, param_kind(param));
		}

		// Collect any additional variables in the body, and the temp locals its loops need
		let temp_locals = collect_variables(&user_fn.body, &mut self.scope);
		let typed_lists = self.find_typed_lists(&user_fn.body);
		let saved_typed_lists = std::mem::replace(&mut self.typed_lists, typed_lists);

		// Declare locals (parameters are already accounted for); temps follow the variables, as in main
		let num_params = user_fn.params.len() as u32;
		let num_locals = self.scope.local_count();

		let saved_scratch = std::mem::replace(&mut self.int_scratch, num_locals + temp_locals);
		let saved_temp_local = std::mem::replace(&mut self.next_temp_local, num_locals);
		let saved_returns_node = std::mem::replace(&mut self.returns_node, returns_node);
		let saved_loop_labels = self.take_loop_labels();
		let mut locals = self.local_declarations(num_params as usize);
		if temp_locals > 0 {
			locals.push((temp_locals, ValType::I64));
		}
		locals.push((big_int::INT_SCRATCH_LOCALS, ValType::I64));
		locals.push((1, Ref(self.node_ref(true)))); // node_scratch
		let mut func = Function::new(locals);

		// Captured variables read their definition-time globals
		let captures = self.ctx.captures.get(name).cloned().unwrap_or_default();
		let shadowed: Vec<_> = captures.iter()
			.map(|(variable, global)| (variable.clone(), self.ctx.user_globals.insert(variable.clone(), *global)))
			.collect();

		let saved_returned_tuple = std::mem::replace(&mut self.returned_tuple, user_fn.tuple_kinds.clone());
		// Compile the function body - use node instructions for Node-returning functions
		if !user_fn.tuple_kinds.is_empty() {
			// every path ends in `return a, b` (tuples::check_definition): the body's own value is never reached
			self.emit_node_instructions(&mut func, &user_fn.body);
			func.instruction(&Instruction::Drop);
			func.instruction(&Instruction::Unreachable);
		} else if returns_node {
			self.emit_node_instructions(&mut func, &user_fn.body);
		} else {
			self.emit_value_of_kind(&mut func, &user_fn.body, user_fn.return_kind);
		}
		func.instruction(&Instruction::End);

		for (variable, global) in shadowed {
			match global {
				Some(global) => self.ctx.user_globals.insert(variable, global),
				None => self.ctx.user_globals.remove(&variable),
			};
		}

		// Add to code section
		self.code.function(&func);
		if !user_fn.tuple_kinds.is_empty() {
			self.compile_tuple_packer(&user_fn.tuple_kinds);
		}
		self.returned_tuple = saved_returned_tuple;

		// Restore scope
		self.scope = saved_scope;
		self.int_scratch = saved_scratch;
		self.next_temp_local = saved_temp_local;
		self.returns_node = saved_returns_node;
		self.restore_loop_labels(saved_loop_labels);
		self.typed_lists = saved_typed_lists;

		// Export the function (get func_idx from the stored function definition)
		let func_idx = self.ctx.user_functions.get(name).unwrap().func_index.unwrap();
		self.exports.export(name, ExportKind::Func, func_idx);
	}

	/// Emit a call to a user-defined function (returns Node)
	fn emit_user_function_call(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let Some(user_fn) = self.ctx.user_functions.get(fn_name).cloned() else {
			self.emit_type_error(func, format!("undefined function: {fn_name}"));
			return;
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		if user_fn.return_kind.is_float() {
			self.emit_call(func, "new_float");
		} else if !returns_node {
			self.emit_call(func, "new_int");
		}
	}

	/// Result kinds of the user functions, and of `floor(x)`, `round(x)`…, which build an Int (emit_introspection_fn)
	/// unless libm's f64 version is imported
	fn user_function_kinds(&self) -> HashMap<String, Kind> {
		let rounding = ROUNDING_FUNCTIONS.iter().filter(|name| !self.ctx.ffi_imports.contains_key(**name)).map(|name| (name.to_string(), Kind::Int));
		let tuple_values = self.ctx.user_functions.iter().flat_map(|(name, function)| {
			function.tuple_kinds.iter().enumerate().map(|(index, kind)| (crate::tuples::element_key(name, index), *kind))
		});
		rounding.chain(self.ctx.user_functions.iter().map(|(name, function)| (name.clone(), function.return_kind))).chain(tuple_values).collect()
	}

	/// Emit a call to a user-defined function whose result is needed as f64
	fn emit_user_function_call_float(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let user_fn = self.ctx.user_functions[fn_name].clone();
		if user_fn.return_kind.is_float() {
			self.emit_user_function_call_inner(func, &user_fn, args);
		} else {
			self.emit_user_function_call_numeric(func, fn_name, args);
			self.emit_int_to_f64(func, None);
		}
	}

	/// Emit a call to a user-defined function (returns raw i64)
	/// Note: For Node-returning functions, this extracts the integer value from the Node
	fn emit_user_function_call_numeric(&mut self, func: &mut Function, fn_name: &str, args: &[Node]) {
		let Some(user_fn) = self.ctx.user_functions.get(fn_name).cloned() else {
			self.emit_type_error(func, format!("undefined function: {fn_name}"));
			return;
		};
		let returns_node = user_fn.return_kind.is_ref();

		// Emit arguments and call
		self.emit_user_function_call_inner(func, &user_fn, args);

		if user_fn.return_kind.is_float() {
			self.emit_float_in_exact_context(func, fn_name);
		} else if returns_node {
			self.emit_call(func, "get_int_value");
		}
	}

	/// Inner helper for emitting user function calls; a tuple function's values are packed into a list
	fn emit_user_function_call_inner(&mut self, func: &mut Function, user_fn: &UserFunctionDef, args: &[Node]) {
		self.emit_user_function_values(func, user_fn, args);
		if !user_fn.tuple_kinds.is_empty() && user_fn.func_index.is_some() {
			self.emit_pack_tuple(func, &user_fn.name);
		}
	}

	/// Arguments and the call: the function's own results on the stack, several for a tuple function
	fn emit_user_function_values(&mut self, func: &mut Function, user_fn: &UserFunctionDef, args: &[Node]) {
		let Some(func_index) = user_fn.func_index else {
			self.emit_type_error(func, format!("function {} is used before it is compiled", user_fn.name));
			return;
		};

		// Emit arguments; a missing argument evaluates its default anew at every call
		for (i, param) in user_fn.params.iter().enumerate() {
			let Some(argument) = args.get(i).or(param.default.as_ref()) else {
				self.emit_type_error(func, format!("{} needs a value for parameter {} (it has no default)", user_fn.name, param.name));
				return;
			};
			let expected = param_kind(param);
			let given = self.get_type(argument);
			// a declared list (`xs:list`, `xs:ints`) refuses a number or text loudly (wiki/Footguns.md "Type annotations not enforced loudly")
			let declared_list = expected == Kind::List && param.annotation.is_some();
			let refused: &[Kind] = if declared_list { &[Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint] } else { &[Kind::List, Kind::Text] };
			if (!expected.is_ref() || declared_list) && refused.contains(&given) && given != expected {
				let message = format!("{} needs {} for parameter {}, got {} ({})", user_fn.name, kind_with_article(expected), param.name, argument.serialize(), kind_with_article(given));
				self.emit_type_error(func, message);
				return;
			}
			self.emit_value_of_kind(func, argument, expected);
			if expected == Kind::Text && given == Kind::Codepoint { // f("a") for f(t:text): "a" lexes as a character
				self.emit_call(func, text_builtins::TEXT_OF);
			}
		}

		// Call the function
		func.instruction(&Instruction::Call(func_index));
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Helper methods for clean, DRY code
	// ═══════════════════════════════════════════════════════════════════════════

	/// Emit string lookup from table and call constructor
	fn emit_string_call(&mut self, func: &mut Function, s: &str, constructor: &'static str) {
		let (ptr, len) = self.allocate_string(s); // a text the pre-scan did not see is added to the table, never read from offset 0
		func.instruction(&I32Const(ptr as i32));
		func.instruction(&I32Const(len as i32));
		self.emit_call(func, constructor);
	}

	/// Append String struct field names (ptr, len) to an IndirectNameMap
	fn append_string_field_names(type_field_names: &mut IndirectNameMap, type_idx: u32) {
		let mut names = NameMap::new();
		names.append(0, "ptr");
		names.append(1, "len");
		type_field_names.append(type_idx, &names);
	}

	/// Infer the Kind for an expression
	/// Extends analyzer::infer_type with user_globals knowledge
	fn get_type(&self, node: &Node) -> Kind {
		let node = node.drop_meta();
		match node {
			// Check user_globals for symbols
			Node::Symbol(name) => {
				if let Some(&(_, kind)) = self.ctx.user_globals.get(name) {
					return kind;
				}
				// Fall back to scope lookup
				if let Some(local) = self.scope.lookup(name) {
					return local.kind;
				}
				Kind::Symbol
			}
			// Call of a user function, also braceless: `f 3`
			Node::List(items, _, _) if items.len() >= 2 && matches!(items[0].drop_meta(), Node::Symbol(name) if self.ctx.user_functions.contains_key(name)) => {
				let Node::Symbol(name) = items[0].drop_meta() else { unreachable!() };
				self.ctx.user_functions[name].return_kind
			}
			// Arithmetic: recursively check operands with our get_type
			Node::Key(left, op, right) if op.is_arithmetic() => {
				self.arithmetic_type(left, op, right)
			}
			// For other nodes, use analyzer's infer_type
			_ => infer_type(node, &self.scope),
		}
	}

	/// Check if an expression is numeric (int, float, or bool)
	fn is_numeric(&self, node: &Node) -> bool {
		let node = node.drop_meta();
		match node {
			Node::Number(_) | Node::True | Node::False => true,
			Node::Key(_left, op, _right) if op.is_arithmetic() || op.is_shift() || op.is_comparison() => true,
			Node::Key(left, op, right) if op.is_logical() => self.is_numeric(left) && self.is_numeric(right),
			// `not x` is always 1 or 0
			Node::Key(left, Op::Not, _) if matches!(left.drop_meta(), Node::Empty) => true,
			Node::Key(_, Op::Define | Op::Assign, right) => self.is_numeric(right),
			// grouping: `(1==2)` is the number it computes
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.is_numeric(&items[0]),
			Node::Symbol(name) => {
				// Check if symbol is a known numeric variable
				if let Some(local) = self.scope.lookup(name) {
					matches!(local.kind, Kind::Int | Kind::Float)
				} else if let Some(&(_, kind)) = self.ctx.user_globals.get(name) {
					matches!(kind, Kind::Int | Kind::Float)
				} else {
					false
				}
			}
			_ => false,  // Empty, Text, Char, List, etc. are not numeric
		}
	}

	/// Emit comparison operator for f64 (result is i32, extended to i64)
	fn emit_float_comparison(&self, func: &mut Function, op: &Op) {
		let cmp = match op {
			Op::Eq => Instruction::F64Eq,
			Op::Ne => Instruction::F64Ne,
			Op::Lt => Instruction::F64Lt,
			Op::Gt => Instruction::F64Gt,
			Op::Le => Instruction::F64Le,
			Op::Ge => Instruction::F64Ge,
			_ => unreachable!("Not a comparison op: {:?}", op),
		};
		func.instruction(&cmp);
		func.instruction(&Instruction::I64ExtendI32U);
	}

	fn should_emit_function(&self, name: &str) -> bool {
		self.config.emit_all_functions || self.ctx.required_functions.contains(name)
	}

	/// Generate all type definitions and functions
	pub fn emit(&mut self) {
		self.memory.memory(MemoryType {
			minimum: 1,
			maximum: None,
			memory64: false,
			shared: false,
			page_size_log2: None,
		});
		self.exports.export("memory", ExportKind::Memory, 0);
		// Host imports must come before GC types (imports section comes before types in WASM)
		self.import_manager
			.emit_imports(&self.config, &mut self.type_manager, &mut self.ctx);
		self.next_func_idx = self.import_manager.import_count();
		self.type_manager.emit_gc_types();
		// Emit user-defined struct types from type_registry (must come after gc_types, before functions)
		self.emit_registered_user_types();
		if self.config.emit_kind_globals {
			self.emit_kind_globals();
		}
		self.emit_constructors();
		// Emit constructors for registered user types
		self.emit_registered_user_type_constructors();
	}

	/// Emit user types from internal type_registry
	fn emit_registered_user_types(&mut self) {
		let types: Vec<TypeDef> = self.ctx.type_registry.types().to_vec();
		for type_def in &types {
			self.type_manager.emit_single_user_type(type_def);
			// Update context with type indices
			if let Some(idx) = self.type_manager.get_user_type_idx(&type_def.name) {
				self.ctx.user_type_indices.insert(type_def.name.clone(), idx);
			}
		}
	}

	/// Emit constructors for registered user types
	fn emit_registered_user_type_constructors(&mut self) {
		let types: Vec<TypeDef> = self.ctx.type_registry.types().to_vec();
		for type_def in &types {
			self.emit_user_type_constructor(type_def);
		}
	}

	/// Emit Kind constants as immutable globals
	/// JIT compilers constant-fold these, so global.get is equally fast
	fn emit_kind_globals(&mut self) {
		let tags = [
			("kind_empty", Kind::Empty),
			("kind_int", Kind::Int),
			("kind_float", Kind::Float),
			("kind_text", Kind::Text),
			("kind_codepoint", Kind::Codepoint),
			("kind_symbol", Kind::Symbol),
			("kind_key", Kind::Key),
			("kind_block", Kind::Block),
			("kind_list", Kind::List),
			("kind_data", Kind::Data),
			("kind_meta", Kind::Meta),
			("kind_error", Kind::Error),
			("kind_type", Kind::TypeDef),
		];

		for (name, tag) in tags {
			self.globals.global(
				GlobalType {
					val_type: ValType::I64,
					mutable: false,
					shared: false,
				},
				&ConstExpr::i64_const(tag as i64),
			);
			self.exports.export(name, ExportKind::Global, self.next_global_idx);
			self.ctx.kind_global_indices.insert(tag, self.next_global_idx);
			self.next_global_idx += 1;
		}
	}

	/// Emit instruction to get a Kind kind value
	fn emit_kind(&self, func: &mut Function, tag: Kind) {
		if let Some(idx) = self.ctx.kind_global_indices.get(&tag) {
			func.instruction(&Instruction::GlobalGet(*idx));
		} else {
			func.instruction(&Instruction::I64Const(tag as i64));
		}
	}

	pub fn emit_for_node(&mut self, node: &Node) {
		let lowered;
		let node = if crate::real::mentions_real(node) {
			lowered = crate::real::lower(node.clone());
			&lowered
		} else {
			node
		};
		self.config.emit_all_functions = false;
		// First pass: register all types (forward reference support)
		collect_all_types(&mut self.ctx.type_registry, node);
		// Analyze: Extract FFI imports, user functions, and required functions
		extract_ffi_imports(&mut self.ctx, node);
		extract_user_functions(&mut self.ctx, node);
		crate::analyzer::extract_host_words(&mut self.ctx, node);
		self.type_errors.extend(self.ctx.parameter_conflicts.drain(..));
		self.scope.function_kinds = self.user_function_kinds();
		self.derive_imports_from_effects(node);
		analyze_required_functions(&mut self.ctx, node);
		self.ctx.required_functions.extend(self.discovered_needs.iter().filter_map(|need| match need {
			Need::Function(name) => Some(*name),
			Need::MathImport(_) => None,
		}));
		text_builtins::add_dependencies(&mut self.ctx.required_functions);
		self.guards_errors = try_guard::guards_errors(node);
		let len = self.ctx.required_functions.len();
		trace!(
			"tree-shaking: {} functions required: {:?}",
			len,
			self.ctx.required_functions
		);
		self.emit();
		// Pre-allocate strings from user function bodies before compiling
		self.collect_user_function_strings();
		self.allocate_declared_globals(node);
		self.allocate_closure_captures(node);
		// Compile user functions after builtin infrastructure is set up
		self.compile_user_functions();
		self.emit_compare_dispatcher();
		self.emit_node_main(node);
		if self.discovered_needs.iter().any(|need| !self.is_provided(need)) {
			let mut rerun = Self::new();
			rerun.config = self.config.clone();
			rerun.discovered_needs = std::mem::take(&mut self.discovered_needs);
			crate::normalize::without_hints(|| rerun.emit_for_node(node)); // the first pass already hinted the same program
			*self = rerun;
		}
	}

	fn is_provided(&self, need: &Need) -> bool {
		match need {
			Need::Function(name) => self.ctx.required_functions.contains(name),
			Need::MathImport(key) => self.ctx.ffi_imports.contains_key(*key),
		}
	}

	/// The first type error found while emitting; the module must not run then
	pub fn type_error(&self) -> Option<Node> {
		self.type_errors.first().or(self.type_manager.type_errors.first()).map(|message| crate::node::error(message))
	}

	/// A value used as a number that has none (ø, a list, a type…): an error value at its source position, not a panic
	fn emit_not_a_number(&mut self, func: &mut Function, located: &Node, value: &Node) {
		let diagnostic = crate::diagnostic::Diagnostic::at(located, format!("cannot extract a numeric value from {}", value.serialize()));
		self.emit_type_error(func, diagnostic.to_string());
	}

	/// A construct whose shape the emitter cannot handle: an error value at its source position, not a panic
	fn emit_malformed(&mut self, func: &mut Function, located: &Node, expected: &str) {
		let diagnostic = crate::diagnostic::Diagnostic::at(located, format!("expected {expected}, got {}", located.serialize()));
		self.emit_type_error(func, diagnostic.to_string());
	}

	fn emit_type_error(&mut self, func: &mut Function, message: String) {
		self.type_errors.push(message);
		func.instruction(&Instruction::Unreachable);
	}

	/// Does the node read a variable (key names and other symbols of a data literal are not variables)
	fn mentions_variable(&self, node: &Node) -> bool {
		let mut found = false;
		node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if self.scope.lookup(name).is_some()));
		found
	}

	/// A name that is no variable, global, function or `$n` parameter here
	fn is_unbound(&self, name: &str) -> bool {
		!name.starts_with('$') && self.scope.lookup(name).is_none() && !self.ctx.user_globals.contains_key(name) && !self.ctx.user_functions.contains_key(name)
	}

	fn emit_undefined_variable(&mut self, func: &mut Function, name: &str) {
		self.emit_type_error(func, format!("undefined variable: {name}"));
	}

	/// The local slot of a defined variable; an undefined one is an error value at the use
	fn defined_local_position(&mut self, func: &mut Function, name: &str) -> Option<u32> {
		let position = self.scope.lookup(name).map(|local| local.position);
		if position.is_none() {
			self.emit_undefined_variable(func, name);
		}
		position
	}

	/// Imports follow resolved calls: only called FFI functions, WASI/host only if called.
	/// Explicitly enabled imports stay enabled.
	fn derive_imports_from_effects(&mut self, node: &Node) {
		use crate::effects::{Capability, EffectReport};
		let effects = EffectReport::of(node);
		self.ctx.ffi_imports.retain(|name, _| effects.calls_external(name));
		for need in &self.discovered_needs {
			if let Need::MathImport(key) = need {
				let function = key.trim_start_matches("m.");
				self.ctx.ffi_imports.extend(crate::ffi::get_ffi_signature(function).map(|signature| (key.to_string(), signature)));
			}
		}
		self.config.emit_ffi_imports = !self.ctx.ffi_imports.is_empty();
		self.config.emit_wasi_imports |= effects.needs(Capability::Wasi);
		self.config.emit_host_imports |= effects.needs(Capability::Host);
	}

	/// Import modules this module declares, known after `emit_for_node`
	pub fn imports(&self, capability: crate::effects::Capability) -> bool {
		use crate::effects::Capability::*;
		match capability {
			Host => self.config.emit_host_imports,
			Wasi => self.config.emit_wasi_imports,
			Ffi => self.config.emit_ffi_imports,
			Sql | Process => false, // never imported: eval refuses such modules before emission
		}
	}

	/// Emit with user-defined types from a TypeRegistry
	/// Order: memory, gc_types, user_types, kind_globals, constructors, user_constructors
	pub fn emit_with_types(&mut self, registry: &TypeRegistry) {
		// Memory
		self.memory.memory(MemoryType {
			minimum: 1,
			maximum: None,
			memory64: false,
			shared: false,
			page_size_log2: None,
		});
		self.exports.export("memory", ExportKind::Memory, 0);

		// Core GC types (String, Node, i64box, f64box)
		self.emit_gc_types();

		// User-defined struct types (before any functions!)
		self.emit_user_types(registry);

		// Kind globals
		if self.config.emit_kind_globals {
			self.emit_kind_globals();
		}

		// Core Node constructors
		self.emit_constructors();

		// User type constructors
		self.emit_user_type_constructors(registry);
	}

	/// Emit constructor functions for user-defined types
	fn emit_user_type_constructors(&mut self, registry: &TypeRegistry) {
		for type_def in registry.types() {
			self.emit_user_type_constructor(type_def);
		}
	}

	/// Emit a constructor function for a single user type: new_TypeName(fields...) -> ref $TypeName
	fn emit_user_type_constructor(&mut self, type_def: &TypeDef) {
		let type_idx = match self.ctx.user_type_indices.get(&type_def.name) {
			Some(idx) => *idx,
			None => return,
		};

		let type_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(type_idx),
		};

		// Build parameter types
		let params: Vec<ValType> = type_def.fields.iter().map(|f| field_def_to_val_type(f, self)).collect();

		// Function type: (params...) -> (ref $TypeName)
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(params.clone(), vec![Ref(type_ref)]);
		self.functions.function(func_type);

		// Function body: get all params, struct.new
		let mut func = Function::new(vec![]);
		for i in 0..type_def.fields.len() {
			func.instruction(&Instruction::LocalGet(i as u32));
		}
		func.instruction(&Instruction::StructNew(type_idx));
		func.instruction(&Instruction::End);

		self.code.function(&func);

		// Export as new_TypeName
		let func_name = format!("new_{}", type_def.name);
		// Leak the string to get a 'static str for the export
		let func_name_static: &'static str = Box::leak(func_name.clone().into_boxed_str());
		self.exports
			.export(func_name_static, ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}


	/// Emit constructor functions for the compact Node
	fn emit_constructors(&mut self) {
		// Emit basic Node constructors using macros
		self.emit_int_heap_globals();
		constructors::emit_all_constructors(self);
		self.emit_int_runtime();
		// Emit list and string operation functions
		self.emit_list_ops();
		self.emit_text_of();
		self.emit_equality_ops();
		// Emit helper functions
		self.emit_getters();
		self.emit_text_as_int(); // after the getters: it calls get_int_value
		if self.config.emit_reflection {
			self.emit_reflection();
		}
		self.emit_math_helpers();
		self.emit_text_builtins();
		self.emit_map_get(); // after the text builtins: map keys are compared by text_of
		self.emit_library_ops(); // after the text builtins: the library words call text_of
	}

	fn emit_getters(&mut self) {
		let node_ref = self.node_ref(true);

		// get_kind(node: ref $Node) -> i64
		self.exported_function("get_kind", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: s.type_manager.node_type,
				field_index: 0,
			});
		});

		// get_int_value(node: ref $Node) -> i64
		// Extract integer from Node's data field (i64box); a node that is no Int is the runtime error not_an_int,
		// which a `try` catches, never a raw cast trap
		self.exported_function("get_int_value", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			Self::emit_list(func, &[
				Instruction::LocalGet(0),
				Instruction::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 },
				Instruction::I64Const(KIND_MASK), Instruction::I64And, Instruction::I64Const(Kind::Int as i64), Instruction::I64Ne,
				Instruction::If(BlockType::Empty),
			]);
			s.emit_runtime_error(func, "not_an_int");
			func.instruction(&Instruction::End);
			func.instruction(&Instruction::LocalGet(0)); // Node
			func.instruction(&Instruction::StructGet {
				struct_type_index: s.type_manager.node_type,
				field_index: 1, // data field
			});
			s.emit_int_from_payload(func);
		});

		// get_text_ptr / get_text_len(node: ref $Node) -> i32: the $String of a Text, Symbol or Error, 0 for any other node,
		// so a host without GC field access (JavaScript) can read a result text from memory
		for (name, field_index) in [("get_text_ptr", 0), ("get_text_len", 1)] {
			self.exported_function(name, vec![Ref(node_ref)], vec![ValType::I32], vec![], |s, f| {
				let string = HeapType::Concrete(s.type_manager.string_type);
				s.emit_field(f, 0, 1);
				f.instruction(&Instruction::RefTestNonNull(string));
				f.instruction(&Instruction::If(BlockType::Result(ValType::I32)));
				s.emit_text_field(f, 0, field_index);
				f.instruction(&Instruction::Else);
				f.instruction(&Instruction::I32Const(0));
				f.instruction(&Instruction::End);
			});
		}
	}

	/// Emit math helper functions (i64_pow, etc.)
	fn emit_math_helpers(&mut self) {
		// i64_pow(base: i64, exp: i64) -> i64
		// Computes base^exp using a loop
		if self.should_emit_function("i64_pow") {
			self.exported_function("i64_pow", vec![ValType::I64, ValType::I64], vec![ValType::I64], vec![ValType::I64], |_, func| {
				// Locals: 0=base, 1=exp, 2=result
				// result = 1
				func.instruction(&Instruction::I64Const(1));
				func.instruction(&Instruction::LocalSet(2));

				// block $done
				func.instruction(&Instruction::Block(BlockType::Empty));
				// loop $loop
				func.instruction(&Instruction::Loop(BlockType::Empty));

				// br_if $done (i64.eqz (local.get $exp))
				func.instruction(&Instruction::LocalGet(1)); // exp
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::BrIf(1)); // break to $done

				// result = result * base
				func.instruction(&Instruction::LocalGet(2)); // result
				func.instruction(&Instruction::LocalGet(0)); // base
				func.instruction(&Instruction::I64Mul);
				func.instruction(&Instruction::LocalSet(2));

				// exp = exp - 1
				func.instruction(&Instruction::LocalGet(1)); // exp
				func.instruction(&Instruction::I64Const(1));
				func.instruction(&Instruction::I64Sub);
				func.instruction(&Instruction::LocalSet(1));

				// br $loop
				func.instruction(&Instruction::Br(0));

				// end loop
				func.instruction(&Instruction::End);
				// end block
				func.instruction(&Instruction::End);

				// return result
				func.instruction(&Instruction::LocalGet(2));
			});
		}
	}

	/// Allocate a binary lookup table in linear memory
	fn allocate_bytes(&mut self, name: &str, bytes: &[u8]) -> u32 {
		self.string_table.allocate_bytes(name, bytes)
	}

	/// Allocate a string in linear memory
	fn allocate_string(&mut self, s: &str) -> (u32, u32) {
		self.string_table.allocate(s)
	}

	/// f64 on the stack → exact Int, truncated toward zero: the explicit casts `as int` and `int(x)`, and the
	/// rounding functions. There is no f64 → bignum path, so a NaN or a value beyond i64 fails cleanly.
	fn emit_truncating_cast(&mut self, func: &mut Function) {
		let value_bits = self.scratch(0);
		func.instruction(&Instruction::I64ReinterpretF64);
		func.instruction(&Instruction::LocalSet(value_bits));
		func.instruction(&Instruction::LocalGet(value_bits));
		func.instruction(&Instruction::F64ReinterpretI64);
		func.instruction(&Instruction::F64Abs);
		func.instruction(&Instruction::F64Const(I64_RANGE_LIMIT.into()));
		func.instruction(&Instruction::F64Lt);
		func.instruction(&Instruction::I32Eqz);
		self.emit_fail_if(func, "float_out_of_int_range");
		func.instruction(&Instruction::LocalGet(value_bits));
		func.instruction(&Instruction::F64ReinterpretI64);
		func.instruction(&Instruction::I64TruncF64S);
		self.emit_int_from_machine(func);
	}

	/// A float has no implicit exact value: reading it as an Int is refused instead of truncated
	fn emit_float_in_exact_context(&mut self, func: &mut Function, what: &str) {
		let message = format!("{what} is a float where an exact Int is expected: use it in float arithmetic or truncate with `as int`");
		self.emit_type_error(func, message);
	}

	/// Declarations of the scope's locals in position order, skipping the first `skipped` (the parameters)
	fn local_declarations(&self, skipped: usize) -> Vec<(u32, ValType)> {
		let mut sorted_locals: Vec<_> = self.scope.locals.values().collect();
		sorted_locals.sort_by_key(|local| local.position);
		sorted_locals.into_iter().skip(skipped).map(|local| (1, self.local_storage_type(&local.name, local.kind))).collect()
	}

	/// Emit main function that constructs the node
	pub fn emit_node_main(&mut self, node: &Node) {
		// Pre-pass: collect variables first so scope is populated
		let temp_locals = collect_variables(node, &mut self.scope);
		self.typed_lists = self.find_typed_lists(node);

		// Allocate strings and update Local data pointers
		self.collect_and_allocate_strings(node);
		let var_count = self.scope.local_count();
		self.next_temp_local = var_count; // Temp locals start after variables

		let node_ref = self.node_ref(false);
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(vec![], vec![Ref(node_ref)]);
		self.functions.function(func_type);

		// Build locals list based on variable types
		// Each variable gets its own entry, then temp locals (all i64)
		let mut locals: Vec<(u32, ValType)> = Vec::new();
		locals.extend(self.local_declarations(0));

		// Add temp locals (i64 for now)
		if temp_locals > 0 {
			locals.push((temp_locals, ValType::I64));
		}
		self.int_scratch = var_count + temp_locals;
		locals.push((big_int::INT_SCRATCH_LOCALS, ValType::I64));
		locals.push((1, Ref(self.node_ref(true)))); // node_scratch

		let mut func = Function::new(locals);
		self.emit_witness_installation(&mut func);
		self.emit_node_instructions(&mut func, node);
		func.instruction(&Instruction::End);

		self.code.function(&func);
		self.exports.export("main", ExportKind::Func, self.next_func_idx);
		self.next_func_idx += 1;
	}

	fn collect_and_allocate_strings(&mut self, node: &Node) {
		self.string_table.collect_from_node(node, &mut self.scope);
	}

	fn emit_call(&mut self, func: &mut Function, name: &'static str) {
		if !self.ctx.func_registry.contains(name) && !self.ctx.user_functions.contains_key(name) {
			assert!(!self.ctx.required_functions.contains(name), "Unknown function: {}", name);
			self.discovered_needs.insert(Need::Function(name));
			func.instruction(&Instruction::Unreachable);
			return;
		}
		self.ctx.used_functions.insert(name);
		func.instruction(&Instruction::Call(self.func_index(name)));
	}

	fn emit_node_null(&self, func: &mut Function) {
		func.instruction(&Instruction::RefNull(HeapType::Concrete(self.type_manager.node_type)));
	}


	/// Emit instructions to construct a Node
	fn emit_node_instructions(&mut self, func: &mut Function, node: &Node) {
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_node_instructions) {
			return;
		}
		if let Some((name, fields)) = crate::type_constructor::instance_parts(node) {
			self.emit_default_key(func, name, fields, &Op::None); // an instance is no data: its own op code (D4)
			return;
		}
		if let Some((target, captured)) = crate::closures::as_closure_new(node) {
			self.emit_closure_new(func, target, captured);
			return;
		}
		let node = node.drop_meta();

		match node {
			Node::Empty => {
				self.emit_call(func, "new_empty");
			}
			Node::Number(num) => match num {
				Number::Int(_) | Number::BigInt(_) => {
					self.emit_int_literal(func, &num.to_bigint());
					self.emit_call(func, "new_int");
				}
				Number::Float(f) if !Number::is_exact_decimal(*f) => {
					func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
					self.emit_call(func, "new_float");
				}
				Number::Float(_) | Number::Quotient(..) => {
					self.emit_numeric_value(func, node);
					self.emit_call(func, "new_int");
				}
				_ => {
					self.emit_call(func, "new_empty");
				}
			},
			Node::Text(s) => {
				self.emit_string_call(func, s, "new_text");
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
				self.emit_call(func, "new_codepoint");
			}
			Node::Symbol(s) => {
				if let Some(user_fn) = self.ctx.user_functions.get(s) {
					match user_fn.params.iter().filter(|param| param.default.is_none()).count() {
						0 => self.emit_user_function_call(func, s, &[]),
						count => self.emit_type_error(func, format!("{s} needs {count} argument{}", if count == 1 { "" } else { "s" })),
					}
					return;
				}
				if self.emit_typed_list_as_node(func, s) {
					return;
				}
				// Check if this is a local variable lookup
				if let Some(local) = self.scope.lookup(s) {
					func.instruction(&Instruction::LocalGet(local.position));
					if !local.kind.is_ref() {
						self.emit_primitive_as_node(func, local.kind);
					}
					return;
				}
				// Check if this is a global variable lookup
				if let Some(&(idx, kind)) = self.ctx.user_globals.get(s) {
					func.instruction(&Instruction::GlobalGet(idx));
					if kind.is_ref() {
						func.instruction(&Instruction::RefAsNonNull);
					} else {
						self.emit_primitive_as_node(func, kind);
					}
					return;
				}
				self.emit_string_call(func, s, "new_symbol");
			}
			Node::Key(left, op, right) => {
				self.emit_key_node(func, left, op, right);
			}
			Node::List(items, bracket, separator) => {
				self.emit_list_node(func, items, bracket, separator);
			}
			Node::Data(dada) => {
				self.emit_string_call(func, &dada.type_name, "new_symbol");
			}
			Node::Meta { .. } => {
				self.emit_call(func, "new_empty");
			}
			Node::Error(inner) => {
				// Emit the inner node, but mark as error in kind
				// For now, just emit the inner
				self.emit_node_instructions(func, inner);
			}
			Node::Type { name, body } => {
				// Emit type as a tagged block with name and fields
				self.emit_node_instructions(func, name);
				self.emit_node_instructions(func, body);
				self.emit_call(func, "new_type");
			}
			&Node::False => {
				func.instruction(&Instruction::I64Const(0));
				self.emit_call(func, "new_int");
			}
			&Node::True => {
				func.instruction(&Instruction::I64Const(1));
				self.emit_call(func, "new_int");
			}
		}
	}

	/// Emit arithmetic operation: evaluate operands and apply operator
	fn emit_arithmetic(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
			self.emit_node_instructions(func, &assignment);
			return;
		}
		if op.is_shift() {
			self.emit_shift(func, left, op, right);
			self.emit_call(func, "new_int");
			return;
		}
		if op.is_arithmetic() && self.emit_typed_arithmetic(func, left, op, right) {
			return;
		}
		let use_float = self.should_use_float(left, right, op);

		if self.emit_assign_or_define(func, left, op, right, use_float) {
			if use_float {
				self.emit_call(func, "new_float");
			} else {
				self.emit_call(func, "new_int");
			}
			return;
		}
		if self.emit_inc_dec(func, left, op) {
			self.emit_call(func, "new_int");
			return;
		}
		if self.emit_compound_assign(func, left, op, right, use_float) {
			if use_float {
				self.emit_call(func, "new_float");
			} else {
				self.emit_call(func, "new_int");
			}
			return;
		}

		let wrap = if use_float {
			if self.emit_float_truthy_logical(func, left, op, right) {
				return;
			}
			self.emit_float_binary(func, left, op, right)
		} else {
			if self.emit_int_truthy_logical(func, left, op, right) {
				return;
			}
			self.emit_int_binary(func, left, op, right)
		};

		self.wrap_arithmetic_result(func, wrap);
	}

	fn should_use_float(&self, left: &Node, right: &Node, op: &Op) -> bool {
		self.arithmetic_type(left, op, right).is_float()
	}

	fn emit_assign_or_define(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
		use_float: bool,
	) -> bool {
		if *op != Op::Define && *op != Op::Assign {
			return false;
		}

		// Check for string assignment in WASI mode - skip emit (tracked in Local)
		if self.config.emit_wasi_imports && matches!(right.drop_meta(), Node::Text(_)) {
			// String data stored in Local's data_pointer/data_length, just emit 0
			func.instruction(&Instruction::I64Const(0));
			return true;
		}

		if let Node::Symbol(name) = left.drop_meta() {
			if self.scope.lookup(name).is_none() {
				if let Some(global_kind) = self.emit_global_store(func, name, right) {
					match (global_kind.is_float(), use_float) {
						(true, false) => self.emit_float_in_exact_context(func, name),
						(false, true) => self.emit_int_to_f64(func, None),
						_ => {}
					}
					return true;
				}
			}
		}

		// x:=42 or x=42 → emit value, store to local, return value
		if use_float {
			self.emit_float_value(func, right);
		} else {
			self.emit_numeric_value(func, right);
			self.emit_fits_declared(func, left);
		}
		if let Node::Symbol(name) = left.drop_meta() {
			if let Some(local) = self.scope.lookup(name) {
				func.instruction(&Instruction::LocalTee(local.position));
			} else {
				self.emit_undefined_variable(func, name);
			}
		} else {
			self.emit_malformed(func, left, "a variable to assign to");
		}
		true
	}

	fn emit_inc_dec(&mut self, func: &mut Function, left: &Node, op: &Op) -> bool {
		if *op != Op::Inc && *op != Op::Dec {
			return false;
		}

		// i++ → i = i + 1 (returns new value)
		// i-- → i = i - 1 (returns new value)
		if let Node::Symbol(name) = left.drop_meta() {
			let Some(local_pos) = self.defined_local_position(func, name) else { return true };
			self.emit_int_step(func, local_pos, op);
			self.emit_fits_declared(func, left);
			// Store and return new value
			func.instruction(&Instruction::LocalTee(local_pos));
			true
		} else {
			self.emit_malformed(func, left, "a variable to increment or decrement");
			true
		}
	}

	fn emit_compound_assign(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
		use_float: bool,
	) -> bool {
		if !op.is_compound_assign() {
			return false;
		}

		if self.emit_compound_index_assignment(func, left, op, right) {
			return true;
		}
		// x += y → x = x + y
		if let Node::Symbol(name) = left.drop_meta() {
			let Some(local_pos) = self.defined_local_position(func, name) else { return true };
			let base_op = op.base_op();
			// Get current value of x
			func.instruction(&Instruction::LocalGet(local_pos));
			// Emit y
			if use_float {
				self.emit_float_value(func, right);
			} else {
				self.emit_numeric_value(func, right);
			}
			// Apply base operation
			if use_float {
				self.emit_float_arithmetic(func, &base_op);
			} else {
				self.emit_int_compound_op(func, &base_op, right);
				self.emit_fits_declared(func, left);
			}
			// Store result and leave on stack
			func.instruction(&Instruction::LocalTee(local_pos));
			true
		} else {
			self.emit_malformed(func, left, "a variable to update");
			true
		}
	}

	/// Stack [x, y] → [x op y] for `x op= y` on Ints; `/=` keeps an integer x an integer
	fn emit_int_compound_op(&mut self, func: &mut Function, op: &Op, right: &Node) {
		match op {
			Op::And => {
				func.instruction(&Instruction::I64And);
			}
			Op::Or => {
				func.instruction(&Instruction::I64Or);
			}
			Op::Div => self.emit_call(func, "exact_div_assign"),
			Op::Add | Op::Sub | Op::Mul | Op::Mod | Op::Pow | Op::Xor => {
				let right_range = self.int_range(right);
				self.emit_int_op(func, op, None, right_range);
			}
			_ => self.emit_type_error(func, format!("`{op}=` is not an operator on exact numbers")),
		}
	}

	fn emit_float_truthy_logical(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
	) -> bool {
		if *op != Op::And && *op != Op::Or {
			return false;
		}

		// and: a falsy left is the value, else right; or: a truthy left is the value, else right. Left is evaluated once.
		self.emit_held_left_is_falsy(func, left, true);
		func.instruction(&Instruction::If(BlockType::Result(ValType::F64)));
		self.emit_logical_branch(func, op == &Op::And, right, true);
		func.instruction(&Instruction::Else);
		self.emit_logical_branch(func, op == &Op::Or, right, true);
		func.instruction(&Instruction::End);
		self.emit_call(func, "new_float");
		true
	}

	/// Push i32 1 when `left` is falsy (0 or 0.0), keeping its value in a scratch local (a float as its bits)
	fn emit_held_left_is_falsy(&mut self, func: &mut Function, left: &Node, is_float: bool) {
		let held = self.scratch(LOGICAL_SCRATCH);
		if is_float {
			self.emit_float_value(func, left);
			Self::emit_list(func, &[Instruction::I64ReinterpretF64, Instruction::LocalTee(held), Instruction::F64ReinterpretI64]);
			Self::emit_list(func, &[Instruction::F64Const(Ieee64::new(0.0f64.to_bits())), Instruction::F64Eq]);
		} else {
			self.emit_numeric_value(func, left);
			Self::emit_list(func, &[Instruction::LocalTee(held), Instruction::I64Eqz]);
		}
	}

	/// The held left value, raw
	fn emit_held_left(&self, func: &mut Function, is_float: bool) {
		func.instruction(&Instruction::LocalGet(self.scratch(LOGICAL_SCRATCH)));
		if is_float {
			func.instruction(&Instruction::F64ReinterpretI64);
		}
	}

	/// One branch of a numeric and/or: the held left value or right, raw
	fn emit_logical_branch(&mut self, func: &mut Function, is_left: bool, right: &Node, is_float: bool) {
		match (is_left, is_float) {
			(true, _) => self.emit_held_left(func, is_float),
			(false, true) => self.emit_float_value(func, right),
			(false, false) => self.emit_numeric_value(func, right),
		}
	}

	fn emit_int_truthy_logical(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &Op,
		right: &Node,
	) -> bool {
		if *op != Op::And && *op != Op::Or {
			return false;
		}

		self.emit_held_left_is_falsy(func, left, false);
		func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
		self.emit_logical_branch(func, op == &Op::And, right, false);
		func.instruction(&Instruction::Else);
		self.emit_logical_branch(func, op == &Op::Or, right, false);
		func.instruction(&Instruction::End);
		self.emit_call(func, "new_int");
		true
	}

	/// Apply an arithmetic operator to the two f64 on the stack. Mod is euclidean like the exact Ints, Rem truncates.
	fn emit_float_arithmetic(&mut self, func: &mut Function, op: &Op) {
		match op {
			Op::Add => {
				func.instruction(&Instruction::F64Add);
			}
			Op::Sub => {
				func.instruction(&Instruction::F64Sub);
			}
			Op::Mul => {
				func.instruction(&Instruction::F64Mul);
			}
			Op::Div => {
				func.instruction(&Instruction::F64Div);
			}
			Op::Mod | Op::Rem => self.emit_float_remainder(func, *op == Op::Mod),
			Op::Pow => self.emit_float_power(func),
			_ => self.emit_type_error(func, format!("`{op}` is not an operator on floats")),
		}
	}

	fn push_float_scratch(&self, func: &mut Function, index: u32) {
		func.instruction(&Instruction::LocalGet(self.scratch(index)));
		func.instruction(&Instruction::F64ReinterpretI64);
	}

	fn pop_float_scratch(&self, func: &mut Function, index: u32) {
		func.instruction(&Instruction::I64ReinterpretF64);
		func.instruction(&Instruction::LocalSet(self.scratch(index)));
	}

	/// `a - |b| * floor(a / |b|)` (euclidean) or `a - b * trunc(a / b)`; the f64 operands live in the i64 scratch locals as bits
	fn emit_float_remainder(&mut self, func: &mut Function, euclidean: bool) {
		let (dividend, divisor) = (0, 1);
		self.pop_float_scratch(func, divisor);
		self.pop_float_scratch(func, dividend);
		let push_divisor = |emitter: &Self, func: &mut Function| {
			emitter.push_float_scratch(func, divisor);
			if euclidean {
				func.instruction(&Instruction::F64Abs);
			}
		};
		self.push_float_scratch(func, dividend);
		self.push_float_scratch(func, dividend);
		push_divisor(self, func);
		func.instruction(&Instruction::F64Div);
		func.instruction(if euclidean { &Instruction::F64Floor } else { &Instruction::F64Trunc });
		push_divisor(self, func);
		func.instruction(&Instruction::F64Mul);
		func.instruction(&Instruction::F64Sub);
	}

	/// base ^ exponent through libm's pow; a NaN (negative base with a fractional exponent) traps as invalid_number
	fn emit_float_power(&mut self, func: &mut Function) {
		let Some(pow) = self.ffi_func_index(LIBM_POW) else {
			self.discovered_needs.insert(Need::MathImport(LIBM_POW));
			func.instruction(&Instruction::Unreachable);
			return;
		};
		func.instruction(&Instruction::Call(pow));
		self.pop_float_scratch(func, 0);
		self.push_float_scratch(func, 0);
		self.push_float_scratch(func, 0);
		func.instruction(&Instruction::F64Ne);
		self.emit_fail_if(func, "invalid_number");
		self.push_float_scratch(func, 0);
	}

	fn emit_float_binary(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) -> ArithmeticWrap {
		self.emit_float_value(func, left);
		self.emit_float_value(func, right);

		match op {
			op if op.is_comparison() => {
				self.emit_float_comparison(func, op);
				return ArithmeticWrap::Int;
			}
			op => self.emit_float_arithmetic(func, op),
		}

		ArithmeticWrap::Float
	}

	fn emit_int_binary(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) -> ArithmeticWrap {
		self.emit_int_operands_op(func, left, op, right);
		ArithmeticWrap::Int
	}

	/// Emit both operands as Ints and apply an arithmetic, xor or comparison operator
	fn emit_int_operands_op(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		if op.is_comparison() && self.should_use_float(left, right, op) {
			self.emit_float_value(func, left);
			self.emit_float_value(func, right);
			self.emit_float_comparison(func, op);
			return;
		}
		self.emit_numeric_value(func, left);
		self.emit_numeric_value(func, right);
		let (left_range, right_range) = (self.int_range(left), self.int_range(right));
		if op.is_comparison() {
			self.emit_int_compare(func, op, left_range, right_range);
			func.instruction(&Instruction::I64ExtendI32U);
		} else {
			self.emit_int_op(func, op, left_range, right_range);
			let width = crate::fixed_width::wider(self.declared_fixed_width(left), self.declared_fixed_width(right));
			self.emit_fixed_width_check(func, width);
		}
	}

	/// `local ± 1` for i++ / i--
	fn emit_int_step(&mut self, func: &mut Function, local_pos: u32, op: &Op) {
		func.instruction(&Instruction::LocalGet(local_pos));
		func.instruction(&Instruction::I64Const(1));
		let step = if *op == Op::Inc { Op::Add } else { Op::Sub };
		self.emit_int_op(func, &step, None, Some((1, 1)));
	}

	fn wrap_arithmetic_result(&mut self, func: &mut Function, wrap: ArithmeticWrap) {
		match wrap {
			ArithmeticWrap::Int => self.emit_call(func, "new_int"),
			ArithmeticWrap::Float => self.emit_call(func, "new_float"),
		}
	}

	/// Emit truthy logical operations (and/or) when operands may be non-numeric
	/// Returns a Node reference based on short-circuit evaluation
	fn emit_truthy_logical(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		let node_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(self.type_manager.node_type),
		};

		// A call of number kind is tested at run time like a numeric variable, and evaluated once
		let is_call = matches!(left.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))));
		let left_is_numeric = self.is_numeric(left) || (is_call && matches!(self.get_type(left), Kind::Int | Kind::Float));

		if left_is_numeric {
			let is_float = self.get_type(left).is_float();
			self.emit_held_left_is_falsy(func, left, is_float);
			if *op == Op::Or {
				func.instruction(&Instruction::I32Eqz); // or: a truthy left is the value
			}
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			self.emit_held_left(func, is_float);
			self.emit_call(func, if is_float { "new_float" } else { "new_int" });
			func.instruction(&Instruction::Else);
			self.emit_node_instructions(func, right);
			func.instruction(&Instruction::End);
		} else if Self::is_run_time_value(left) {
			// a variable, call or element holding a Node: its truthiness is known at run time only
			let held = self.node_scratch();
			self.emit_node_instructions(func, left);
			func.instruction(&Instruction::LocalTee(held));
			self.emit_call(func, IS_TRUTHY);
			if *op == Op::And {
				func.instruction(&Instruction::I32Eqz); // and: a falsy left is the value
			}
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			Self::emit_list(func, &[Instruction::LocalGet(held), Instruction::RefAsNonNull, Instruction::Else]);
			self.emit_node_instructions(func, right);
			func.instruction(&Instruction::End);
		} else {
			// a literal left is falsy or not at compile time: `[] or 3`, `"a" and 4`
			let left_is_value = left.is_falsy() == (*op == Op::And);
			self.emit_node_instructions(func, if left_is_value { left } else { right });
		}
	}

	/// A name, call, element or field: its value exists only at run time
	fn is_run_time_value(node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(_) => true,
			Node::List(items, Bracket::Round, _) => matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))),
			Node::Key(_, Op::Hash | Op::Dot, _) => true,
			_ => false,
		}
	}

	/// The one nullable Node local of every function, after its int scratch locals
	fn node_scratch(&self) -> u32 {
		self.int_scratch + big_int::INT_SCRATCH_LOCALS
	}

	/// Emit a fetch call using the host.fetch import (host.fetch_within for an explicit timeout)
	/// Returns a Text node with the body, or an Error node with the reason: the host marks a failure by a negative length
	pub(super) fn emit_fetch_call(&mut self, func: &mut Function, url_node: &Node, timeout: Option<std::time::Duration>) {
		let url = self.extract_url_string(url_node);
		let (url_ptr, url_len) = self.allocate_string(&url);
		func.instruction(&I32Const(url_ptr as i32));
		func.instruction(&I32Const(url_len as i32));
		let import = match timeout {
			Some(timeout) => {
				func.instruction(&Instruction::I64Const(timeout.as_millis().min(i64::MAX as u128) as i64));
				"host_fetch_within"
			}
			None => "host_fetch",
		};
		if let Some(f) = self.ctx.func_registry.get(import) {
			func.instruction(&Instruction::Call(f.call_index as u32));
		} else {
			if timeout.is_some() {
				func.instruction(&Instruction::Drop);
			}
			let reason = format!("fetch {url} failed: host imports are not available");
			let (ptr, len) = self.allocate_string(&reason);
			func.instruction(&I32Const(ptr as i32));
			func.instruction(&I32Const(-(len as i32)));
		}
		let (len, ptr) = (self.scratch(0), self.scratch(1));
		func.instruction(&Instruction::I64ExtendI32S);
		func.instruction(&Instruction::LocalSet(len));
		func.instruction(&Instruction::I64ExtendI32U);
		func.instruction(&Instruction::LocalSet(ptr));
		self.emit_host_text_result(func, len, ptr);
	}


	/// Extract URL string from parsed node tree
	/// Handles patterns like: https://... which parses as Key(Symbol("https"), Colon, ...)
	fn extract_url_string(&self, node: &Node) -> String {
		fn node_to_string(n: &Node) -> String {
			match n.drop_meta() {
				Node::Symbol(s) => s.clone(),
				Node::Text(s) => s.clone(),
				Node::Key(left, op, right) => {
					let left_str = node_to_string(left);
					let op_str = match op {
						Op::Colon => ":",
						Op::Div => "/",
						Op::Dot => ".",
						_ => "",
					};
					let right_str = node_to_string(right);
					format!("{}{}{}", left_str, op_str, right_str)
				}
				Node::List(items, _, _) => items.iter().map(node_to_string).collect::<Vec<_>>().join(""),
				Node::Error(inner) => {
					// Handle parse errors - check if it's an "Unexpected character '/'" error
					if let Node::Text(msg) = inner.drop_meta() {
						if msg.contains("Unexpected character '/'") {
							return "/".to_string();
						}
					}
					// Otherwise recurse into the inner node
					node_to_string(inner)
				}
				_ => format!("{:?}", n),
			}
		}
		node_to_string(node)
	}

	/// Emit global variable declaration: global x = value
	/// Creates a mutable WASM global and initializes it, or reassigns existing global
	/// A local or declared global holding a primitive number, not a Node reference
	pub(super) fn is_numeric_variable(&self, name: &str) -> bool {
		match self.scope.lookup(name) {
			Some(local) => !local.kind.is_ref(),
			None => self.ctx.user_globals.get(name).is_some_and(|(_, kind)| !kind.is_ref()),
		}
	}

	/// `x += y` and `x++` on a declared global (not shadowed by a local) as the plain assignment `x = x + y`,
	/// so globals resolve exactly like in plain assignment
	fn global_update_as_assignment(&self, target: &Node, op: &Op, operand: &Node) -> Option<Node> {
		let Node::Symbol(name) = target.drop_meta() else { return None };
		if self.scope.lookup(name).is_some() || !self.ctx.user_globals.contains_key(name) {
			return None;
		}
		let (base_op, operand) = match op {
			Op::Inc => (Op::Add, Node::int(1)),
			Op::Dec => (Op::Sub, Node::int(1)),
			_ if op.is_compound_assign() => (op.base_op(), operand.clone()),
			_ => return None,
		};
		let updated = Node::Key(Box::new(target.clone()), base_op, Box::new(operand));
		Some(Node::Key(Box::new(target.clone()), Op::Assign, Box::new(updated)))
	}

	/// `x = v` for a declared global x: store v in the global's own representation and leave it on the stack
	fn emit_global_store(&mut self, func: &mut Function, name: &str, value: &Node) -> Option<Kind> {
		let &(index, kind) = self.ctx.user_globals.get(name)?;
		self.emit_value_of_kind(func, value, kind);
		func.instruction(&Instruction::GlobalSet(index));
		func.instruction(&Instruction::GlobalGet(index));
		if kind.is_ref() {
			func.instruction(&Instruction::RefAsNonNull);
		}
		Some(kind)
	}

	/// `global x=7` declares x with the value 7; `global x` declares x zero-initialized
	fn global_declaration_parts(decl: &Node) -> Option<(String, Node)> {
		match decl.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), Node::int(0))),
			Node::Key(left, Op::Define | Op::Assign, right) => match left.drop_meta() {
				Node::Symbol(name) => Some((name.clone(), right.as_ref().clone())),
				_ => None,
			},
			_ => None,
		}
	}

	/// `global x = value`: x is declared ahead of all code (`allocate_declared_globals`), so this stores the value
	fn emit_global_declaration(&mut self, func: &mut Function, decl: &Node) {
		let Some((name, value)) = Self::global_declaration_parts(decl) else {
			self.emit_malformed(func, decl, "`global name` or `global name = value`");
			return;
		};
		let kind = self.declare_global(&name, &value);
		self.emit_global_store(func, &name, &value);
		if !kind.is_ref() {
			self.emit_primitive_as_node(func, kind);
		}
	}

	/// The global named `name`, declared with the kind of its first value unless it already exists
	fn declare_global(&mut self, name: &str, value: &Node) -> Kind {
		match self.ctx.user_globals.get(name) {
			Some(&(_, kind)) => kind,
			None => self.allocate_global(name, crate::analyzer::held_kind(value, || self.get_type(value))),
		}
	}

	fn allocate_global(&mut self, name: &str, kind: Kind) -> Kind {
		let global_idx = self.declare_mutable_global(kind);
		self.ctx.user_globals.insert(name.to_string(), (global_idx, kind));
		kind
	}

	/// Every `global` of the program exists before any function is compiled, so function bodies can change it
	fn allocate_declared_globals(&mut self, program: &Node) {
		let mut main = Scope::new();
		collect_variables(program, &mut main);
		let mut names: Vec<&String> = main.globals.keys().filter(|name| !self.ctx.user_globals.contains_key(*name)).collect();
		names.sort();
		for name in names {
			self.allocate_global(name, main.globals[name].kind);
		}
		self.ctx.declared_globals = main.globals;
	}

	/// The primitive on the stack (i64 or f64 as stored, see `storage_type`) as a Node of its kind
	fn emit_primitive_as_node(&mut self, func: &mut Function, kind: Kind) {
		if kind.is_float() {
			self.emit_call(func, "new_float");
		} else if kind == Kind::Codepoint {
			func.instruction(&Instruction::I32WrapI64);
			self.emit_call(func, "new_codepoint");
		} else {
			self.emit_call(func, "new_int");
		}
	}

	/// How a value of `kind` is stored in locals, parameters and globals
	fn storage_type(&self, kind: Kind) -> ValType {
		if kind.is_ref() {
			Ref(self.node_ref(false))
		} else if kind.is_float() {
			ValType::F64
		} else {
			ValType::I64
		}
	}

	/// Emit `node` in the representation `storage_type(kind)` expects
	fn emit_value_of_kind(&mut self, func: &mut Function, node: &Node, kind: Kind) {
		if kind.is_ref() {
			self.emit_node_instructions(func, node);
		} else if kind.is_float() {
			self.emit_float_value(func, node);
		} else {
			self.emit_numeric_value(func, node);
		}
	}

	/// Declare a zero/null-initialized mutable global holding a value of `kind`
	fn declare_mutable_global(&mut self, kind: Kind) -> u32 {
		let init_expr = if kind.is_ref() {
			ConstExpr::ref_null(HeapType::Concrete(self.type_manager.node_type))
		} else if kind.is_float() {
			ConstExpr::f64_const(Ieee64::new(0.0f64.to_bits()))
		} else {
			ConstExpr::i64_const(0)
		};
		let val_type = if kind.is_ref() { Ref(self.node_ref(true)) } else { self.storage_type(kind) };
		self.globals.global(GlobalType { val_type, mutable: true, shared: false }, &init_expr);
		self.next_global_idx += 1;
		self.next_global_idx - 1
	}

	/// Leave `value` in the exported global `trap_detail` (declared on first use), for the runtime error that follows
	/// to name it: the runner reads it after the trap (wasm_reader::with_trap_detail)
	pub(super) fn emit_trap_detail(&mut self, func: &mut Function, value: &Node) {
		const TRAP_DETAIL_GLOBAL: &str = "trap·detail"; // not a wasp name, so no user global meets it
		if !self.ctx.user_globals.contains_key(TRAP_DETAIL_GLOBAL) {
			let index = self.declare_mutable_global(Kind::Empty);
			self.exports.export(TRAP_DETAIL, ExportKind::Global, index);
			self.ctx.user_globals.insert(TRAP_DETAIL_GLOBAL.to_string(), (index, Kind::Empty));
		}
		self.emit_global_store(func, TRAP_DETAIL_GLOBAL, value);
		func.instruction(&Instruction::Drop);
	}

	/// Emit global declaration and return numeric value (for use in emit_numeric_value)
	fn emit_global_numeric(&mut self, func: &mut Function, decl: &Node) {
		let Some((name, value)) = Self::global_declaration_parts(decl) else {
			self.emit_malformed(func, decl, "`global name` or `global name = value`");
			return;
		};
		let kind = self.declare_global(&name, &value);
		self.emit_global_store(func, &name, &value);
		if kind.is_ref() {
			self.emit_call(func, "get_int_value");
		} else if kind.is_float() {
			self.emit_float_in_exact_context(func, &name);
		}
	}

	/// Emit ternary expression: condition ? then_expr : else_expr
	/// Returns a Node reference, handling mixed-type branches (numbers, strings, etc.)
	fn emit_ternary(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		// Structure: condition ? Key(then, Colon, else)
		let Node::Key(then_expr, Op::Colon, else_expr) = then_else.drop_meta() else {
			self.emit_malformed(func, then_else, TERNARY_BRANCHES);
			return;
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_condition(func, condition, Self::emit_numeric_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - use emit_node_instructions to handle any type (Text, Number, etc.)
		self.emit_node_instructions(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch - use emit_node_instructions to handle any type
		self.emit_node_instructions(func, else_expr);

		func.instruction(&Instruction::End);
	}

	/// Emit ternary expression returning i64: condition ? then_expr : else_expr
	fn emit_ternary_numeric(&mut self, func: &mut Function, condition: &Node, then_else: &Node) {
		// Structure: condition ? Key(then, Colon, else)
		let Node::Key(then_expr, Op::Colon, else_expr) = then_else.drop_meta() else {
			self.emit_malformed(func, then_else, TERNARY_BRANCHES);
			return;
		};

		// Evaluate condition and convert to i32 for if instruction
		self.emit_condition(func, condition, Self::emit_numeric_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

		// Then branch
		self.emit_numeric_value(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch
		self.emit_numeric_value(func, else_expr);

		func.instruction(&Instruction::End);
	}

	/// Emit if-then-else returning i64: if condition then then_expr else else_expr
	fn emit_if_then_else_numeric(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		// Extract condition and then_expr from structure
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let Some((condition, then_expr)) = if_then_parts(left) else {
			self.emit_malformed(func, left, IF_THEN);
			return;
		};

		// Evaluate condition
		self.emit_condition(func, condition, Self::emit_numeric_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

		// Then branch
		self.emit_numeric_value(func, then_expr);

		func.instruction(&Instruction::Else);

		// Else branch
		if let Some(else_node) = else_expr {
			self.emit_numeric_value(func, else_node);
		} else {
			// No else branch - return 0
			func.instruction(&Instruction::I64Const(0));
		}

		func.instruction(&Instruction::End);
	}

	/// Emit if-then-else expression: if condition then then_expr [else else_expr]
	/// Structure: Key(Key(Key(Empty, If, condition), Then, then_expr), Else, else_expr)
	/// Or for if-then without else: Key(Key(Empty, If, condition), Then, then_expr)
	fn emit_if_then_else(&mut self, func: &mut Function, left: &Node, else_expr: Option<&Node>) {
		// Extract condition and then_expr from structure
		// Structure: Key(Key(Empty, If, condition), Then, then_expr)
		let Some((condition, then_expr)) = if_then_parts(left) else {
			self.emit_malformed(func, left, IF_THEN);
			return;
		};

		// A branch yielding a text, character, list or error (`if x {x} else {"offline"}`): both branches are Node values
		let branch_value = |branch: &Node| match branch.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => items[0].clone(),
			other => other.clone(),
		};
		let then_value = branch_value(then_expr);
		let else_value = else_expr.map(branch_value);
		let is_node_valued = |emitter: &Self, value: &Node| {
			let kind = crate::analyzer::branch_kind(value, &emitter.scope);
			!matches!(value.drop_meta(), Node::Empty) && (emitter.is_structured_value(value) || kind.is_ref() || kind == Kind::Codepoint)
		};
		if is_node_valued(self, &then_value) || else_value.as_ref().is_some_and(|value| is_node_valued(self, value)) {
			self.emit_condition(func, condition, Self::emit_block_value);
			func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(true))))); // a local holds a nullable ref
			self.emit_node_instructions(func, &then_value);
			func.instruction(&Instruction::Else);
			self.emit_node_instructions(func, else_value.as_ref().unwrap_or(&Node::Empty));
			func.instruction(&Instruction::End);
			func.instruction(&Instruction::RefAsNonNull);
			return;
		}

		// Evaluate condition and convert to i32 for if instruction
		self.emit_condition(func, condition, Self::emit_block_value);

		// if (condition) { then_expr } else { else_expr }
		func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(false)))));

		// Then branch - extract value from block if needed
		self.emit_block_value(func, then_expr);
		self.emit_call(func, "new_int");

		func.instruction(&Instruction::Else);

		// Else branch - use provided else_expr or default to 0
		if let Some(else_node) = else_expr {
			self.emit_block_value(func, else_node);
		} else {
			func.instruction(&Instruction::I64Const(0));
		}
		self.emit_call(func, "new_int");

		func.instruction(&Instruction::End);
	}

	/// Emit while loop: (while condition) do body
	/// If wrap_result is true, wraps result in Node; otherwise returns raw i64
	fn emit_while_loop_impl(&mut self, func: &mut Function, left: &Node, body: &Node, wrap_result: bool) {
		let Node::Key(_, Op::While, condition) = left.drop_meta() else {
			self.emit_malformed(func, left, "`while condition`");
			return;
		};

		let result_local = self.next_temp_local;
		self.next_temp_local += 1;

		let ran_local = self.next_temp_local;
		self.next_temp_local += 1;

		func.instruction(&Instruction::I64Const(0));
		func.instruction(&Instruction::LocalSet(result_local));
		func.instruction(&Instruction::I64Const(0));
		func.instruction(&Instruction::LocalSet(ran_local));

		func.instruction(&Instruction::Block(BlockType::Empty));
		let break_frame = loop_control::open_control_frames(func);
		func.instruction(&Instruction::Loop(BlockType::Empty));

		self.emit_condition(func, condition, Self::emit_block_value);
		func.instruction(&Instruction::I32Eqz);
		func.instruction(&Instruction::BrIf(1));

		func.instruction(&Instruction::I64Const(1));
		func.instruction(&Instruction::LocalSet(ran_local));
		if loop_control::has_jump(body) {
			let (statements, step) = loop_control::split_step(body);
			func.instruction(&Instruction::Block(BlockType::Empty));
			self.enter_loop(break_frame, loop_control::open_control_frames(func));
			self.emit_loop_body(func, &statements, result_local);
			self.leave_loop();
			func.instruction(&Instruction::End);
			if let Some(step) = step {
				self.emit_loop_body(func, &step, result_local);
			}
		} else {
			self.emit_loop_body(func, body, result_local);
		}
		func.instruction(&Instruction::Br(0));

		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);

		if wrap_result {
			// the value of a loop is its last body value; a loop whose body never ran is empty
			func.instruction(&Instruction::LocalGet(ran_local));
			func.instruction(&Instruction::I32WrapI64);
			func.instruction(&Instruction::If(BlockType::Result(Ref(self.node_ref(false)))));
			func.instruction(&Instruction::LocalGet(result_local));
			self.emit_call(func, "new_int");
			func.instruction(&Instruction::Else);
			self.emit_call(func, "new_empty");
			func.instruction(&Instruction::End);
		} else {
			func.instruction(&Instruction::LocalGet(result_local));
		}
	}

	/// One pass of a loop body; a numeric value becomes the loop's value
	fn emit_loop_body(&mut self, func: &mut Function, body: &Node, result_local: u32) {
		if body.is_nothing() { // `while c {}` only spins: nothing to evaluate
		} else if self.get_type(body).is_ref() { // `while c { s += "a" }`: a text or list update, its value is not a number
			self.emit_node_instructions(func, body);
			func.instruction(&Instruction::Drop);
		} else {
			self.emit_block_value(func, body);
			func.instruction(&Instruction::LocalSet(result_local));
		}
	}

	fn emit_while_loop(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, true);
	}

	fn emit_while_loop_value(&mut self, func: &mut Function, left: &Node, body: &Node) {
		self.emit_while_loop_impl(func, left, body, false);
	}

	/// Text converts only if it is a number literal (truncated for int); anything else is a runtime error, never a plausible 0
	fn emit_text_cast(&mut self, func: &mut Function, text: &str, target_type: &Node) {
		match crate::wasp_parser::number_in_text(text) {
			Some(number) => self.emit_cast(func, &Node::Number(number), target_type),
			None => self.emit_runtime_error(func, "invalid_number"),
		}
	}

	/// An f64 has no exact value yet: `x as exact` of a runtime float is refused instead of rounded silently
	fn emit_inexact_to_exact(&mut self, func: &mut Function, value: &Node) {
		let message = format!("{} is an IEEE float, `as exact` of a float is not supported yet: keep it exact from the start", value.serialize());
		self.emit_type_error(func, message);
	}

	/// `v as T` as a raw Int: `as float` has no exact value, `as exact` keeps it
	/// A cast that makes no sense: a list as a number or character, a number as a list
	fn cast_refusal(&self, value: &Node, target: &Node) -> Option<String> {
		let target_name = target.name().to_lowercase();
		let kind = self.get_type(value);
		let text_like = matches!(target_name.as_str(), "string" | "str" | "text");
		let refused = match kind {
			Kind::List => target_name != "list" && !text_like,
			Kind::Int | Kind::Float => target_name == "list",
			_ => false,
		};
		refused.then(|| format!("cannot cast {kind} to {target_name}: {} as {target_name}", value.serialize()))
	}

	fn emit_numeric_cast(&mut self, func: &mut Function, located: &Node, value: &Node, target: &Node) {
		if let Some(refusal) = self.cast_refusal(value, target) {
			self.emit_type_error(func, refusal);
			return;
		}
		let exact_value = !matches!(value.drop_meta(), Node::Text(_) | Node::Char(_)) && !self.get_type(value).is_float();
		match crate::type_kinds::canonical_type_name(&target.name().to_lowercase()) {
			"float" => {
				let message = format!("{} is a float where an exact Int is expected: `as float` promotes, `as int` truncates", located.serialize());
				self.emit_type_error(func, message);
			}
			"exact" if exact_value => self.emit_numeric_value(func, value),
			// a character held unboxed is its code point
			"char" | "character" => match value.drop_meta() {
				Node::Char(character) => {
					func.instruction(&Instruction::I64Const(*character as i64));
				}
				_ => self.emit_numeric_value(func, value),
			},
			_ if crate::analyzer::builtin_type_kind(&target.name()) == Some(Kind::Int) => {
				self.emit_cast(func, value, target);
				self.emit_call(func, "get_int_value");
			}
			_ => self.emit_not_a_number(func, located, located.drop_meta()),
		}
	}

	/// `text as list` is its characters, a list stays as it is; the characters of a text known only at runtime need a runtime splitter
	fn emit_list_cast(&mut self, func: &mut Function, value: &Node) {
		match value {
			Node::Text(text) => {
				let characters = text.chars().map(Node::Char).collect();
				self.emit_node_instructions(func, &Node::List(characters, Bracket::Square, Separator::Space));
			}
			Node::Char(_) => self.emit_node_instructions(func, &Node::List(vec![value.clone()], Bracket::Square, Separator::Space)),
			_ if self.get_type(value) == Kind::List => self.emit_node_instructions(func, value),
			_ if matches!(self.get_type(value), Kind::Text | Kind::Codepoint) => {
				self.emit_node_instructions(func, value);
				self.emit_call(func, "text_chars");
			}
			_ => {
				let kind = self.get_type(value);
				self.emit_type_error(func, format!("cannot cast {kind} to list: {} as list", value.serialize()));
			}
		}
	}

	/// `x as string` for a variable: an Int, a Text or a character is the join of the one-element list;
	/// anything else has no runtime text yet
	fn emit_runtime_text_cast(&mut self, func: &mut Function, value: &Node) {
		let kind = self.get_type(value);
		let node = match kind {
			Kind::Int | Kind::Float | Kind::Text | Kind::Codepoint => joined_text(std::slice::from_ref(value), ""),
			// user decision #35: an int list joins to "[1 2]", like its literal; a general runtime serializer comes later
			Kind::List => {
				let text = |text: &str| Box::new(Node::Text(text.to_string()));
				let items = Node::Key(Box::new(join_call(value.clone(), " ")), Op::Add, text("]"));
				Node::Key(text("["), Op::Add, Box::new(items))
			}
			_ => {
				self.emit_type_error(func, format!("cannot cast {kind} to string: `{} as string` has no runtime text yet", value.serialize()));
				return;
			}
		};
		self.emit_node_instructions(func, &node);
	}

	/// Emit type cast: value as type
	/// Handles conversions between int, float, string
	/// Optimizes literal conversions at compile time
	fn emit_cast(&mut self, func: &mut Function, value: &Node, target_type: &Node) {
		let type_name = match target_type.drop_meta() {
			Node::Symbol(s) => s.to_lowercase(),
			Node::Text(s) => s.to_lowercase(),
			_ => {
				// Unknown type, emit as-is
				self.emit_node_instructions(func, value);
				return;
			}
		};

		let value = value.drop_meta();
		if let Some(refusal) = self.cast_refusal(value, target_type) {
			self.emit_type_error(func, refusal);
			return;
		}

		match crate::type_kinds::canonical_type_name(&type_name) {
			"list" => self.emit_list_cast(func, value),
			"i64" | "int64" if !self.get_type(value).is_float() && !matches!(value, Node::Text(_) | Node::Char(_)) => {
				self.emit_wrapping_int(func, value);
				self.emit_call(func, "new_int");
			}
			"int" | "integer" | "i32" | "i64" | "long" => {
				// Cast to integer
				match value {
					// Compile-time: float literal to int
					Node::Number(Number::Float(f)) => {
						func.instruction(&Instruction::I64Const(*f as i64));
						self.emit_call(func, "new_int");
					}
					Node::Text(s) => self.emit_text_cast(func, s, target_type),
					// Compile-time: char literal to int (parse digit)
					Node::Char(c) => {
						let n: i64 = c.to_string().parse().unwrap_or(*c as i64);
						func.instruction(&Instruction::I64Const(n));
						self.emit_call(func, "new_int");
					}
					// Runtime: float expression to int
					_ if self.get_type(value).is_float() => {
						self.emit_float_value(func, value);
						self.emit_truncating_cast(func);
						self.emit_call(func, "new_int");
					}
					// Runtime: a text, or a value known only at run time (a list element), parses its digits
					_ if matches!(self.get_type(value), Kind::Text | Kind::Empty) => {
						self.emit_node_instructions(func, value);
						self.emit_call(func, list_ops::TEXT_AS_INT);
						self.emit_call(func, "new_int");
					}
					// Already int or coercible; a ratio is truncated
					_ => {
						self.emit_numeric_value(func, value);
						if self.int_runtime() && !big_int::is_fixnum_range(self.int_range(value)) {
							self.emit_call(func, "exact_trunc");
						}
						self.emit_call(func, "new_int");
					}
				}
			}
			"exact" => match value {
				Node::Text(s) => self.emit_text_cast(func, s, target_type),
				Node::Char(_) => self.emit_cast(func, value, &Node::Symbol("int".into())),
				_ if self.get_type(value).is_float() => self.emit_inexact_to_exact(func, value),
				_ => {
					// already exact: decimal literals are ratios (exact.rs)
					self.emit_numeric_value(func, value);
					self.emit_call(func, "new_int");
				}
			},
			"float" | "f32" => {
				// Cast to float
				match value {
					Node::Text(s) => self.emit_text_cast(func, s, target_type),
					// Compile-time: char literal to float (parse digit)
					Node::Char(c) => {
						let f: f64 = c.to_string().parse().unwrap_or(*c as i64 as f64);
						func.instruction(&Instruction::F64Const(f.into()));
						self.emit_call(func, "new_float");
					}
					// Compile-time: int literal to float
					Node::Number(Number::Int(n)) => {
						func.instruction(&Instruction::F64Const((*n as f64).into()));
						self.emit_call(func, "new_float");
					}
					// Runtime: emit as float
					_ => {
						self.emit_float_value(func, value);
						self.emit_call(func, "new_float");
					}
				}
			}
			"string" | "str" | "text" => {
				// Cast to string
				match value {
					// Compile-time: number literal to string
					Node::Number(n) => {
						let s = n.to_string();
						let (ptr, len) = self.allocate_string(&s);
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
						self.emit_call(func, "new_text");
					}
					// Compile-time: char to string
					Node::Char(c) => {
						let s = c.to_string();
						let (ptr, len) = self.allocate_string(&s);
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
						self.emit_call(func, "new_text");
					}
					// Already a string - use the string table
					Node::Text(s) => {
						self.emit_string_call(func, s, "new_text");
					}
					// data and names are their source text; an Int expression (`str(1+2)`) is the text of its value
					_ if !self.mentions_variable(value) && self.get_type(value) != Kind::Int => {
						let (ptr, len) = self.allocate_string(&value.serialize());
						func.instruction(&I32Const(ptr as i32));
						func.instruction(&I32Const(len as i32));
						self.emit_call(func, "new_text");
					}
					_ => self.emit_runtime_text_cast(func, value),
				}
			}
			"char" | "character" => {
				// Cast to char: int to char (digit representation)
				// new_codepoint expects i32
				match value {
					Node::Number(Number::Int(n)) => {
						// Convert digit to char: 2 → '2'
						let c = if *n >= 0 && *n <= 9 {
							char::from_digit(*n as u32, 10).unwrap_or('?')
						} else {
							char::from_u32(*n as u32).unwrap_or('?')
						};
						func.instruction(&I32Const(c as i32));
						self.emit_call(func, "new_codepoint");
					}
					Node::Char(c) => {
						func.instruction(&I32Const(*c as i32));
						self.emit_call(func, "new_codepoint");
					}
					_ => {
						self.emit_numeric_value(func, value);
						func.instruction(&Instruction::I32WrapI64);
						self.emit_call(func, "new_codepoint");
					}
				}
			}
			"bool" | "boolean" => {
				// Cast to bool
				match value {
					// Compile-time: string to bool
					Node::Text(s) => {
						let b = !matches!(
							s.to_lowercase().as_str(),
							"" | "0" | "false" | "no" | "ø" | "nil" | "null" | "none"
						);
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Char(c) => {
						let b = !matches!(*c, '0' | 'ø');
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Number(Number::Int(n)) => {
						let b = *n != 0;
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::Number(Number::Float(f)) => {
						let b = *f != 0.0;
						func.instruction(&Instruction::I64Const(if b { 1 } else { 0 }));
						self.emit_call(func, "new_int");
					}
					Node::True => {
						func.instruction(&Instruction::I64Const(1));
						self.emit_call(func, "new_int");
					}
					Node::False => {
						func.instruction(&Instruction::I64Const(0));
						self.emit_call(func, "new_int");
					}
					_ => {
						// Runtime: non-zero/non-empty is truthy
						self.emit_numeric_value(func, value);
						func.instruction(&Instruction::I64Eqz);
						func.instruction(&Instruction::I64ExtendI32U);
						func.instruction(&Instruction::I64Const(1));
						func.instruction(&Instruction::I64Xor);
						self.emit_call(func, "new_int");
					}
				}
			}
			"number" | "num" => {
				// Cast to number: auto-detect int or float
				match value {
					Node::Text(s) => {
						// Try parsing as int first, then float
						if let Ok(n) = s.parse::<i64>() {
							func.instruction(&Instruction::I64Const(n));
							self.emit_call(func, "new_int");
						} else if let Ok(f) = s.parse::<f64>() {
							func.instruction(&Instruction::F64Const(f.into()));
							self.emit_call(func, "new_float");
						} else {
							func.instruction(&Instruction::I64Const(0));
							self.emit_call(func, "new_int");
						}
					}
					Node::Char(c) => {
						if let Some(n) = c.to_digit(10) {
							func.instruction(&Instruction::I64Const(n as i64));
							self.emit_call(func, "new_int");
						} else {
							func.instruction(&Instruction::I64Const(*c as i64));
							self.emit_call(func, "new_int");
						}
					}
					_ => {
						// Already numeric
						self.emit_node_instructions(func, value);
					}
				}
			}
			_ => {
				// Unknown type, emit as key node for dynamic dispatch
				self.emit_node_instructions(func, value);
				self.emit_node_instructions(func, target_type);
				func.instruction(&Instruction::I64Const(op_to_code(&Op::As)));
				self.emit_call(func, "new_key");
			}
		}
	}

	/// Extract numeric value from a block { expr } or plain expr
	fn emit_block_value(&mut self, func: &mut Function, node: &Node) {
		match node.drop_meta() {
			Node::List(items, Bracket::Curly, _) if items.len() == 1 => {
				// Block with single item: { expr } -> extract expr
				self.emit_numeric_value(func, &items[0]);
			}
			_ => self.emit_numeric_value(func, node),
		}
	}

	/// Emit the numeric value of a node onto the stack (as i64)
	/// `print x`, `puti x`: an output builtin (not shadowed by a user function), whose value is the printed node
	pub(super) fn is_output_call(&self, node: &Node) -> bool {
		matches!(node.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, _]
			if matches!(word.drop_meta(), Node::Symbol(name) if OUTPUT_CALLS.contains(&name.as_str()) && !self.ctx.user_functions.contains_key(name))))
	}

	fn emit_numeric_value(&mut self, func: &mut Function, node: &Node) {
		if let Some((list, sum_loop)) = list_dispatch::list_sum_parts(node) {
			return self.emit_list_sum(func, list, sum_loop, list_dispatch::Wanted::Int);
		}
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_numeric_value) {
			return;
		}
		let located = node;
		let node = node.drop_meta();
		// Handle global declaration: global:Key(name, =, value)
		if let Node::Key(left, Op::Colon, right) = node {
			if let Node::Symbol(kw) = left.drop_meta() {
				if kw == "global" {
					// Emit global and return value on stack
					self.emit_global_numeric(func, right);
					return;
				}
			}
		}
		match node {
			Node::Number(num) => {
				match num {
					Number::Int(_) | Number::BigInt(_) => {
						self.emit_int_literal(func, &num.to_bigint());
						func
					}
					Number::Float(f) => {
						self.emit_decimal_literal(func, *f);
						func
					}
					Number::Quotient(n, d) => {
						self.emit_exact_literal(func, &(*n).into(), &(*d).into());
						func
					}
					Number::Complex(r, _i) => func.instruction(&Instruction::I64Const(*r as i64)),
					// lowered to Float before emission (real.rs), kept total for safety
					Number::Real(r) => func.instruction(&Instruction::I64Const(r.to_f64() as i64)),
					Number::Nan | Number::Inf | Number::NegInf => {
						func.instruction(&Instruction::I64Const(0)) // special values → 0
					}
				};
			}
			Node::True => {
				func.instruction(&Instruction::I64Const(1));
			}
			Node::False => {
				func.instruction(&Instruction::I64Const(0));
			}
			Node::Char(c) => {
				func.instruction(&Instruction::I64Const(*c as i64));
			}
			// Variable definition/assignment: x:=42 or x=42 → store and return value
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Key(node_expr, Op::Hash, index_expr) = left.drop_meta() {
					self.emit_index_assignment(func, node_expr, index_expr, right);
					return;
				}
				if let Node::Symbol(name) = left.drop_meta() {
					if let Some((position, kind)) = self.scope.lookup(name).map(|local| (local.position, local.kind)) {
						if kind.is_float() {
							let message = format!("{} assigns a float where an exact Int is expected", located.serialize());
							self.emit_type_error(func, message);
							return;
						}
						if self.emit_typed_list_store(func, name, right) {
							func.instruction(&Instruction::Drop);
							func.instruction(&Instruction::I64Const(0));
							return;
						}
						if kind.is_ref() {
							// a variable holding ø, a list or a text: the value lives in the local, a number is also the value of the assignment
							self.emit_node_instructions(func, right);
							func.instruction(&Instruction::LocalSet(position));
							if self.get_type(right) == Kind::Int {
								self.emit_numeric_value(func, left);
							} else {
								func.instruction(&Instruction::I64Const(0));
							}
							return;
						}
						self.emit_value_of_kind(func, right, kind);
						self.emit_fits_declared(func, left);
						func.instruction(&Instruction::LocalTee(position));
					} else if let Some(kind) = self.emit_global_store(func, name, right) {
						if kind.is_float() {
							self.emit_float_in_exact_context(func, name);
						}
					} else {
						self.emit_undefined_variable(func, name);
					}
				} else {
					self.emit_malformed(func, left, "a variable to assign to");
				}
			}
			// Increment/decrement: i++ or i--
			Node::Key(left, op, right) if *op == Op::Inc || *op == Op::Dec => {
				if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
					self.emit_numeric_value(func, &assignment);
					return;
				}
				if let Node::Symbol(name) = left.drop_meta() {
					let Some(local_pos) = self.defined_local_position(func, name) else { return };
					self.emit_int_step(func, local_pos, op);
					self.emit_fits_declared(func, left);
					// Store and return new value
					func.instruction(&Instruction::LocalTee(local_pos));
				} else {
					self.emit_malformed(func, left, "a variable to increment or decrement");
				}
			}
			// Compound assignment: x += y → x = x + y
			Node::Key(left, op, right) if op.is_compound_assign() => {
				if self.emit_compound_index_assignment(func, left, op, right) {
					return;
				}
				if let Some(assignment) = self.global_update_as_assignment(left, op, right) {
					self.emit_numeric_value(func, &assignment);
					return;
				}
				if let Node::Symbol(name) = left.drop_meta() {
					// Get local position first to avoid borrow issues
					let Some(local_pos) = self.defined_local_position(func, name) else { return };
					let base_op = op.base_op();
					// Get current value of x
					func.instruction(&Instruction::LocalGet(local_pos));
					// Emit y
					self.emit_numeric_value(func, right);
					self.emit_int_compound_op(func, &base_op, right);
					self.emit_fits_declared(func, left);
					// Store result and leave on stack
					func.instruction(&Instruction::LocalTee(local_pos));
				} else {
					self.emit_malformed(func, left, "a variable to update");
				}
			}
			// Arithmetic operators
			Node::Key(left, op, right) if op.is_arithmetic() => {
				let kind = self.arithmetic_type(left, op, right);
				if self.emit_arithmetic_type_error(func, left, op, right, kind) {
					return;
				}
				self.emit_int_operands_op(func, left, op, right);
			}
			Node::Key(left, op, right) if op.is_shift() => self.emit_shift(func, left, op, right),
			// Logical operators (and, or) use truthy semantics, xor uses bitwise
			Node::Key(left, Op::And, right) => {
				// Truthy and: if left is 0, return 0; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				func.instruction(&Instruction::I64Const(0));
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::End);
			}
			Node::Key(left, Op::Or, right) => {
				// Truthy or: if left is non-0, return left; else return right
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::I64Eqz);
				func.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
				self.emit_numeric_value(func, right);
				func.instruction(&Instruction::Else);
				self.emit_numeric_value(func, left);
				func.instruction(&Instruction::End);
			}
			Node::Key(left, op, right) if self.compares_structurally(op, left, right) => {
				self.emit_structural_equality(func, left, op, right);
			}
			Node::Key(left, op, right) if self.orders_a_list(op, left, right) => self.emit_unordered_list_error(func, left, op, right),
			Node::Key(left, op, right) if *op == Op::Xor || op.is_comparison() => {
				self.emit_int_operands_op(func, left, op, right);
			}
			// Prefix operators: √x, -x, !x, ‖x‖, #x (count)
			Node::Key(left, op, right) if op.is_prefix() && matches!(left.drop_meta(), Node::Empty) => {
				match op {
					Op::Sqrt => {
						self.emit_float_value(func, right);
						func.instruction(&Instruction::F64Sqrt);
						self.emit_float_in_exact_context(func, &located.serialize());
					}
					Op::Neg => {
						self.emit_numeric_value(func, right);
						let range = self.int_range(right);
						self.emit_int_neg(func, range);
					}
					Op::Not => {
						// not x = x is falsy: 0, ø and errors
						self.emit_condition(func, right, Self::emit_numeric_value);
						func.instruction(&Instruction::I32Eqz);
						func.instruction(&Instruction::I64ExtendI32U);
					}
					Op::Abs => {
						self.emit_numeric_value(func, right);
						let range = self.int_range(right);
						self.emit_int_abs(func, range);
					}
					_ => self.emit_type_error(func, format!("`{op}` of an exact number is not supported yet")),
				}
			}
			// `xs.size`, `xs.count`, `xs.bytes`: the raw i64 of the counting getter
			Node::Key(counted, Op::Dot, property) if self.counting_getter(property).is_some() => {
				let counter = self.counting_getter(property).expect("guarded");
				self.emit_list_count(func, counted, counter);
			}
			// Prefix # means count/length: #list returns element count
			Node::Key(left, Op::Hash, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_list_count(func, right, "node_count");
			}
			// Index operator: list#index (1-based)
			Node::Key(list, Op::Hash, index) => {
				if self.map_is_indexed_by_key(index) {
					self.emit_indexed_node(func, list, index);
					self.emit_call(func, "get_int_value");
					return;
				}
				self.emit_list_element_number(func, list, index);
			}
			// Ternary operator: condition ? then_expr : else_expr
			Node::Key(condition, Op::Question, then_else) => {
				self.emit_ternary_numeric(func, condition, then_else);
			}
			// If-then-else: if condition then then_expr else else_expr
			Node::Key(if_then, Op::Else, else_expr) => {
				self.emit_if_then_else_numeric(func, if_then, Some(else_expr));
			}
			// If-then (no else): if condition then then_expr
			Node::Key(if_cond, Op::Then, then_expr) => {
				// Construct node for emit_if_then_else_numeric
				let full_node = Node::Key(if_cond.clone(), Op::Then, then_expr.clone());
				self.emit_if_then_else_numeric(func, &full_node, None);
			}
			// Variable lookup (local or global)
			Node::Symbol(name) => {
				// Handle $n parameter reference (e.g., $0 = first param)
				if let Some(rest) = name.strip_prefix('$') {
					if let Ok(idx) = rest.parse::<u32>() {
						func.instruction(&Instruction::LocalGet(idx));
						return;
					}
				}
				if self.emit_typed_list_as_node(func, name) {
					self.emit_call(func, "get_int_value");
				} else if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
					if local.kind.is_ref() {
						// an optional held as a Node, checked non-ø before use (analyzer::check_null_use)
						self.emit_call(func, "get_int_value");
					} else if local.kind.is_float() {
						self.emit_float_in_exact_context(func, name);
					}
				} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
					func.instruction(&Instruction::GlobalGet(idx));
					if kind.is_float() {
						self.emit_float_in_exact_context(func, name);
					}
				} else {
					self.emit_undefined_variable(func, name);
				}
			}
			// Statement sequence or function call
			Node::List(items, bracket, separator) if !items.is_empty() => {
				// Check for zero-argument function call: (funcname)
				if items.len() == 1 && *bracket == Bracket::Round {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_numeric(func, fn_name, &[]);
							return;
						}
						// Check for zero-arg FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &[], Some(Kind::Int));
							return;
						}
					}
				}
				// Check for return statement: [Symbol("return"), value]
				if items.len() == 2 {
					if let Node::Symbol(keyword) = items[0].drop_meta() {
						if keyword == "return" {
							// Emit the return value, as the Node a Node-returning function gives back
							if self.returns_node {
								self.emit_node_instructions(func, &items[1]);
							} else {
								self.emit_numeric_value(func, &items[1]);
							}
							self.emit_leave_tries(func, 0);
							func.instruction(&Instruction::Return);
							// After return, emit unreachable to satisfy block types
							func.instruction(&Instruction::I64Const(0));
							return;
						}
					}
				}
				// Integer conversion: int("5") + 3
				if items.len() == 2 && self.get_type(node) == Kind::Int && matches!(items[0].drop_meta(), Node::Symbol(s) if type_word_kind(s) == Some(Kind::Int)) {
					self.emit_cast(func, &items[1], &items[0]);
					self.emit_call(func, "get_int_value");
					return;
				}
				// Check for user function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_numeric(func, fn_name, &items[1..]);
							return;
						}
						// Check for FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Int));
							return;
						}
					}
				}
				// Rounding and counting builtins build a node: its Int is the number
				if self.emit_integer_builtin(func, items) || self.reject_unresolved_call(func, items, bracket, separator) {
					return;
				}
				// `puti x`, `print x`: the emitter's own builtins give a node whose Int is the number
				if self.is_output_call(node) {
					self.emit_node_instructions(func, node);
					self.emit_call(func, "get_int_value");
					return;
				}
				// a library word gives a node too: `m.get(k, 0) + 1`, `xs.has(x) + 1`
				if crate::analyzer::call_name(items, bracket, separator).is_some_and(crate::library_words::is_runtime_word) {
					self.emit_node_instructions(func, node);
					self.emit_call(func, "get_int_value");
					return;
				}
				self.emit_statement_sequence(func, items, Self::emit_numeric_value);
			}
			// While loop: emit loop and get numeric result
			Node::Key(left, Op::Do, right) => {
				self.emit_while_loop_value(func, left, right);
			}
			Node::Key(value, Op::As, target) => self.emit_numeric_cast(func, located, value, target),
			other => self.emit_not_a_number(func, located, other),
		}
	}

	/// `floor(x)`, `count(list)`…: the builtin builds a node, its Int (raw i64 on the stack) is the number
	fn emit_integer_builtin(&mut self, func: &mut Function, items: &[Node]) -> bool {
		if let [Node::Symbol(fn_name), arguments @ ..] = items {
			if text_builtins::text_builtin_kind(fn_name, arguments.len()) == Some(Kind::Int) {
				self.emit_integer_text_builtin(func, fn_name, arguments);
				return true;
			}
		}
		let [Node::Symbol(fn_name), argument] = items else {
			return false;
		};
		let integer_builtin = ROUNDING_FUNCTIONS.contains(&fn_name.as_str())
			|| fn_name == crate::min_max::EMPTY_EXTREMUM_CALL
			|| fn_name == crate::switch::NO_CASE_CALL
			|| crate::analyzer::counting_function(fn_name, &self.ctx).is_some();
		if !integer_builtin || !self.emit_introspection_fn(func, fn_name, argument) {
			return false;
		}
		self.emit_call(func, "get_int_value");
		true
	}

	/// Emit the float value of a node onto the stack (as f64)
	/// Integers are converted to f64 for type upgrading
	fn emit_float_value(&mut self, func: &mut Function, node: &Node) {
		if let Some((list, sum_loop)) = list_dispatch::list_sum_parts(node) {
			return self.emit_list_sum(func, list, sum_loop, list_dispatch::Wanted::Float);
		}
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_float_value) {
			return;
		}
		let located = node;
		let node = node.drop_meta();
		match node {
			Node::Number(num) => {
				match num {
					Number::Int(_) | Number::BigInt(_) => {
						let value: f64 = num.clone().into();
						func.instruction(&Instruction::F64Const(Ieee64::new(value.to_bits())));
					}
					Number::Float(f) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
					}
					Number::Quotient(n, d) => {
						func.instruction(&Instruction::F64Const(Ieee64::new((*n as f64 / *d as f64).to_bits())));
					}
					Number::Complex(r, _i) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(r.to_bits())));
					}
					Number::Real(r) => {
						func.instruction(&Instruction::F64Const(Ieee64::new(r.to_f64().to_bits())));
					}
					Number::Nan => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::NAN.to_bits())));
					}
					Number::Inf => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::INFINITY.to_bits())));
					}
					Number::NegInf => {
						func.instruction(&Instruction::F64Const(Ieee64::new(f64::NEG_INFINITY.to_bits())));
					}
				};
			}
			// Variable definition/assignment: x:=42 or x=42
			Node::Key(left, Op::Define | Op::Assign, right) => {
				if let Node::Symbol(name) = left.drop_meta() {
					if let Some(position) = self.scope.lookup(name).map(|local| local.position) {
						self.emit_float_value(func, right);
						func.instruction(&Instruction::LocalTee(position));
					} else if let Some(kind) = self.emit_global_store(func, name, right) {
						if !kind.is_float() {
							self.emit_int_to_f64(func, None);
						}
					} else {
						self.emit_float_value(func, right);
						self.emit_undefined_variable(func, name);
					}
				} else {
					self.emit_malformed(func, left, "a variable to assign to");
				}
			}
			// Arithmetic operators with float
			Node::Key(left, op, right) if op.is_arithmetic() => {
				let kind = self.arithmetic_type(left, op, right);
				if self.emit_arithmetic_type_error(func, left, op, right, kind) {
					return;
				}
				if kind == Kind::Int {
					// a wholly exact expression is computed exactly and rounded once: `float y = 0.1+0.2` is 0.3
					self.emit_numeric_value(func, node);
					let range = self.int_range(node);
					self.emit_int_to_f64(func, range);
					return;
				}
				self.emit_float_value(func, left);
				self.emit_float_value(func, right);
				self.emit_float_arithmetic(func, op);
			}
			// Suffix operators: x² = x*x, x³ = x*x*x (returns f64)
			Node::Key(left, Op::Square, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
			}
			Node::Key(left, Op::Cube, _) => {
				self.emit_float_value(func, left);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
				self.emit_float_value(func, left);
				func.instruction(&Instruction::F64Mul);
			}
			// Prefix operators: √x = sqrt(x) (returns f64)
			Node::Key(left, Op::Sqrt, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Sqrt);
			}
			// Prefix negation: -x (returns f64)
			Node::Key(left, Op::Neg, right) if matches!(left.drop_meta(), Node::Empty) => {
				func.instruction(&Instruction::F64Const(0.0.into()));
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Sub);
			}
			// Prefix abs: ‖x‖ (returns f64)
			Node::Key(left, Op::Abs, right) if matches!(left.drop_meta(), Node::Empty) => {
				self.emit_float_value(func, right);
				func.instruction(&Instruction::F64Abs);
			}
			// `v as float` is v's f64; any other cast's exact value converted
			Node::Key(value, Op::As, _) if self.get_type(node).is_float() && !matches!(value.drop_meta(), Node::Text(_) | Node::Char(_)) => {
				self.emit_float_value(func, value);
			}
			Node::Key(list, Op::Hash, index) if self.is_typed_list(list) => self.emit_typed_element_float(func, list, index),
			Node::Key(_, Op::As, _) => {
				self.emit_numeric_value(func, node);
				self.emit_int_to_f64(func, None);
			}
			// Variable lookup (local or global) - convert i64 to f64 if needed
			Node::Symbol(name) => {
				if self.emit_typed_list_as_node(func, name) {
					self.emit_call(func, "get_int_value");
					self.emit_int_to_f64(func, None);
				} else if let Some(local) = self.scope.lookup(name) {
					func.instruction(&Instruction::LocalGet(local.position));
					if local.kind.is_ref() {
						self.emit_call(func, "get_int_value");
						self.emit_int_to_f64(func, None);
					} else if !local.kind.is_float() {
						self.emit_int_to_f64(func, None);
					}
				} else if let Some(&(idx, kind)) = self.ctx.user_globals.get(name) {
					func.instruction(&Instruction::GlobalGet(idx));
					if !kind.is_float() {
						self.emit_int_to_f64(func, None);
					}
				} else {
					self.emit_undefined_variable(func, name);
				}
			}
			// Function calls and statement sequences
			Node::List(items, bracket, separator) if !items.is_empty() => {
				// Check for function call: [Symbol("funcname"), arg1, arg2, ...]
				if items.len() >= 2 {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						// Check for FFI function call
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &items[1..], Some(Kind::Float));
							return;
						}
						// Check for user function call
						if self.ctx.user_functions.contains_key(fn_name) {
							self.emit_user_function_call_float(func, fn_name, &items[1..]);
							return;
						}
					}
				}
				// Check for zero-arg function call: (funcname)
				if items.len() == 1 && *bracket == Bracket::Round {
					if let Node::Symbol(fn_name) = items[0].drop_meta() {
						if self.ctx.ffi_imports.contains_key(fn_name) {
							self.emit_ffi_call(func, fn_name, &[], Some(Kind::Float));
							return;
						}
					}
				}
				if self.emit_integer_builtin(func, items) {
					self.emit_int_to_f64(func, None);
					return;
				}
				if self.reject_unresolved_call(func, items, bracket, separator) {
					return;
				}
				// Statement sequence: execute all, return last as float
				for (i, item) in items.iter().enumerate() {
					if i < items.len() - 1 {
						self.emit_discarded_statement(func, item, Self::emit_numeric_value);
						func.instruction(&Instruction::Drop);
					} else {
						// Last item as float
						self.emit_float_value(func, item);
					}
				}
			}
			other => self.emit_not_a_number(func, located, other),
		}
	}

	/// Emit a range as a list of integers
	/// inclusive: true for .../ (0...3 = [0,1,2,3]), false for .. (0..3 = [0,1,2])
	fn emit_range(&mut self, func: &mut Function, start: &Node, end: &Node, inclusive: bool) {
		let (Node::Number(Number::Int(start_val)), Node::Number(Number::Int(end_val))) = (start.drop_meta(), end.drop_meta()) else {
			let bound = if matches!(start.drop_meta(), Node::Number(Number::Int(_))) { end } else { start };
			self.emit_malformed(func, bound, "a constant integer as range bound");
			return;
		};
		let (start_val, end_val) = (*start_val, *end_val);
		let actual_end = if inclusive { end_val + 1 } else { end_val };
		let items: Vec<Node> = (start_val..actual_end).map(|i| Node::Number(Number::Int(i))).collect();
		if items.is_empty() {
			self.emit_call(func, "new_empty");
			return;
		}
		self.emit_list_structure(func, &items, &Bracket::Square);
	}

	/// Emit a list as linked cons cells
	fn emit_list_structure(&mut self, func: &mut Function, items: &[Node], bracket: &Bracket) {
		let bracket_info = match bracket {
			Bracket::Curly => 0i64,
			Bracket::Square => 1,
			Bracket::Round => 2,
			Bracket::Less => 3,
			Bracket::Other(_, _) => 4,
			Bracket::None => 5,
		};

		// Emit first item
		self.emit_node_instructions(func, &items[0]);

		// Emit rest as a proper linked list
		// The value field must always be a list node (or null), never an element directly
		if items.len() > 1 {
			// Recursively build the rest of the list
			// This ensures proper cons-cell structure: (data=first, value=list_node_for_rest)
			self.emit_list_structure(func, &items[1..], bracket);
		} else {
			// Single element list: rest is null
			self.emit_node_null(func);
		}

		// bracket_info
		func.instruction(&Instruction::I64Const(bracket_info));

		// Call new_list if available, otherwise inline struct.new
		if self.ctx.func_registry.contains("new_list") {
			self.emit_call(func, "new_list");
		} else {
			// Inline: kind = (bracket_info << 8) | List
			// We need to reconstruct since stack has: first, rest, bracket_info
			// Actually we need to reorder. Let's use locals.
			// For simplicity, always emit new_list function when needed
			self.emit_inline_list(func, bracket_info);
		}
	}

	fn emit_inline_list(&mut self, func: &mut Function, _bracket_info: i64) {
		// Stack: first, rest, bracket_info
		// Need: kind, data(first), value(rest)
		// Use struct.new directly with proper ordering

		// This is complex due to stack order. For now, require new_list function.
		// Pop bracket_info (already on stack as i64)
		// Compute kind
		func.instruction(&Instruction::I64Const(8));
		func.instruction(&Instruction::I64Shl);
		self.emit_kind(func, Kind::List);
		func.instruction(&Instruction::I64Or);
		// But now we have: first, rest, kind - wrong order!
		// We need: kind, first, rest
		// This requires locals or restructuring.

		// new_list is always emitted; reaching this is a compiler bug, reported as an error value
		self.emit_type_error(func, "internal error: new_list is not available".to_string());
	}

	#[cfg(not(feature = "validate"))]
	fn try_validate_wasm(_bytes: &[u8]) -> Result<(), String> {
		Ok(()) // the engine that runs the module validates it (the browser build, web/playground)
	}

	#[cfg(feature = "validate")]
	fn try_validate_wasm(bytes: &[u8]) -> Result<(), String> {
		let mut features = WasmFeatures::default();
		features.set(WasmFeatures::REFERENCE_TYPES, true);
		features.set(WasmFeatures::GC, true);
		features.set(WasmFeatures::EXCEPTIONS, true);
		let mut validator = Validator::new_with_features(features);
		match validator.validate_all(bytes) {
			Ok(_) => {
				trace!("✓ WASM validation with GC features passed");
				Ok(())
			}
			Err(e) => Err(format!("{}", e)),
		}
	}

	/// The module bytes; a module that fails validation is a compiler bug, only test helpers may treat it as a panic
	pub fn finish(self) -> Vec<u8> {
		self.try_finish().unwrap_or_else(|message| panic!("{message}"))
	}

	/// The module bytes, or the validation failure as an error message
	pub fn try_finish(mut self) -> Result<Vec<u8>, String> {
		// WASM section order: types, imports, functions, memory, globals, exports, code, data, names
		self.module.section(self.type_manager.types());
		if self.ctx.func_registry.import_count() > 0 {
			self.module.section(self.import_manager.imports());
		}
		self.module.section(&self.functions);
		self.module.section(&self.memory);
		if !self.tags.is_empty() {
			self.module.section(&self.tags);
		}
		if self.next_global_idx > 0 {
			self.module.section(&self.globals);
		}
		self.module.section(&self.exports);
		// functions taken by ref.func: one declarative element segment for all (a module has one element section)
		let declared_functions: Vec<u32> = self.closures.declared_functions().into_iter().chain(self.witness_dispatcher()).collect();
		if !declared_functions.is_empty() {
			let mut elements = ElementSection::new();
			elements.declared(Elements::Functions(declared_functions.into()));
			self.module.section(&elements);
		}
		self.module.section(&self.code);
		// Get data section from string table
		self.module.section(self.string_table.data_section());
		self.emit_names();
		self.module.section(&self.names);

		let bytes = self.module.finish();
		write_debug_module(&bytes);
		Self::try_validate_wasm(&bytes).map_err(|message| format!("internal error: WASM validation failed: {message}"))?;
		Ok(bytes)
	}

	/// The name section: subsections in id order (module, functions, types, globals, fields, tags), every map by index
	fn emit_names(&mut self) {
		self.names.module("wasp_compact");

		let functions: Vec<(u32, String)> = self.ctx.func_registry.all().iter()
			.map(|f| (f.call_index as u32, f.name.clone()))
			.chain(self.ctx.user_functions.values().filter_map(|function| Some((function.func_index?, function.name.clone()))))
			.chain(self.closures.entry_names())
			.chain(self.tuple_packers.iter().map(|(function, index)| (*index, crate::tuples::packer_name(function))))
			.collect();
		self.names.functions(&name_map(&mut functions.iter().map(|(idx, name)| (*idx, name.as_str())).collect()));

		let tm = &self.type_manager;
		let mut types = vec![(tm.string_type, "String"), (tm.i64_box_type, "i64box"), (tm.f64_box_type, "f64box"), (tm.node_type, "Node"),
			(tm.int_array_type, "IntArray"), (tm.int_list_type, "IntList"), (tm.float_array_type, "FloatArray"), (tm.float_list_type, "FloatList")];
		types.extend(self.ctx.user_type_indices.iter().map(|(name, idx)| (*idx, name.as_str())));
		self.names.types(&name_map(&mut types));

		if self.next_global_idx > 0 {
			const KIND_GLOBALS: [&str; 12] = ["kind_empty", "kind_int", "kind_float", "kind_text", "kind_codepoint", "kind_symbol",
				"kind_key", "kind_block", "kind_list", "kind_data", "kind_meta", "kind_error"];
			let mut globals: Vec<(u32, &str)> = KIND_GLOBALS.iter().enumerate()
				.filter(|(idx, _)| (*idx as u32) < self.next_global_idx).map(|(idx, name)| (idx as u32, *name)).collect();
			globals.extend(self.extra_global_names.iter().copied());
			self.names.globals(&name_map(&mut globals));
		}

		let mut fields: Vec<(u32, Vec<(u32, &str)>)> = vec![
			(tm.node_type, vec![(0, "kind"), (1, "data"), (2, "value")]),
			(tm.string_type, vec![(0, "ptr"), (1, "len")]),
			(tm.i64_box_type, vec![(0, "value")]),
			(tm.f64_box_type, vec![(0, "value")]),
			(tm.int_list_type, vec![(0, "length"), (1, "items")]),
			(tm.float_list_type, vec![(0, "length"), (1, "items")]),
		];
		for type_def in self.ctx.type_registry.types() {
			if let Some(&type_idx) = self.ctx.user_type_indices.get(&type_def.name) {
				fields.push((type_idx, type_def.fields.iter().enumerate().map(|(i, field)| (i as u32, field.name.as_str())).collect()));
			}
		}
		fields.sort_by_key(|(type_idx, _)| *type_idx);
		fields.dedup_by_key(|(type_idx, _)| *type_idx);
		let mut type_field_names = IndirectNameMap::new();
		for (type_idx, mut names) in fields {
			type_field_names.append(type_idx, &name_map(&mut names));
		}
		self.names.fields(&type_field_names);

		if let Some(tag_names) = self.error_tag_names() {
			self.names.tags(&tag_names);
		}
	}

	pub fn get_unused_functions(&self) -> Vec<String> {
		self.ctx.func_registry
			.all()
			.iter()
			.map(|f| f.name.clone())
			.filter(|name: &String| !self.ctx.used_functions.contains(name.as_str()))
			.collect()
	}

	pub fn get_used_functions(&self) -> Vec<&'static str> {
		self.ctx.used_functions.iter().copied().collect()
	}

	/// Emit a standalone WASM module that returns a raw GC struct instance.
	/// This is the standard path for returning user-defined GC objects.
	///
	/// # Arguments
	/// * `type_def` - The type definition for the struct
	/// * `field_values` - Field values as (name, RawFieldValue) pairs
	///
	/// # Returns
	/// WASM bytes that can be loaded and executed to get the GC struct
	pub fn emit_raw_struct(type_def: &TypeDef, field_values: &[RawFieldValue]) -> Vec<u8> {
		use wasm_encoder::StorageType::Val;
		use wasm_encoder::*;

		let mut module = Module::new();

		// Collect string data and compute offsets
		let mut string_data = Vec::new();
		let mut string_offsets: Vec<(usize, usize)> = Vec::new(); // (offset, len) for each string field
		let mut current_offset = 0usize;

		for value in field_values {
			if let RawFieldValue::String(s) = value {
				string_offsets.push((current_offset, s.len()));
				string_data.extend_from_slice(s.as_bytes());
				current_offset += s.len();
			} else {
				string_offsets.push((0, 0)); // placeholder for non-strings
			}
		}

		// Build types section
		let mut types = TypeSection::new();

		// Type 0: $String = struct { ptr: i32, len: i32 }
		types.ty().struct_(vec![
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			},
			FieldType {
				element_type: Val(ValType::I32),
				mutable: false,
			},
		]);
		let string_type_idx = 0u32;

		// Type 1: User struct type
		let string_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(string_type_idx),
		};
		let struct_fields: Vec<FieldType> = type_def
			.fields
			.iter()
			.map(|f| {
				let element_type = match f.type_name.as_str() {
					"i64" | "Int" | "long" => Val(ValType::I64),
					"i32" | "int" => Val(ValType::I32),
					"f64" | "Float" | "double" => Val(ValType::F64),
					"f32" | "float" => Val(ValType::F32),
					"String" | "Text" | "string" => Val(Ref(string_ref)),
					_ => Val(ValType::I64), // default
				};
				FieldType {
					element_type,
					mutable: false,
				}
			})
			.collect();
		types.ty().struct_(struct_fields);
		let struct_type_idx = 1u32;

		// Type 2: main() -> ref $StructType
		let struct_ref = RefType {
			nullable: false,
			heap_type: HeapType::Concrete(struct_type_idx),
		};
		types.ty().func_type(&FuncType::new([], [Ref(struct_ref)]));

		module.section(&types);

		// Function section
		let mut functions = FunctionSection::new();
		functions.function(2); // main uses type 2
		module.section(&functions);

		// Memory section (only if we have strings)
		let has_strings = !string_data.is_empty();
		if has_strings {
			let mut memories = MemorySection::new();
			memories.memory(MemoryType {
				minimum: 1,
				maximum: None,
				memory64: false,
				shared: false,
				page_size_log2: None,
			});
			module.section(&memories);
		}

		// Export section
		let mut exports = ExportSection::new();
		if has_strings {
			exports.export("memory", ExportKind::Memory, 0);
		}
		exports.export("main", ExportKind::Func, 0);
		module.section(&exports);

		// Code section
		let mut codes = CodeSection::new();
		let mut func = Function::new([]);

		// Emit field values in order
		let mut string_idx = 0usize;
		for value in field_values.iter() {
			match value {
				RawFieldValue::I64(v) => {
					func.instruction(&Instruction::I64Const(*v));
				}
				RawFieldValue::I32(v) => {
					func.instruction(&I32Const(*v));
				}
				RawFieldValue::F64(v) => {
					func.instruction(&Instruction::F64Const(Ieee64::new(v.to_bits())));
				}
				RawFieldValue::F32(v) => {
					func.instruction(&Instruction::F32Const(Ieee32::new(v.to_bits())));
				}
				RawFieldValue::String(_) => {
					let (ptr, len) = string_offsets[string_idx];
					func.instruction(&I32Const(ptr as i32));
					func.instruction(&I32Const(len as i32));
					func.instruction(&Instruction::StructNew(string_type_idx));
					string_idx += 1;
				}
			}
		}
		func.instruction(&Instruction::StructNew(struct_type_idx));
		func.instruction(&Instruction::End);
		codes.function(&func);
		module.section(&codes);

		// Data section for strings
		if has_strings {
			let mut data = DataSection::new();
			data.active(0, &ConstExpr::i32_const(0), string_data.iter().copied());
			module.section(&data);
		}

		// Name section for field name resolution, subsections in ascending id order (function 1, type 4, field 10)
		let mut names = NameSection::new();

		// Function names (subsection 1 comes before types and fields)
		let mut func_names = NameMap::new();
		func_names.append(0, "main");
		names.functions(&func_names);

		// Type names
		let mut type_names = NameMap::new();
		type_names.append(string_type_idx, "String");
		type_names.append(struct_type_idx, &type_def.name);
		names.types(&type_names);

		// Field names for structs
		let mut type_field_names = IndirectNameMap::new();

		// String struct fields
		Self::append_string_field_names(&mut type_field_names, string_type_idx);

		// User struct fields
		let mut struct_fields_names = NameMap::new();
		for (i, field) in type_def.fields.iter().enumerate() {
			struct_fields_names.append(i as u32, &field.name);
		}
		type_field_names.append(struct_type_idx, &struct_fields_names);
		names.fields(&type_field_names);

		module.section(&names);

		let bytes = module.finish();
		write_debug_module(&bytes);
		bytes
	}
}

/// A name map in index order (the name section requires it), one name per index
fn name_map(names: &mut Vec<(u32, &str)>) -> NameMap {
	names.sort_by_key(|(idx, _)| *idx);
	names.dedup_by_key(|(idx, _)| *idx);
	let mut map = NameMap::new();
	for (idx, name) in names.iter() {
		map.append(*idx, name);
	}
	map
}

#[cfg(feature = "native")]
/// Run raw struct WASM and return GcObject wrapped in Node::Data
pub fn run_raw_struct(wasm_bytes: &[u8]) -> Result<Node, String> {
	use wasmtime::{Linker, Module, Val};

	// Register WASM metadata for field name lookup in Debug output
	let module_id = crate::gc_traits::register_gc_types_from_wasm(wasm_bytes).ok();

	let engine = gc_engine();
	let mut store = crate::util::fueled_store(&engine, ());
	let module = Module::new(&engine, wasm_bytes).map_err(|e: wasmtime::Error| e.to_string())?;

	let linker = Linker::new(&engine);
	let instance = linker
		.instantiate(&mut store, &module)
		.map_err(|e: wasmtime::Error| e.to_string())?;

	let main = instance
		.get_func(&mut store, "main")
		.ok_or_else(|| "no main function".to_string())?;

	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results)
		.map_err(|e: wasmtime::Error| e.to_string())?;

	let gc_obj = ErgonomicGcObject::new(results[0], store, Some(instance))
		.map_err(|e: anyhow::Error| e.to_string())?
		.with_module(module_id);

	Ok(crate::node::data(gc_obj))
}

/// Find a struct instantiation anywhere in the AST using TypeRegistry
/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
fn find_struct_instantiation(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
	find_instantiation_recursive(registry, node)
}


fn find_instantiation_recursive(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
	use crate::type_kinds::extract_instance_values;
	let node = node.drop_meta();
	match node {
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(type_def) = registry.get_by_name(name) {
					if let Some((_, values)) = extract_instance_values(node) {
						return Some((type_def.clone(), values));
					}
				}
			}
			// Recurse into right
			find_instantiation_recursive(registry, right)
		}
		Node::List(items, _, _) => {
			for item in items {
				if let Some(result) = find_instantiation_recursive(registry, item) {
					return Some(result);
				}
			}
			None
		}
		_ => None,
	}
}

// Re-export eval function for tests
/// The program written in `code`, or in the file `code` names: a file sees the definitions of its folder (D15)
pub fn eval(code: &str) -> Node {
	let names_a_file = !code.contains('\n') && (code.ends_with(".wasp") || code.ends_with(".warp"));
	match names_a_file.then(|| std::fs::read_to_string(code).ok()).flatten() {
		Some(source) => crate::modules::with_program_file(std::path::Path::new(code), || eval_source(&source)),
		None => eval_source(code),
	}
}

fn eval_source(code: &str) -> Node {
	crate::diagnostic::begin_program(); // only the guesses made for this program explain its errors
	match lawful_program(code) {
		Ok(program) => {
			let exclusive_range = has_exclusive_range(&program);
			explain_runtime_error(eval_parsed(program, code), exclusive_range)
		}
		Err(violation) => violation,
	}
}

fn has_exclusive_range(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::Range, _) => true,
		Node::Key(left, _, right) => has_exclusive_range(left) || has_exclusive_range(right),
		Node::List(items, _, _) => items.iter().any(has_exclusive_range),
		_ => false,
	}
}

/// A runtime error fails far from its cause: it names the range hint and the defaults unanswered Asks took
fn explain_runtime_error(result: Node, exclusive_range: bool) -> Node {
	let assumptions = crate::diagnostic::take_assumptions();
	let Node::Error(reason) = &result else { return result };
	let Node::Text(message) = reason.drop_meta() else { return result };
	let mut lines = vec![message.clone()];
	if exclusive_range && message.starts_with(INDEX_OUT_OF_RANGE) {
		lines.push(RANGE_END_HINT.to_string());
	}
	let mut assumed: Vec<(String, Vec<String>)> = Vec::new();
	for assumption in assumptions {
		let guess = format!("{}; fix: {}", assumption.message, assumption.fix.unwrap_or_default());
		let position = format!("{}:{}", assumption.line, assumption.column);
		match assumed.iter_mut().find(|(known, _)| *known == guess) {
			Some((_, positions)) => positions.push(position),
			None => assumed.push((guess, vec![position])),
		}
	}
	lines.extend(assumed.into_iter().map(|(guess, positions)| format!("assumed at {}: {guess}", positions.join(", "))));
	match lines.len() {
		1 => result,
		_ => crate::node::error(&lines.join("\n  ")),
	}
}

/// Parse the source and check its laws; `Err` is the violation.
fn lawful_program(code: &str) -> Result<Node, Node> {
	let lawful = crate::law::separate_laws(crate::diagnostic::in_source_mode(code, || WaspParser::parse(code)));
	match crate::law::assert_laws(&lawful, code) {
		Some(violation) => Err(violation),
		None => Ok(lawful.program),
	}
}

/// Evaluate foreign data with an empty capability set: a program that would call any host, WASI or FFI
/// function is refused before it is compiled. To only read data, use `parse_data`, which evaluates nothing.
pub fn eval_untrusted(code: &str) -> Node {
	crate::diagnostic::begin_program();
	let program = WaspParser::parse(code);
	match crate::effects::EffectReport::of(&program).externals.keys().next() {
		Some(external) => crate::node::error(&format!("untrusted code has no capabilities, refusing to call {external}")),
		None => eval_parsed(program, code),
	}
}

/// A compiled program and the host capabilities its imports need.
pub struct CompiledModule {
	pub bytes: Vec<u8>,
	needs_host: bool,
	needs_wasi: bool,
	needs_ffi: bool,
}

/// Everything before code generation. `Err` is the final value of the program when it needs no module
/// (a constant answer, an error, a denied capability).
fn lower_for_emission(node: Node) -> Result<Node, Node> {
	use crate::effects::{without_constraints, Capability, EffectReport};

	let node = crate::analyzer::lower_negated_calls(crate::versions::lower_versions(crate::meta_entries::lower(crate::type_name_matching::lower(crate::modules::resolve(crate::host::lower_aliases(crate::mutation::lower(crate::tuples::lower(node))))))));
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	let node = crate::interpolation::lower(crate::injection::lower_templates(node)?);
	let node = crate::function_equality::decide_comparisons(node);
	if let Node::Error(_) = node {
		return Err(node);
	}
	if let Some(answer) = crate::time::answer(&node) {
		return Err(answer);
	}
	if let Some(answer) = crate::units::answer(&node) {
		return Err(answer);
	}
	if let Some(answer) = crate::real::answer(&node) {
		return Err(answer);
	}
	let node = crate::type_tests::lower(crate::traits::lower_declarations(node));
	let node = crate::ambiguous_forms::lower(node);
	let node = crate::analyzer::lower_list_times(node);
	let node = crate::lambdas::lower(node);
	let node = crate::function_values::lower(node);
	let node = crate::closures::lower(node);
	let node = crate::lambdas::lower_strict(node);
	let node = crate::real::lower(node);
	let node = crate::traits::lower_conformances(crate::type_constructor::lower(node));
	let node = crate::min_max::lower(node);
	let node = crate::declarations::lower(node);
	let node = crate::switch::lower(node);
	let node = crate::phrase_words::lower(node);
	let node = crate::traits::lower_dispatch(crate::library_words::lower(node));
	if let Some(error) = node.first_error() {
		return Err(error.clone());
	}
	let effects = EffectReport::of(&node);
	if let Some(answer) = effects.answer(&node) {
		return Err(answer);
	}
	if let Some((name, capability)) = effects.denied(&Capability::GRANTED_BY_EVAL) {
		return Err(crate::node::error(&format!(
			"capability denied: {name} needs the {} capability, which eval does not grant", capability.name())));
	}
	let node = crate::analyzer::resolve_main_variable_assignments(without_constraints(node))?;
	if let Some(error) = crate::analyzer::diagnose(&node) {
		return Err(error);
	}
	crate::diagnostic::report(&crate::analyzer::lint(&node))?;
	Ok(crate::analyzer::lower_declarations(crate::analyzer::resolve_data_scope(node)))
}

fn emit_module(node: &Node) -> Result<CompiledModule, Node> {
	use crate::effects::Capability;
	let mut emitter = WasmGcEmitter::new();
	emitter.emit_for_node(node);
	if let Some(type_error) = emitter.type_error() {
		return Err(type_error);
	}
	let needs_host = emitter.imports(Capability::Host);
	let needs_wasi = emitter.imports(Capability::Wasi);
	let needs_ffi = emitter.imports(Capability::Ffi);
	let bytes = emitter.try_finish().map_err(|message| crate::node::error(&message))?;
	Ok(CompiledModule { bytes, needs_host, needs_wasi, needs_ffi })
}

/// Leave the last module in `test.wasm` for inspection; a read-only directory must not fail the program
fn write_debug_module(bytes: &[u8]) {
	if let Err(failure) = std::fs::write("test.wasm", bytes) {
		warn!("could not write test.wasm: {failure}");
	}
}

/// Compile source text to a wasm module without running it. `Err` carries the error, or the constant
/// answer of a program that needs no module.
pub fn compile(code: &str) -> Result<CompiledModule, Node> {
	crate::diagnostic::begin_program();
	crate::diagnostic::in_program_mode(lawful_program(code)?, |program| choose_module(&lower_for_emission(program)?))
}

/// `TypeName:{field:value, ...}` compiles to a raw struct module, everything else to the standard Node encoding.
fn raw_struct_module(node: &Node) -> Option<Vec<u8>> {
	let mut type_registry = crate::type_kinds::TypeRegistry::new();
	crate::analyzer::collect_all_types(&mut type_registry, node);
	let (type_def, field_values) = find_struct_instantiation(&type_registry, node)?;
	Some(WasmGcEmitter::emit_raw_struct(&type_def, &field_values))
}

fn choose_module(node: &Node) -> Result<CompiledModule, Node> {
	match raw_struct_module(node) {
		Some(bytes) => Ok(CompiledModule { bytes, needs_host: false, needs_wasi: false, needs_ffi: false }),
		None => emit_module(node),
	}
}

/// Compile and run an already parsed program; imports follow its resolved effects.
pub fn eval_parsed(node: Node, _code: &str) -> Node {
	crate::diagnostic::in_program_mode(node, eval_program)
}

fn eval_program(node: Node) -> Node {
	let node = match lower_for_emission(node) {
		Ok(node) => node,
		Err(final_value) => return final_value,
	};

	#[cfg(feature = "native")]
	if let Some(wasm_bytes) = raw_struct_module(&node) {
		match run_raw_struct(&wasm_bytes) {
			Ok(result) => return result,
			Err(e) => warn!("raw struct eval failed: {}", e),
		}
	}

	// Fallback to standard Node encoding
	match emit_module(&node) {
		Ok(module) => run_module(module),
		Err(type_error) => type_error,
	}
}

/// Run a compiled program with the linker its imports need
#[cfg(feature = "native")]
fn run_module(CompiledModule { bytes, needs_host, needs_wasi, needs_ffi }: CompiledModule) -> Node {
	use crate::wasm_reader::{read_bytes_with_imports, Imports};
	let result = if needs_ffi || needs_wasi || needs_host {
		read_bytes_with_imports(&bytes, Imports { host: needs_host, wasi: needs_wasi, ffi: needs_ffi })
	} else {
		read_bytes(&bytes)
	};
	result.unwrap_or_else(failed_run)
}

/// Without wasmtime the embedding host runs the program (the browser playground: web.rs)
#[cfg(not(feature = "native"))]
fn run_module(module: CompiledModule) -> Node {
	crate::web::run_in_host(&module.bytes)
}

/// The run used up its fuel: it probably does not terminate, or needs a larger budget
pub fn out_of_fuel(steps: u64) -> Node {
	crate::node::error(&format!(
		"out of fuel after {steps} steps: the program may not terminate (raise the budget with {}=<steps> or --fuel <steps>)",
		crate::util::FUEL_VARIABLE))
}

/// A trap is a runtime error of the program; any other failure (link, instantiation, validation) is an error
/// of the compiler or the environment. Both become error values, never the unevaluated program.
#[cfg(feature = "native")]
pub(crate) fn failed_run(failure: anyhow::Error) -> Node {
	let trap = match failure.downcast_ref::<wasmtime::Trap>() {
		None => return crate::node::error(&format!("could not run the program: {failure:#}")),
		Some(wasmtime::Trap::OutOfFuel) => return out_of_fuel(crate::util::fuel_budget()),
		Some(trap) => trap,
	};
	trap_error(&format!("{:?}", failure), trap.to_string())
}

/// The runtime error a trap means, read from its trace: the function names on the stack and the `trap detail: …`
/// line; `trap` (the engine's own words) when the trace names none. Shared by wasmtime and the browser (web.rs).
pub fn trap_error(trace: &str, trap: String) -> Node {
	let missing_field = trace.split_once(list_ops::NO_FIELD_PREFIX).map(|(_, rest)| {
		let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == crate::node::ATTRIBUTE_MARK).collect();
		format!("no field {name}")
	});
	let no_case = trace.split_once(crate::switch::NO_CASE_PREFIX).map(|(_, rest)| {
		let label: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
		match trace.split_once(TRAP_DETAIL_PREFIX).and_then(|(_, rest)| rest.lines().next()) {
			Some(value) => format!("no case for {label} = {value}"),
			None => format!("no case for {label}"),
		}
	});
	let overflow = trace.split_once(crate::fixed_width::OVERFLOW_PREFIX).map(|(_, rest)| {
		let type_name: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
		crate::fixed_width::overflow_message(&type_name)
	});
	let runtime_error = missing_field.or(no_case).or(overflow).or_else(|| list_ops::RUNTIME_ERRORS.iter().find(|name| trace.contains(*name)).map(|name| list_ops::runtime_error_message(name)));
	let exact_trap = EXACT_TRAP_MESSAGES.iter().find(|(function, _)| trace.contains(function)).map(|(_, message)| message.to_string());
	let message = runtime_error.or(exact_trap).unwrap_or(trap);
	crate::node::error(&message)
}
