//! WASM GC code emitter - generates WebAssembly modules with GC support

mod user_function_calls;
mod arithmetic;
mod globals;
mod control_flow;
mod casts;
mod values;
mod big_int;
pub mod cells;
mod closures;
pub use closures::{CLOSURE_CAPTURED, CLOSURE_REBUILD};
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
pub(crate) mod linear_arrays;
mod list_emitter;
mod list_dispatch;
mod library_ops;
mod text_unicode;
pub(crate) mod list_ops;
mod list_abi;
mod map_backend;
mod struct_backend;
pub use map_backend::MAP_COPY_SUFFIX;
mod loop_control;
pub(crate) use loop_control::mark_step;
pub(crate) mod text_builtins;
mod reflection;
mod string_table;
mod type_manager;
mod try_guard;
mod tuple_emitter;
pub use try_guard::{CAUGHT_ERROR, RAN_WITHOUT_ERROR};
mod witness;
pub(crate) mod wasi_emitter;

pub use big_int::{is_fixnum, EXACT_BUILDERS, INT_RUNTIME};

/// Something emission found it must have that the analysis pass did not request
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Need {
	/// A runtime function, by registry name
	Function(&'static str),
	/// A libm import, keyed like ffi_imports (`m.pow`) so it never shadows a user function
	MathImport(&'static str),
}

/// The texts `as bool` reads as false
const FALSY_TEXTS: [&str; 8] = ["", "0", "false", "no", "ø", "nil", "null", "none"];
/// wasmtime's words for an integer division or remainder by zero
const ENGINE_DIVIDE_BY_ZERO: &str = "integer divide by zero";
/// The position a diagnostic without a recorded one names (diagnostic::Diagnostic::at)
const UNKNOWN_POSITION: &str = " at 0:0";
/// Between a diagnostic's message and its fix
const FIX_SEPARATOR: &str = "; fix: ";
/// Builtins that write their argument and give it back: `puti i` as a statement of a loop body
/// The export name prefix of a closure's captured variable, `capture·add·k` (tasks copy them to their instance)
pub const CAPTURE_EXPORT_PREFIX: &str = "capture·";
/// The WASI output words besides print: each gives an Int (analyzer)
pub const OUTPUT_WORDS: [&str; 4] = ["puts", "puti", "putl", "putf"];
const OUTPUT_CALLS: [&str; 5] = ["print", OUTPUT_WORDS[0], OUTPUT_WORDS[1], OUTPUT_WORDS[2], OUTPUT_WORDS[3]];

/// Builtins that round a float to an exact Int
pub(crate) const ROUNDING_FUNCTIONS: [&str; 6] = ["ceil", "floor", "round", "round_half_up", "round_half_even", crate::wasp_parser::FLOOR_QUOTIENT];

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
/// Why a unit word is an undefined variable: quantities are computed in constant expressions only
const UNIT_AT_RUN_TIME: &str = " (a unit: quantities compute only in constant expressions so far, not yet in functions, loops, lists, branches or print; notes/units_runtime.md)";
/// The global behind the export `trap_detail`: not a wasp name, so no user global meets it
const TRAP_DETAIL_GLOBAL: &str = "trap·detail";

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
use crate::function::Function as FuncDef;
#[cfg(feature = "native")]
use crate::gc_traits::GcObject as ErgonomicGcObject;
use crate::node::{Bracket, Node, Separator};
use crate::operators::{op_to_code, Op};
use crate::type_kinds::{field_def_to_val_type, Kind, RawFieldValue, TypeDef, TypeRegistry, KIND_MASK};
#[cfg(feature = "native")]
use crate::util::gc_engine;
use log::{trace, warn};
use std::collections::HashMap;
use wasm_encoder::*;
use Instruction as I;
#[cfg(feature = "validate")]
use wasmparser::{Validator, WasmFeatures};
use Instruction::I32Const;
pub use crate::pipeline::{compile, compile_printing_result, eval, eval_parsed, eval_untrusted, lower, out_of_fuel, CompiledModule};
use crate::pipeline::returned_error_message;
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
	/// Emitting data, not code: the value of an object entry or a quoted form, where unknown words stay words (P62)
	data_context: bool,
	error_catching: Option<try_guard::ErrorCatching>, // the tag and globals of that `try`
	memo_caches: HashMap<i64, (u32, u32)>, // per memoized function id: the globals of its values and known flags (memoization.rs)
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
	returns_float: bool, // the function being compiled returns an f64, so `return 7` returns 7.0
	returned_tuple: Vec<Kind>, // the value kinds of the tuple function being compiled (`return a, b`), else empty
	tuple_packers: HashMap<String, u32>, // tuple function → its `f$list` packer (tuple_emitter.rs)
	text_heap_global: Option<u32>, // bump pointer for texts built at runtime, in memory grown past the string table
	// Needs the analyzer could not foresee (they depend on inferred types); emission reruns with them
	discovered_needs: std::collections::HashSet<Need>,
	type_errors: Vec<String>,
	/// The latest source position among the nodes being emitted (note_position)
	source_position: Option<(usize, usize)>,
	loop_labels: Vec<loop_control::LoopLabels>, // enclosing loops of the code being emitted, innermost last
	typed_lists: HashMap<String, list_dispatch::TypedList>, // list variables of the body being emitted held as typed arrays
	bounded_counters: std::collections::HashSet<String>, // loop counters of the body being emitted proven to stay in 0..i32::MAX (big_int.rs)
	typed_maps: std::collections::HashSet<String>, // map variables of the body being emitted held as hash tables (map_backend.rs)
	typed_structs: HashMap<String, String>, // instance variables of the body being emitted held as GC structs, with their class (struct_backend.rs)
	instance_types: HashMap<String, struct_backend::InstanceType>, // the `P·instance` struct type of each class
	/// `x = (t = x; …; t)`, an inlined call updating the list it is given back: x and t share one array, no copies
	moved_lists: Vec<(String, String)>,
	/// The array calling convention of the user functions that have one (list_abi.rs)
	list_abi: HashMap<String, list_abi::ListAbi>,
	/// The function being compiled returns a $NodeList
	returns_list: bool,
	compare_witness: Option<witness::WitnessTable>, // runtime dispatch of Comparable to user types (witness.rs)
	closures: closures::ClosureTypes,
	/// The user function whose body is being compiled; None in main
	compiling: Option<String>,
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
			data_context: false,
			error_catching: None,
			memo_caches: HashMap::new(),
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
			returns_float: false,
			returned_tuple: vec![],
			tuple_packers: HashMap::new(),
			text_heap_global: None,
			discovered_needs: Default::default(),
			type_errors: Vec::new(),
			source_position: None,
			loop_labels: Vec::new(),
			typed_lists: HashMap::new(),
			bounded_counters: std::collections::HashSet::new(),
			typed_maps: std::collections::HashSet::new(),
			typed_structs: HashMap::new(),
			instance_types: HashMap::new(),
			moved_lists: Vec::new(),
			list_abi: HashMap::new(),
			returns_list: false,
			compare_witness: None,
			closures: closures::ClosureTypes::default(),
			compiling: None,
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
				if let Some(arity) = crate::closures::closure_call_arity(name) {
					let callee = items.get(1).and_then(|argument| match argument.drop_meta() {
						Node::Symbol(variable) => Some(variable.as_str()),
						_ => None,
					});
					let kinds = self.user_function_kinds();
					return crate::closures::closure_call_site_kind(callee, arity, &self.ctx, &kinds);
				}
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
			Op::Eq => I::F64Eq,
			Op::Ne => I::F64Ne,
			Op::Lt => I::F64Lt,
			Op::Gt => I::F64Gt,
			Op::Le => I::F64Le,
			Op::Ge => I::F64Ge,
			_ => unreachable!("Not a comparison op: {:?}", op),
		};
		func.instruction(&cmp);
		func.instruction(&I::I64ExtendI32U);
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
		self.emit_instance_types();
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
			func.instruction(&I::GlobalGet(*idx));
		} else {
			func.instruction(&I::I64Const(tag as i64));
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
		crate::analyzer::extract_signal_polls(&mut self.ctx);
		self.type_errors.append(&mut self.ctx.parameter_conflicts);
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
		let message = self.located(message);
		self.type_errors.push(message);
		func.instruction(&I::Unreachable);
	}

	/// The latest source position among the nodes being emitted: where an emitter error points
	fn note_position(&mut self, node: &Node) {
		if let Some(info) = node.get_lineinfo() {
			self.source_position = Some((info.line_nr, info.column));
		}
	}

	/// An emitter error at the source position of the statement being emitted, unless it names one already
	fn located(&self, message: String) -> String {
		let Some((line, column)) = self.source_position else { return message };
		let at = format!(" at {line}:{column}");
		if message.contains(UNKNOWN_POSITION) {
			return message.replacen(UNKNOWN_POSITION, &at, 1);
		}
		if crate::diagnostic::names_position(&message) {
			return message;
		}
		match message.split_once(FIX_SEPARATOR) {
			Some((what, fix)) => format!("{what}{at}{FIX_SEPARATOR}{fix}"),
			None => format!("{message}{at}"),
		}
	}

	/// Does the node call a function of the program (`str(f(1))` is the text of f's result, not of the call)
	fn mentions_call(&self, node: &Node) -> bool {
		let mut found = false;
		node.visit(&mut |part| {
			if let Node::List(items, _, _) = part {
				found |= matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.ctx.user_functions.contains_key(name));
			}
		});
		found
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

	/// `x is 5` of a name defined nowhere: `is` compares (P61, user: "educate the user to use the be key word for
	/// definitions"), so the error teaches the definition `x be 5`
	fn emit_undefined_comparison(&mut self, func: &mut Function, node: &Node) -> bool {
		let Node::Key(subject, Op::Eq, value) = node.drop_meta() else { return false };
		let Node::Symbol(name) = subject.drop_meta() else { return false };
		if !self.is_unbound(name) {
			return false;
		}
		let value = crate::type_tests::compared_text(subject).unwrap_or_else(|| value.drop_meta().serialize().trim().to_string());
		self.emit_type_error(func, format!("undefined variable: {name}; `is` compares, a definition is written `{name} be {value}`"));
		true
	}

	/// An operand of arithmetic as an i64: a character is no number there (P65): a character written or held is the type
	/// error, an element read whose kind shows only at run time (`x#2` of `[1,'a']`) fails with not_a_number
	pub(super) fn emit_arithmetic_operand(&mut self, func: &mut Function, operand: &Node) {
		match operand.drop_meta() {
			_ if matches!(self.get_type(operand), Kind::Codepoint | Kind::Text) => {
				let kind = self.get_type(operand);
				self.emit_type_error(func, format!("type error: {kind} in arithmetic: {}", list_ops::CHARACTER_IS_NO_NUMBER));
			}
			// an array of Ints or Floats holds no character: its fast read stays
			Node::Key(list, Op::Hash, index) if !matches!(list.drop_meta(), Node::Empty) && !self.holds_only_numbers(list) && !self.map_is_indexed_by_key(index) => {
				self.emit_list_element_node(func, list, index);
				let held = self.node_scratch();
				Self::emit_list(func, &[I::LocalTee(held), I::RefAsNonNull, I::StructGet { struct_type_index: self.type_manager.node_type, field_index: 0 }]);
				Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Codepoint as i64), I::I64Eq]);
				self.emit_fail_if(func, list_ops::NOT_A_NUMBER);
				Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
				self.emit_call(func, "get_int_value");
			}
			_ => self.emit_numeric_value(func, operand),
		}
	}

	fn emit_undefined_variable(&mut self, func: &mut Function, name: &str) {
		// a unit reaches the emitter only where quantities are not computed yet (notes/units_runtime.md)
		let unit_hint = if crate::units::is_unit(name) { UNIT_AT_RUN_TIME } else { "" };
		self.emit_type_error(func, format!("undefined variable: {name}{unit_hint}"));
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
		// task_poll is called by the emitted loops of a program that controls tasks, not by the program itself
		let polls = effects.calls_external(crate::host::TASK_CONTROL);
		// signal_poll likewise, by a program with `on interrupt {…}` or `on every … {…}`
		let polls_signals = crate::analyzer::handles_system_signals(&self.ctx);
		self.ctx.ffi_imports.retain(|name, _| effects.calls_external(name) || (polls && name == crate::host::TASK_POLL) || (polls_signals && name == crate::host::SIGNAL_POLL));
		for need in &self.discovered_needs {
			if let Need::MathImport(key) = need {
				let function = key.trim_start_matches("m.");
				self.ctx.ffi_imports.extend(crate::ffi::get_ffi_signature(function).map(|signature| (key.to_string(), signature)));
			}
		}
		self.config.emit_ffi_imports = !self.ctx.ffi_imports.is_empty();
		self.config.emit_wasi_imports |= effects.needs(Capability::Wasi);
		// another runtime's module is reached through the host word foreign_call
		self.config.emit_host_imports |= effects.needs(Capability::Host) || effects.needs(Capability::Foreign);
	}

	/// Import modules this module declares, known after `emit_for_node`
	pub fn imports(&self, capability: crate::effects::Capability) -> bool {
		use crate::effects::Capability::*;
		match capability {
			Host | Foreign => self.config.emit_host_imports,
			Wasi => self.config.emit_wasi_imports,
			Ffi | Libm => self.config.emit_ffi_imports,
			Sql | Process => false, // no host implements execute or exec yet: such a module fails to link, loudly
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
			func.instruction(&I::LocalGet(i as u32));
		}
		func.instruction(&I::StructNew(type_idx));
		func.instruction(&I::End);

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
		self.emit_runtime_errors(); // before the int runtime: exact_div fails divide_by_zero
		self.emit_int_runtime();
		self.export_exact_builders();
		// a C function returning char * hands its text in through the text heap (ffi.rs text_node)
		if self.ctx.ffi_imports.values().any(|sig| sig.library != crate::host::HOST_LIBRARY && matches!(sig.results.first(), Some(ValType::Ref(_)))) {
			self.emit_text_heap_global();
		}
		self.emit_getters(); // before the list ops (list_at calls get_int_value), after the runtime errors it calls
		// Emit list and string operation functions
		self.emit_list_ops();
		self.emit_linear_arrays();
		self.emit_cells();
		self.emit_text_of();
		self.emit_equality_ops();
		self.emit_text_as_int(); // after the getters: it calls get_int_value
		self.emit_text_as_float();
		if self.config.emit_reflection {
			self.emit_reflection();
		}
		self.emit_math_helpers();
		self.emit_text_builtins();
		self.emit_node_arithmetic(); // after text_as_float, get_int_value and text_concat, which it calls
		self.emit_map_get(); // after the text builtins: map keys are compared by text_of
		self.emit_node_map_runtime();
		self.emit_library_ops(); // after the text builtins: the library words call text_of
	}

	fn emit_getters(&mut self) {
		let node_ref = self.node_ref(true);

		// get_kind(node: ref $Node) -> i64
		self.exported_function("get_kind", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			func.instruction(&I::LocalGet(0));
			func.instruction(&I::StructGet {
				struct_type_index: s.type_manager.node_type,
				field_index: 0,
			});
		});

		// get_int_value(node: ref $Node) -> i64
		// Extract integer from Node's data field (i64box); a character is its code point ('5' as int is 53), so a
		// character variable compares with c >= '0'. Any other node is the runtime error not_an_int, which a `try`
		// catches, never a raw cast trap
		self.exported_function("get_int_value", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			let node_kind_is = |kind: Kind| [
				I::LocalGet(0),
				I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 },
				I::I64Const(KIND_MASK), I::I64And, I::I64Const(kind as i64), I::I64Eq,
			];
			Self::emit_list(func, &node_kind_is(Kind::Codepoint));
			Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(0)]);
			s.emit_codepoint_of_node(func);
			Self::emit_list(func, &[I::Return, I::End]);
			Self::emit_list(func, &node_kind_is(Kind::Int));
			Self::emit_list(func, &[I::I32Eqz, I::If(BlockType::Empty)]);
			s.emit_runtime_error(func, "not_an_int");
			func.instruction(&I::End);
			func.instruction(&I::LocalGet(0)); // Node
			func.instruction(&I::StructGet {
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
				f.instruction(&I::RefTestNonNull(string));
				f.instruction(&I::If(BlockType::Result(ValType::I32)));
				s.emit_text_field(f, 0, field_index);
				f.instruction(&I::Else);
				f.instruction(&I::I32Const(0));
				f.instruction(&I::End);
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
				func.instruction(&I::I64Const(1));
				func.instruction(&I::LocalSet(2));

				// block $done
				func.instruction(&I::Block(BlockType::Empty));
				// loop $loop
				func.instruction(&I::Loop(BlockType::Empty));

				// br_if $done (i64.eqz (local.get $exp))
				func.instruction(&I::LocalGet(1)); // exp
				func.instruction(&I::I64Eqz);
				func.instruction(&I::BrIf(1)); // break to $done

				// result = result * base
				func.instruction(&I::LocalGet(2)); // result
				func.instruction(&I::LocalGet(0)); // base
				func.instruction(&I::I64Mul);
				func.instruction(&I::LocalSet(2));

				// exp = exp - 1
				func.instruction(&I::LocalGet(1)); // exp
				func.instruction(&I::I64Const(1));
				func.instruction(&I::I64Sub);
				func.instruction(&I::LocalSet(1));

				// br $loop
				func.instruction(&I::Br(0));

				// end loop
				func.instruction(&I::End);
				// end block
				func.instruction(&I::End);

				// return result
				func.instruction(&I::LocalGet(2));
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
		func.instruction(&I::I64ReinterpretF64);
		func.instruction(&I::LocalSet(value_bits));
		func.instruction(&I::LocalGet(value_bits));
		func.instruction(&I::F64ReinterpretI64);
		func.instruction(&I::F64Abs);
		func.instruction(&I::F64Const(I64_RANGE_LIMIT.into()));
		func.instruction(&I::F64Lt);
		func.instruction(&I::I32Eqz);
		self.emit_fail_if(func, "float_out_of_int_range");
		func.instruction(&I::LocalGet(value_bits));
		func.instruction(&I::F64ReinterpretI64);
		func.instruction(&I::I64TruncF64S);
		self.emit_int_from_machine(func);
	}

	/// A float has no implicit exact value: reading it as an Int is refused instead of truncated
	fn emit_float_in_exact_context(&mut self, func: &mut Function, what: &str) {
		let message = format!("{what} is a float where an exact Int is expected: use it in float arithmetic or truncate with `as int`");
		self.emit_type_error(func, message);
	}

	/// Node locals not first assigned by a statement of the body itself (`if c { n = "a" } else { n = "b" }; n`) start
	/// as ø: wasm reads a non-nullable local only after a set on every path, and a set inside a block ends with it
	fn emit_node_local_defaults(&mut self, func: &mut Function, body: &Node, skipped: usize) {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly | Bracket::Round, Separator::Semicolon | Separator::Newline) => items.as_slice(),
			_ => std::slice::from_ref(body),
		};
		let node_storage = Ref(self.node_ref(false));
		let mut defaulted: Vec<u32> = self.scope.locals.values()
			.filter(|local| local.position as usize >= skipped && self.local_storage_type(&local.name, local.kind) == node_storage)
			.filter(|local| !first_mention_assigns(statements, &local.name))
			.map(|local| local.position).collect();
		defaulted.sort();
		for slot in defaulted {
			self.emit_call(func, "new_empty");
			func.instruction(&I::LocalSet(slot));
		}
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
		self.typed_maps = self.find_typed_maps(node);
		self.typed_structs = self.find_typed_structs(node);
		self.bounded_counters = big_int::bounded_counters(node);

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
		self.emit_node_local_defaults(&mut func, node, 0);
		self.emit_witness_installation(&mut func);
		// a program with `on interrupt {…}` watches for ctrl-c from its start and takes one that came at its end
		let signal_poll = self.ffi_func_index(crate::host::SIGNAL_POLL);
		signal_poll.iter().for_each(|poll| { func.instruction(&I::Call(*poll)); });
		self.emit_node_instructions(&mut func, node);
		signal_poll.iter().for_each(|poll| { func.instruction(&I::Call(*poll)); });
		func.instruction(&I::End);

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
			func.instruction(&I::Unreachable);
			return;
		}
		self.ctx.used_functions.insert(name);
		func.instruction(&I::Call(self.func_index(name)));
	}

	fn emit_node_null(&self, func: &mut Function) {
		func.instruction(&I::RefNull(HeapType::Concrete(self.type_manager.node_type)));
	}


	/// Emit instructions to construct a Node
	fn emit_node_instructions(&mut self, func: &mut Function, node: &Node) {
		self.note_position(node);
		if self.emit_undefined_comparison(func, node) {
			return;
		}
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
					func.instruction(&I::F64Const(Ieee64::new(f.to_bits())));
					self.emit_call(func, "new_float");
				}
				Number::Float(_) | Number::Quotient(..) | Number::BigQuotient(_) => {
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
				// a parameter named like a function of the program (`f(inc) := inc(3)`) is the parameter
				let parameter = self.scope.lookup(s).is_some_and(|local| local.is_param);
				if let Some(user_fn) = self.ctx.user_functions.get(s).filter(|_| !parameter) {
					match user_fn.params.iter().filter(|param| param.default.is_none()).count() {
						0 => self.emit_user_function_call(func, s, &[]),
						count => {
							// P82: the function itself is `function add` (or `&add`); a nested `outer·add` is written `add`
							let written = s.rsplit('·').next().unwrap_or(s);
							self.emit_type_error(func, format!("{written} needs {count} argument{}; fix: function {written}", if count == 1 { "" } else { "s" }))
						}
					}
					return;
				}
				if self.emit_typed_list_as_node(func, s) {
					return;
				}
				// Check if this is a local variable lookup
				if let Some(local) = self.scope.lookup(s) {
					func.instruction(&I::LocalGet(local.position));
					if !local.kind.is_ref() {
						self.emit_primitive_as_node(func, local.kind);
					}
					return;
				}
				// Check if this is a global variable lookup
				if let Some(&(idx, kind)) = self.ctx.user_globals.get(s) {
					func.instruction(&I::GlobalGet(idx));
					if kind.is_ref() {
						func.instruction(&I::RefAsNonNull);
					} else {
						self.emit_primitive_as_node(func, kind);
					}
					return;
				}
				// a lone word that names nothing is a symbol; one close to a defined name may be a typo (near-miss warning)
				if !self.data_context {
					if let Err(error) = self.warn_near_miss(s) {
						let message = match error { Node::Error(reason) => reason.drop_meta().name(), other => other.serialize() };
						self.emit_type_error(func, message);
						return;
					}
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
				func.instruction(&I::I64Const(0));
				self.emit_call(func, "new_int");
			}
			&Node::True => {
				func.instruction(&I::I64Const(1));
				self.emit_call(func, "new_int");
			}
		}
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
		self.module.section(self.string_table.data_section());
		self.emit_names();
		self.module.section(&self.names);

		let bytes = crate::dead_functions::without_dead_functions(&self.module.finish())
			.map_err(|message| format!("internal error: dropping dead functions failed: {message}"))?;
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
		let instance_names: Vec<(u32, String)> = self.instance_types.iter().map(|(class, instance)| (instance.type_index, format!("{class}{}", struct_backend::INSTANCE_SUFFIX))).collect();
		types.extend(instance_names.iter().map(|(idx, name)| (*idx, name.as_str())));
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
		fields.extend(self.instance_types.values().map(|instance| (instance.type_index, instance.fields.iter().enumerate().map(|(i, (field, _))| (i as u32, field.as_str())).collect())));
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
				let element_type = match crate::type_kinds::field_storage(&f.type_name) {
					crate::type_kinds::FieldStorage::I64 => Val(ValType::I64),
					crate::type_kinds::FieldStorage::I32 => Val(ValType::I32),
					crate::type_kinds::FieldStorage::F64 => Val(ValType::F64),
					crate::type_kinds::FieldStorage::F32 => Val(ValType::F32),
					crate::type_kinds::FieldStorage::Text => Val(Ref(string_ref)),
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
					func.instruction(&I::I64Const(*v));
				}
				RawFieldValue::I32(v) => {
					func.instruction(&I32Const(*v));
				}
				RawFieldValue::F64(v) => {
					func.instruction(&I::F64Const(Ieee64::new(v.to_bits())));
				}
				RawFieldValue::F32(v) => {
					func.instruction(&I::F32Const(Ieee32::new(v.to_bits())));
				}
				RawFieldValue::String(_) => {
					let (ptr, len) = string_offsets[string_idx];
					func.instruction(&I32Const(ptr as i32));
					func.instruction(&I32Const(len as i32));
					func.instruction(&I::StructNew(string_type_idx));
					string_idx += 1;
				}
			}
		}
		func.instruction(&I::StructNew(struct_type_idx));
		func.instruction(&I::End);
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
	use wasmtime::{Linker, Val};

	// Register WASM metadata for field name lookup in Debug output
	let module_id = crate::gc_traits::register_gc_types_from_wasm(wasm_bytes).ok();

	let engine = gc_engine();
	let mut store = crate::util::fueled_store(&engine, ());
	let module = crate::run::module_cache::compiled_module(&engine, wasm_bytes).map_err(|e: wasmtime::Error| e.to_string())?;

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
/// The instance a program consists of: type definitions, then `TypeName:{field:value, ...}` as its value. An instance
/// anywhere else (an argument `f(pt{…})`, a statement among others) is no such program: it runs the standard way
pub(crate) fn find_struct_instantiation(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
	let statements = match node.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.as_slice(),
		_ => std::slice::from_ref(node),
	};
	let (last, before) = statements.split_last()?;
	if !before.iter().all(|statement| matches!(statement.drop_meta(), Node::Type { .. } | Node::Empty)) {
		return None;
	}
	find_instantiation_recursive(registry, last)
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

pub(crate) fn emit_module(node: &Node) -> Result<CompiledModule, Node> {
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

/// Is the first statement that mentions `name` its plain assignment `name = value` (value not reading name)
fn first_mention_assigns(statements: &[Node], name: &str) -> bool {
	let mentions = |node: &Node| {
		let mut found = false;
		node.visit(&mut |part| found |= matches!(part, Node::Symbol(word) if word == name));
		found
	};
	match statements.iter().find(|statement| mentions(statement)).map(Node::drop_meta) {
		Some(Node::Key(target, Op::Assign | Op::Define, value)) => matches!(target.drop_meta(), Node::Symbol(word) if word == name) && !mentions(value),
		Some(_) => false,
		None => true,
	}
}

/// The bracket of a list as the high bits of its kind (`new_list(first, rest, bracket_info)`); node_in reads it back
pub fn bracket_info(bracket: &Bracket) -> i64 {
	match bracket {
		Bracket::Curly => 0,
		Bracket::Square => 1,
		Bracket::Round => 2,
		Bracket::Less => 3,
		Bracket::Other(_, _) => 4,
		Bracket::None => 5,
	}
}

/// Leave the last module in `test.wasm` for inspection; a read-only directory must not fail the program
pub(crate) fn write_debug_module(bytes: &[u8]) {
	if let Err(failure) = std::fs::write("test.wasm", bytes) {
		warn!("could not write test.wasm: {failure}");
	}
}

/// A trap is a runtime error of the program; any other failure (link, instantiation, validation) is an error
/// of the compiler or the environment. Both become error values, never the unevaluated program.
#[cfg(feature = "native")]
pub(crate) fn failed_run(failure: anyhow::Error) -> Node {
	if let Some(task) = failure.chain().find_map(|cause| cause.downcast_ref::<crate::tasks::TaskFailure>()) {
		return crate::node::error(&task.0);
	}
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
		// a meta key starts with its mark (`no_field_@source`); a later `@` begins the url of a Firefox or Safari frame
		let mark = rest.strip_prefix(crate::node::ATTRIBUTE_MARK).map_or("", |_| "@");
		let name: String = rest[mark.len()..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
		format!("no field {mark}{name}")
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
	// `return error("…")` from a number function: the message is the trap detail
	let returned_error = trace.contains(list_ops::RETURNED_ERROR).then(|| trace.split_once(TRAP_DETAIL_PREFIX).and_then(|(_, rest)| rest.lines().next()))
		.flatten().map(|detail| detail.trim_matches('"').to_string());
	let runtime_error = returned_error.or(missing_field).or(no_case).or(overflow).or_else(|| list_ops::RUNTIME_ERRORS.iter().find(|name| trace.contains(*name)).map(|name| list_ops::runtime_error_message(name)));
	let exact_trap = EXACT_TRAP_MESSAGES.iter().find(|(function, _)| trace.contains(function)).map(|(_, message)| message.to_string());
	// the engine's own integer divide trap (a division the emitter did not guard) reads as the guarded one
	let divide_trap = trap.ends_with(ENGINE_DIVIDE_BY_ZERO).then(|| list_ops::runtime_error_message(list_ops::DIVIDE_BY_ZERO));
	let message = runtime_error.or(exact_trap).or(divide_trap).unwrap_or(trap);
	crate::node::error(&message)
}
