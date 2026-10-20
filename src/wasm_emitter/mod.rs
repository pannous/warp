//! WASM GC code emitter - generates WebAssembly modules with GC support

pub use control::traps::*;
mod diagnostics;
mod export_names;
mod module_finish;
#[cfg(feature = "native")]
pub use maps::raw_structs::run_raw_struct;
pub(crate) use maps::raw_structs::*;

pub(crate) mod control;
mod globals;
pub(crate) mod numbers;
use control::control_flow::LoopValues;
mod declared_values;
pub mod functions;
mod values;
pub use functions::closures::{CAPTURE_EXPORT_PREFIX, CLOSURE_APPLY, CLOSURE_CAPTURED, CLOSURE_REBUILD};
pub(crate) use host::wasi_emitter::{OUTPUT_CALLS, OUTPUT_WORDS};
pub(crate) use lists::list_sharing::reads_collection;
pub(crate) use text::text_builtins::{join_call, joined_text};
mod constructors;
pub(crate) use constructors::{
	CONSTANT_TEXT, TEXT_ADDRESS, TEXT_BYTES_EQUAL, TEXT_BYTES_JOIN, TEXT_FLAT, TEXT_FROM_MEMORY, TEXT_LITERAL,
	TEXT_PIECE,
};
pub use constructors::{NEW_BOOL, NEW_SYMBOL_OF, NEW_TEXT_OF, NEW_TYPE};
pub(crate) use numbers::casts::{is_called_conversion, ROUNDING_FUNCTIONS};
mod config;
mod function_builder;
mod import_manager;
mod layout;
pub(crate) mod lists;
pub use reflection::node_api::NODE_AT;
pub(crate) mod time;
pub use text::similarity::NUMBERS_SIMILAR;
pub(crate) mod maps;
pub(crate) use control::loop_control::{is_step, mark_step, split_step};
pub use maps::map_backend::MAP_COPY_SUFFIX;
pub(crate) mod reflection;
mod runtime_dependencies;
mod string_table;
pub(crate) use string_table::TEXT_SEGMENT;
pub(crate) mod text;
mod type_manager;
pub use control::try_guard::{CAUGHT_ERROR, RAN_WITHOUT_ERROR};
pub(crate) mod host;

pub use numbers::big_int::{is_fixnum, EXACT_ARITHMETIC, EXACT_BUILDERS, INT_RUNTIME};

/// Something emission found it must have that the analysis pass did not request
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Need {
	/// A runtime function, by registry name
	Function(&'static str),
	/// A libm import, keyed like ffi_imports (`m.pow`) so it never shadows a user function
	MathImport(&'static str),
	/// The operator code of a Key the program makes at run time, which list_text writes as written (`x+1`)
	KeyOperator(i64),
}

/// The last compiled module, under the home directory (debug_module_path)
const DEBUG_MODULE: &str = ".cache/warp/last.wasm";
/// The texts `as bool` reads as false
const FALSY_TEXTS: [&str; 8] = ["", "0", "false", "no", "ø", "nil", "null", "none"];
/// wasmtime's words for an integer division or remainder by zero
const ENGINE_DIVIDE_BY_ZERO: &str = "integer divide by zero";
/// Node locals every function has after its Int scratch: node_scratch and container_scratch (globals.rs)
const NODE_SCRATCH_LOCALS: u32 = 2;
/// The name section's name of the start function, which installs witness tables and late calls in each instance
const START_FUNCTION: &str = "install";
/// The first stand-in call index of a runtime function the analysis did not foresee, far above any real one
/// (emit_call); the module's finish puts in the real index
const UNFORESEEN_CALL_BASE: u32 = 1 << 30;
/// What `use { memory, table } from "env"` imports instead of defining: the linear memory and a function table
pub const MEMORY_ENTITY: &str = "memory";
pub const TABLE_ENTITY: &str = "table";
pub const IMPORTABLE_ENTITIES: [&str; 2] = [MEMORY_ENTITY, TABLE_ENTITY];
/// The bytes of a wasm memory page
const PAGE_BYTES: u64 = 65536;
const MEMORY: MemoryType =
	MemoryType { minimum: 1, maximum: None, memory64: false, shared: false, page_size_log2: None };

pub use config::EmitterConfig;
pub use import_manager::ImportManager;
pub use reflection::equality::{IS_TRUTHY, SAME_NODE, VALUES_EQUAL};
pub use string_table::StringTable;
pub use type_manager::TypeManager;

use crate::analyzer::{
	analyze_required_functions, captured_variables, collect_all_types, collect_variables, extract_ffi_imports,
	extract_user_functions, infer_type, kind_with_article, param_kind, type_word_kind, Scope,
};
use crate::compiler::context::{Context, UserFunctionDef};
use crate::compiler::function::Function as FuncDef;
use crate::compiler::local::Local;
pub use crate::errors::run_time::out_of_fuel;
use crate::errors::run_time::returned_error_message;
use crate::extensions::numbers::Number;
#[cfg(feature = "native")]
use crate::gc_traits::GcObject as ErgonomicGcObject;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
pub use crate::pipeline::{compile, compile_printing_result, eval, eval_parsed, eval_untrusted, lower, CompiledModule};
use crate::type_kinds::{field_def_to_val_type, Kind, RawFieldValue, TypeDef, TypeRegistry, KIND_MASK};
#[cfg(feature = "native")]
use crate::util::gc_engine;
use log::{trace, warn};
use std::collections::{HashMap, HashSet};
use wasm_encoder::*;
#[cfg(feature = "validate")]
use wasmparser::{Validator, WasmFeatures};
use Instruction as I;
use Instruction::I32Const;
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
#[derive(Default)]
pub struct WasmGcEmitter {
	module: Module,
	functions: FunctionSection,
	code: CodeSection,
	exports: export_names::Exports,
	names: NameSection,
	/// main's index, named in the name section (a trap's backtrace shows `main`, not `<wasm function N>`)
	main_index: Option<u32>,
	/// The start function installing the witness tables and late calls (emit_installation_start)
	start_index: Option<u32>,
	/// the address of the items of `print(value)` that ends main when the pipeline added it (CompileMode::prints_result)
	result_print: usize,
	/// the module declares its own memory (not imported), sized by its texts when the module is finished
	owns_memory: bool,
	globals: GlobalSection,
	tags: TagSection,
	guards_errors: bool, // the program has a `try` that catches runtime errors
	/// The program declares a list type (`names: texts`), so its lists may carry element marks (P215)
	declares_list_types: bool,
	/// Emitting data, not code: the value of an object entry or a quoted form, where unknown words stay words (P62)
	data_context: bool,
	error_catching: Option<control::try_guard::ErrorCatching>, // the tag and globals of that `try`
	memo_caches: HashMap<i64, (u32, u32)>, // per memoized function id: the globals of its values and known flags (memoization.rs)
	extra_global_names: Vec<(u32, String)>,
	number_constants: HashMap<(num_bigint::BigInt, num_bigint::BigInt), u32>, // a big or fractional literal's global, made on first use (exact.rs)

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
	int_scratch: u32,        // first of INT_SCRATCH_LOCALS i64 locals in the current function
	loop_values: LoopValues, // the Node locals holding text loop values (control_flow.rs)
	wrapping_ints: bool,     // inside `expr as i64`: machine arithmetic
	/// `%` and `//` of Ints beyond the fixnums: the ratio-aware exact_* runtime when ratios may reach them, else int_*
	ratio_division: bool,
	int_division: bool, // emitted an int_* slow path, valid only if the module makes no ratio (emit_module)
	int_heap_global: u32, // $BigInts heap that handles index
	int_count_global: u32, // used slots in that heap
	/// the cursor of cons-list indexing: the first of the globals list, cell, index (list_ops.rs emit_list_cursor_globals)
	list_cursor_global: Option<u32>,
	/// the cursors of text indexing: the first of the globals text, index, at (list_ops.rs emit_text_cursor_globals)
	text_cursor_global: Option<u32>,
	/// the indexes of maps held as cons lists: the first of their globals (map_index.rs emit_map_index_globals)
	map_index_global: Option<u32>,
	returns_number: bool, // the function being compiled returns an i64 or f64; main returns a Node, so `return x` gives x's Node
	returns_float: bool,  // the function being compiled returns an f64, so `return 7` returns 7.0
	returned_tuple: Vec<Kind>, // the value kinds of the tuple function being compiled (`return a, b`), else empty
	tuple_packers: HashMap<String, u32>, // tuple function → its `f$list` packer (tuple_emitter.rs)
	text_heap_global: Option<u32>, // bump pointer for texts built at runtime, in memory grown past the string table
	// Needs the analyzer could not foresee (they depend on inferred types); emission reruns with them
	discovered_needs: std::collections::HashSet<Need>,
	/// Runtime functions called that the analysis did not require: the stand-in index their calls name until
	/// emit_unforeseen_functions emits them after main (emit_call)
	unforeseen_calls: HashMap<&'static str, u32>,
	/// Functions emitted code was compiled without: should_emit_function said no inside a function or outside the
	/// runtime emitters' own gates. Adding one of them later would change that code, which takes a rerun
	compiled_without: std::cell::RefCell<HashSet<String>>,
	gating_runtime: bool, // the runtime emitters decide which functions to emit (emit_constructors)
	body_depth: u32,      // functions being emitted inside each other (function_body)
	makes_floats: bool,   // the program or the emitted code makes a Float (new_float), which list_join shows by float_text
	/// The needs this emitter was rerun with: one still missing after the rerun cannot be provided, another rerun would loop
	inherited_needs: std::collections::HashSet<Need>,
	/// The highest operator code the needs ask list_text to write as written (see written_operator_bound)
	highest_key_operator: i64,
	/// The program quotes a tag (`data point{x:1}`), which list_text writes as an instance: only then does it check for one
	writes_tags: bool,
	/// The program parses code at run time (`parse(t)`): only then does a structure joining a text become its literal
	parses_code: bool,
	type_errors: Vec<String>,
	/// The std modules the program calls through std_pure / std_io (STD_MODULES_SECTION), ANY_STD_MODULE for a module
	/// named at run time
	std_modules: std::collections::BTreeSet<String>,
	/// The program's named live loops (LIVE_LOOPS_SECTION)
	live_loops: std::collections::BTreeSet<String>,
	/// The latest source position among the nodes being emitted (note_position)
	source_position: Option<crate::compiler::meta::LineInfo>,
	/// The position of the statement being emitted, which `trap_position` holds while it runs (emit_trap_position)
	statement_position: Option<crate::compiler::meta::LineInfo>,
	loop_labels: Vec<control::loop_control::LoopLabels>, // enclosing loops of the code being emitted, innermost last
	typed_lists: HashMap<String, lists::list_dispatch::TypedList>, // list variables of the body being emitted held as typed arrays
	typed_globals: HashMap<String, (u32, lists::list_dispatch::TypedList)>, // globals held as typed arrays: their global and element type
	typed_capture_globals: HashMap<u32, lists::list_dispatch::TypedList>,   // capture globals holding a typed list of main
	main_typed_lists: Option<HashMap<String, lists::list_dispatch::TypedList>>, // main's typed lists, worked out before the capture globals
	bounded_counters: std::collections::HashSet<String>, // loop counters of the body being emitted proven to stay in 0..i32::MAX (big_int.rs)
	typed_map_globals: HashMap<String, u32>, // globals held as hash tables (map_backend.rs find_typed_map_globals): their global
	typed_maps: std::collections::HashSet<String>, // map variables of the body being emitted held as hash tables (map_backend.rs)
	typed_structs: HashMap<String, String>, // instance variables of the body being emitted held as GC structs, with their class (struct_backend.rs)
	instance_types: HashMap<String, maps::struct_backend::InstanceType>, // the `P·instance` struct type of each class
	struct_abi: HashMap<String, Vec<Option<String>>>, // the parameters user functions take as structs, by class (struct_backend.rs)
	struct_results: HashMap<String, String>, // the user functions giving back a struct, with its class (struct_backend.rs)
	/// `x = (t = x; …; t)`, an inlined call updating the list it is given back: x and t share one array, no copies
	moved_lists: Vec<(String, String)>,
	/// The array calling convention of the user functions that have one (list_abi.rs)
	list_abi: HashMap<String, lists::list_abi::ListAbi>,
	/// The function being compiled returns a $NodeList
	returns_list: bool,
	compare_witness: Option<reflection::witness::WitnessTable>, // runtime dispatch of Comparable to user types (witness.rs)
	text_witness: Option<reflection::witness::WitnessTable>,    // runtime dispatch of Printable to user types (witness.rs)
	shown: Option<u32>, // the function giving main's value with printable instances as their texts (witness.rs)
	late_calls: Vec<functions::late_calls::LateCall>, // runtime calls of warp-written functions (late_calls.rs)
	/// prelude words the emitted code calls that the program did not load (late_calls.rs compiler_word)
	missing_compiler_words: Vec<String>,
	closures: functions::closures::ClosureTypes,
	/// The user function whose body is being compiled; None in main
	compiling: Option<String>,
	/// The names the program's `for … in` loops and comprehensions bind: variables even beside a function of that name
	loop_variables: HashSet<String>,
}

impl WasmGcEmitter {
	/// An emitter at the start of main, which returns a Node and whose list_text writes no operator but `:`
	pub fn new() -> Self {
		Self::default()
	}

	/// The highest operator code list_text writes as written: `:` at least, any code above the bound is written `:`
	fn written_operator_bound(&self) -> i64 {
		self.highest_key_operator.max(crate::operators::op_to_code(&crate::operators::Op::Colon))
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Type management helpers (delegate to type_manager)
	// ═══════════════════════════════════════════════════════════════════════════

	/// Get a RefType for Node with specified nullability
	fn node_ref(&self, nullable: bool) -> RefType {
		self.type_manager.node_ref(nullable)
	}

	/// The struct types of the declared types, their indices noted in the context
	fn emit_struct_types(&mut self, types: &[TypeDef]) {
		for type_def in types {
			self.type_manager.emit_single_user_type(type_def);
			if let Some(idx) = self.type_manager.get_user_type_idx(&type_def.name) {
				self.ctx.user_type_indices.insert(type_def.name.clone(), idx);
			}
		}
	}

	/// Emit core GC types (String, Node, i64box, f64box)
	fn emit_gc_types(&mut self) {
		self.type_manager.emit_gc_types(self.should_emit_function(reflection::node_api::NODE_AT));
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
		let index = self.next_func_idx;
		self.ctx.func_registry.register_at(FuncDef::builtin(name), index);
		self.next_func_idx += 1;
		index
	}

	/// The index of the runtime function or import `name`, else of the program's function: a program word named like a
	/// runtime function (`node_count`) is a function of its own (card export-name-clash)
	fn func_index(&self, name: &str) -> u32 {
		if let Some(function) = self.ctx.func_registry.get(name) {
			return function.call_index as u32;
		}
		self.ctx
			.user_functions
			.get(name)
			.and_then(|user_fn| user_fn.func_index)
			.unwrap_or_else(|| panic!("Unknown function: {}", name))
	}

	// ═══════════════════════════════════════════════════════════════════════════
	// Helper methods for clean, DRY code
	// ═══════════════════════════════════════════════════════════════════════════

	/// The Text node of the literal `s`
	fn emit_text_node(&mut self, func: &mut Function, s: &str) {
		let (offset, length) = self.string_table.literal(s);
		Self::emit_list(func, &[I::I32Const(offset as i32), I::I32Const(length as i32)]);
		self.emit_call(func, CONSTANT_TEXT);
	}

	/// The Symbol node of the literal `s`
	fn emit_symbol_node(&mut self, func: &mut Function, s: &str) {
		self.emit_text_literal(func, s);
		self.emit_call(func, NEW_SYMBOL_OF);
	}

	/// The type value `name` (`int`, `Person`): its symbol marked TYPE_VALUE_MARK, which printing shows as `class int`
	pub(crate) fn emit_type_value(&mut self, func: &mut Function, name: &str) {
		func.instruction(&I::I64Const(crate::type_kinds::TYPE_VALUE_KIND));
		self.emit_text_literal(func, name);
		func.instruction(&I::RefNull(HeapType::Concrete(self.type_manager.node_type)));
		func.instruction(&I::StructNew(self.type_manager.node_type));
	}

	/// The text `class int` of the type value in Node local `local`: the prefix (the literal of TYPE_VALUE_PREFIX)
	/// joined with its name
	pub(crate) fn emit_type_value_text(&self, func: &mut Function, local: u32, prefix: (u32, u32)) {
		func.instruction(&I::I64Const(Kind::Text as i64));
		self.emit_literal_text(func, prefix);
		self.emit_text_bytes(func, local);
		self.call(func, TEXT_BYTES_JOIN);
		func.instruction(&I::RefNull(HeapType::Concrete(self.type_manager.node_type)));
		func.instruction(&I::StructNew(self.type_manager.node_type));
	}

	/// Whether the Node in local `local` is a type value (TYPE_VALUE_MARK), as an i32
	pub(crate) fn emit_is_type_value(&self, func: &mut Function, local: u32) {
		self.emit_field(func, local, 0);
		Self::emit_type_value_test(func);
	}

	/// The kind field on the stack, as an i32: whether it is a type value's (TYPE_VALUE_KIND); a zoned time's kind with a
	/// negative offset has every high bit set, so the Symbol tag is tested too
	pub(crate) fn emit_type_value_test(func: &mut Function) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::I64Const(crate::type_kinds::TYPE_VALUE_MARK | KIND_MASK), I::I64And,
			I::I64Const(crate::type_kinds::TYPE_VALUE_KIND), I::I64Eq,
		]);
	}

	/// A word standing for a type as a value: a built-in type's or a declared class's name
	pub(crate) fn names_a_type_value(&self, word: &str) -> bool {
		crate::type_tests::is_builtin_type_word(word)
			|| word == crate::type_tests::TYPE_WORD
			|| self.ctx.type_registry.types().iter().any(|t| t.name == word)
	}

	/// Infer the Kind for an expression
	/// Extends analyzer::infer_type with user_globals knowledge
	fn get_type(&self, node: &Node) -> Kind {
		let node = node.drop_meta();
		match node {
			// a local or parameter, then a global of that name: `g(x) := x as list` never takes a global x's kind
			Node::Symbol(name) => self
				.scope
				.lookup(name)
				.map(|local| local.kind)
				.or_else(|| self.ctx.user_globals.get(name).map(|&(_, kind)| kind))
				.unwrap_or(Kind::Symbol),
			// Call of a user function, also braceless: `f 3`
			Node::List(items, _, _)
				if items.len() >= 2
					&& let Some(name) = self.user_function_head(&items[0]) =>
			{
				if let Some(arity) = crate::closures::closure_call_arity(name) {
					let callee = items.get(1).and_then(Node::symbol_name);
					let kinds = self.user_function_kinds();
					return crate::closures::closure_call_site_kind(callee, arity, &self.ctx, &kinds);
				}
				self.ctx.user_functions[name].return_kind
			}
			// `(x)` of a variable is its value, a global's too; `(random)` is a call
			Node::List(items, Bracket::Round, _) if matches!(items.as_slice(), [item] if matches!(item.drop_meta(), Node::Symbol(name) if !self.is_unbound(name) && !self.ctx.user_functions.contains_key(name))) => {
				self.get_type(&items[0])
			}
			// Arithmetic: recursively check operands with our get_type
			Node::Key(left, op, right) if op.is_arithmetic() => self.arithmetic_type(left, op, right),
			// For other nodes, use analyzer's infer_type
			_ => infer_type(node, &self.scope),
		}
	}

	/// The linear memory, exported as `memory`: defined, or imported with a function table by `use { memory, table } from "env"`
	fn emit_memory(&mut self) {
		let imported = |emitter: &Self, entity: &str| {
			emitter.ctx.imported_entities.iter().find(|(_, name)| name == entity).map(|(module, _)| module.clone())
		};
		match imported(self, MEMORY_ENTITY) {
			Some(module) => self.import_manager.import_entity(&module, MEMORY_ENTITY, EntityType::Memory(MEMORY)),
			None => self.owns_memory = true,
		}
		if let Some(module) = imported(self, TABLE_ENTITY) {
			let table =
				TableType { element_type: RefType::FUNCREF, minimum: 0, maximum: None, table64: false, shared: false };
			self.import_manager.import_entity(&module, TABLE_ENTITY, EntityType::Table(table));
		}
		self.exports.export(MEMORY_ENTITY, ExportKind::Memory, 0);
	}

	fn should_emit_function(&self, name: &str) -> bool {
		let emits = self.config.emit_all_functions || self.ctx.required_functions.contains(name);
		// a runtime emitter's own gate only decides whether to emit; any other answer shapes the code being emitted
		if !emits && (self.body_depth > 0 || !self.gating_runtime) && !self.compiled_without.borrow().contains(name) {
			self.compiled_without.borrow_mut().insert(name.to_string());
		}
		emits
	}

	/// Generate all type definitions and functions
	pub fn emit(&mut self) {
		self.emit_memory();
		// Host imports must come before GC types (imports section comes before types in WASM)
		self.import_manager.emit_imports(&self.config, &mut self.type_manager, &mut self.ctx);
		self.next_func_idx = self.import_manager.import_count();
		self.emit_gc_types();
		// the declared types' structs come after the GC types, before the functions
		let declared_types = self.ctx.type_registry.types().to_vec();
		self.emit_struct_types(&declared_types);
		self.emit_instance_types();
		if self.config.emit_kind_globals {
			self.emit_kind_globals();
		}
		self.emit_constructors();
		for type_def in &declared_types {
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
			let global = self.declare_constant_global(ValType::I64, ConstExpr::i64_const(tag as i64));
			self.exports.export(name, ExportKind::Global, global);
			self.ctx.kind_global_indices.insert(tag, global);
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
		let node = if crate::numbers::real::mentions_real(node) {
			lowered = crate::numbers::real::lower(node.clone());
			&lowered
		} else {
			node
		};
		self.config.emit_all_functions = false;
		self.loop_variables = crate::lowering::for_loop::loop_variables(node).into_iter().collect();
		// First pass: register all types (forward reference support)
		collect_all_types(&mut self.ctx.type_registry, node);
		// Analyze: Extract FFI imports, user functions, and required functions
		extract_ffi_imports(&mut self.ctx, node);
		extract_user_functions(&mut self.ctx, node);
		crate::analyzer::extract_host_words(&mut self.ctx, node);
		crate::analyzer::extract_signal_polls(&mut self.ctx);
		self.type_errors.append(&mut self.ctx.parameter_conflicts);
		self.scope.function_kinds = self.user_function_kinds().into();
		crate::analyzer::note_bool_functions(self.ctx.user_functions.values());
		self.derive_imports_from_effects(node);
		self.declares_list_types = declared_values::declares_list_types(node);
		analyze_required_functions(&mut self.ctx, node);
		// quoted or parsed code keeps its positions: node_at and $MetaNode only in a program that quotes, parses or
		// positions a node (cards data-meta, parse-word, carries-line)
		if node.mentions_any(&lists::list_emitter::QUOTE_WORDS)
			|| node.mentions_any(&[crate::host::PARSE_CODE, reflection::node_api::POSITIONED])
		{
			self.ctx.required_functions.insert(reflection::node_api::NODE_AT);
		}
		// code joins a text as its literal (card print-parse)
		if node.mentions_any(&[crate::words::AS_CODE]) {
			self.ctx.required_functions.insert(crate::words::AS_CODE);
			self.ctx.required_functions.insert(reflection::library_ops::LIST_TEXT);
			self.parses_code = true;
		}
		// code parsed at run time holds any operator
		if node.mentions_any(&[crate::host::PARSE_CODE]) {
			self.ctx.required_functions.insert(reflection::library_ops::LIST_TEXT);
			self.ctx.required_functions.insert(constructors::NEW_TYPE);
			self.parses_code = true;
			self.highest_key_operator = (crate::operators::OP_CODES.len() - 1) as i64;
		}
		if crate::analyzer::makes_floats(&self.ctx, node) {
			self.makes_floats = true;
			self.ctx.required_functions.insert(numbers::float_text::FLOAT_TEXT);
		}
		self.ctx.required_functions.extend(self.discovered_needs.iter().filter_map(|need| match need {
			Need::Function(name) => Some(*name),
			Need::MathImport(_) | Need::KeyOperator(_) => None,
		}));
		let key_operators = self.discovered_needs.iter().filter_map(|need| {
			if let Need::KeyOperator(code) = need {
				Some(*code)
			} else {
				None
			}
		});
		self.highest_key_operator = key_operators.fold(self.highest_key_operator, i64::max);
		self.writes_tags = reflection::library_ops::mentions_quoted_tag(node);
		let dependencies: Vec<_> =
			runtime_dependencies::every_dependency().into_iter().chain(self.function_text_dependency()).collect();
		runtime_dependencies::require_dependencies(&mut self.ctx.required_functions, &dependencies);
		text::text_builtins::require_float_text(&mut self.ctx.required_functions);
		self.require_list_marks(); // before any function emits a list writer, which reads marks_lists
		self.guards_errors = control::try_guard::guards_errors(node);
		let len = self.ctx.required_functions.len();
		trace!("tree-shaking: {} functions required: {:?}", len, self.ctx.required_functions);
		self.emit();
		// Pre-allocate strings from user function bodies before compiling
		self.collect_user_function_strings();
		self.allocate_declared_globals(node);
		self.allocate_closure_captures(node);
		(self.struct_abi, self.struct_results) = self.find_struct_abi(node);
		// Compile user functions after builtin infrastructure is set up
		self.compile_user_functions();
		self.emit_component_adapters();
		self.emit_compare_dispatcher();
		self.emit_text_dispatcher();
		self.emit_node_main(node);
		self.emit_unforeseen_functions(node);
	}

	/// The runtime functions main and the others called without the analysis requiring them, emitted after them by the
	/// runtime emitters running again; a function some emitted code was compiled without, or a need no function
	/// meets, takes a rerun of the whole emission instead
	fn emit_unforeseen_functions(&mut self, node: &Node) {
		loop {
			let unforeseen = self.unforeseen_functions();
			let mut missing: Vec<Need> =
				self.discovered_needs.iter().filter(|need| !self.is_provided(need)).copied().collect();
			if unforeseen.is_empty() && missing.is_empty() {
				return;
			}
			let compiled_without = unforeseen.iter().any(|name| self.compiled_without.borrow().contains(*name));
			if missing.is_empty() && !compiled_without {
				trace!("emitting the unforeseen functions {unforeseen:?} after main");
				self.ctx.required_functions.extend(&unforeseen);
				let installed = self.installed_at_start();
				self.emit_constructors();
				// a witness table or late call the start function would have installed came too late for it
				let all_emitted = unforeseen.iter().all(|name| self.ctx.func_registry.contains(name));
				if all_emitted && self.installed_at_start() == installed {
					continue;
				}
			}
			missing.extend(unforeseen.into_iter().map(Need::Function));
			return self.rerun_with(missing, node);
		}
	}

	/// What the start function installs and main reads (emit_installation_start): the witness tables, the late calls
	fn installed_at_start(&self) -> (bool, bool, usize) {
		(self.compare_witness.is_some(), self.text_witness.is_some(), self.late_calls.len())
	}

	/// The functions the calls emitted so far need beyond the required ones, with what those need in turn
	fn unforeseen_functions(&self) -> Vec<&'static str> {
		let mut wanted = self.ctx.required_functions.clone();
		wanted.extend(self.unforeseen_calls.keys().filter(|name| !self.ctx.func_registry.contains(name)));
		let dependencies: Vec<_> =
			runtime_dependencies::every_dependency().into_iter().chain(self.function_text_dependency()).collect();
		runtime_dependencies::require_dependencies(&mut wanted, &dependencies);
		text::text_builtins::require_float_text(&mut wanted);
		// a float made by the program or the emitted code may be shown
		if self.makes_floats && wanted.contains("list_join") {
			wanted.insert(numbers::float_text::FLOAT_TEXT);
		}
		wanted.retain(|name| !self.ctx.required_functions.contains(name));
		wanted.into_iter().collect()
	}

	/// The whole emission again, the needs found required from the start; a need found again cannot be provided
	fn rerun_with(&mut self, needs: Vec<Need>, node: &Node) {
		if let Some(unprovidable) = needs.iter().find(|need| self.inherited_needs.contains(need)) {
			let message = format!("the compiler found it needs {unprovidable:?} but cannot provide it");
			return self.type_errors.push(message);
		}
		let mut rerun = Self::new();
		rerun.config = self.config.clone();
		rerun.ratio_division = self.ratio_division;
		rerun.discovered_needs = needs.into_iter().chain(self.discovered_needs.drain()).collect();
		rerun.inherited_needs = rerun.discovered_needs.clone();
		trace!("emitting again for {:?}", rerun.discovered_needs);
		let emit_again = || crate::normalize::without_hints(|| rerun.emit_for_node(node)); // the first pass hinted it
		crate::compiler::phase_times::timed("emit rerun", emit_again);
		*self = rerun;
	}

	/// Each stand-in call index (emit_call) with the index of the function emitted for it
	fn unforeseen_call_indices(&self) -> HashMap<u32, u32> {
		let emitted = |name: &str| self.ctx.func_registry.get(name).map(|function| function.call_index as u32);
		(self.unforeseen_calls.iter())
			.map(|(name, &stand_in)| {
				(stand_in, emitted(name).unwrap_or_else(|| panic!("{name} called but never emitted")))
			})
			.collect()
	}

	fn is_provided(&self, need: &Need) -> bool {
		match need {
			Need::Function(name) => self.ctx.required_functions.contains(name),
			Need::MathImport(key) => self.ctx.ffi_imports.contains_key(*key),
			Need::KeyOperator(code) => {
				*code <= self.written_operator_bound()
					|| !self.ctx.required_functions.contains(reflection::library_ops::LIST_TEXT)
			}
		}
	}

	/// Imports follow resolved calls: only called FFI functions, WASI/host only if called.
	/// Explicitly enabled imports stay enabled.
	fn derive_imports_from_effects(&mut self, node: &Node) {
		use crate::analyzer::effects::{Capability, EffectReport};
		let effects = EffectReport::of(node);
		// task_poll is called by the emitted loops of a program that controls tasks, not by the program itself
		let polls = effects.calls_external(crate::host::TASK_CONTROL);
		// signal_poll likewise, by a program with `on interrupt {…}` or `on every … {…}`
		let polls_signals = crate::analyzer::handles_system_signals(&self.ctx);
		// local_offset by time_field, imported only by a program that makes an instant (analyzer extract_host_words)
		let emitter_calls = |name: &str| {
			(polls && name == crate::host::TASK_POLL)
				|| (polls_signals && name == crate::host::SIGNAL_POLL)
				|| name == crate::host::LOCAL_OFFSET
		};
		self.ctx.ffi_imports.retain(|name, _| effects.calls_external(name) || emitter_calls(name));
		for need in &self.discovered_needs {
			if let Need::MathImport(key) = need {
				let function = key.trim_start_matches("m.");
				self.ctx
					.ffi_imports
					.extend(crate::ffi::get_ffi_signature(function).map(|signature| (key.to_string(), signature)));
			}
		}
		self.config.emit_ffi_imports = !self.ctx.ffi_imports.is_empty();
		self.config.emit_wasi_imports |= effects.needs(Capability::Wasi);
		// warp's own host words imported like C (sleep, the task words, foreign_call: another runtime's module) are
		// Host too, for warp --sandbox, but need no fetch, read, warn or run: an executable has none (card exe-foreign)
		let host_import_word = effects.externals.iter().any(|(name, external)| {
			external.capability == Capability::Host && !self.ctx.ffi_imports.contains_key(name)
		});
		self.config.emit_host_imports |= host_import_word;
		// an ordering of ± values that overlap warns through the host (P224b)
		self.config.emit_host_imports |= crate::numbers::uncertain::may_order_intervals(node);
	}

	/// Import modules this module declares, known after `emit_for_node`
	pub fn imports(&self, capability: crate::analyzer::effects::Capability) -> bool {
		use crate::analyzer::effects::Capability::*;
		match capability {
			Host | Foreign => {
				self.config.emit_host_imports || self.ctx.ffi_imports.contains_key(crate::host::FOREIGN_CALL)
			}
			Wasi => self.config.emit_wasi_imports,
			Ffi | Libm => self.config.emit_ffi_imports,
			Sql | Process => false, // no host implements execute or exec yet: such a module fails to link, loudly
		}
	}

	/// Allocate a binary lookup table in linear memory
	fn allocate_bytes(&mut self, name: &str, bytes: &[u8]) -> u32 {
		self.string_table.allocate_bytes(name, bytes)
	}

	/// Place the text `s` in linear memory, for a host or C function that reads it by address: (ptr, len)
	fn memory_string(&mut self, s: &str) -> (u32, u32) {
		self.string_table.in_memory(s)
	}

	/// Node locals not first assigned by a statement of the body itself (`if c { n = "a" } else { n = "b" }; n`) start
	/// as ø: wasm reads a non-nullable local only after a set on every path, and a set inside a block ends with it.
	/// So do Node locals a nested function captures: each call refreshes the captures, also one in the local's own
	/// first value (`m = mean(xs); f = x => x - m`)
	fn emit_node_local_defaults(&mut self, func: &mut Function, body: &Node, skipped: usize) {
		let statements = match body.drop_meta() {
			Node::List(items, Bracket::Curly | Bracket::Round, Separator::Semicolon | Separator::Newline) => {
				items.as_slice()
			}
			_ => std::slice::from_ref(body),
		};
		let captured: HashSet<String> = self
			.compiling
			.clone()
			.map(|compiling| self.nested_functions(&compiling))
			.unwrap_or_default()
			.iter()
			.flat_map(|nested| self.ctx.captures.get(nested).into_iter().flatten().map(|(name, _)| name.clone()))
			.collect();
		let node_storage = Ref(self.node_ref(false));
		let mut defaulted: Vec<u32> = self
			.scope
			.locals
			.values()
			.filter(|local| {
				local.position as usize >= skipped && self.local_storage_type(&local.name, local.kind) == node_storage
			})
			.filter(|local| captured.contains(&local.name) || !first_mention_assigns(statements, &local.name))
			.map(|local| local.position)
			.collect();
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
		sorted_locals
			.into_iter()
			.skip(skipped)
			.map(|local| (1, self.local_storage_type(&local.name, local.kind)))
			.collect()
	}

	/// Emit main function that constructs the node
	pub fn emit_node_main(&mut self, node: &Node) {
		// an error in main never points into the user or prelude function compiled last (lib/prelude.warp:30:272)
		self.source_position = None;
		self.wrapping_ints = crate::compiler::compile_mode::is_fast_program();
		self.result_print = if config::compile_mode().prints_result { last_statement_items(node) } else { 0 };
		self.live_loops = crate::host::live_loop_names(node);
		// Pre-pass: collect variables first so scope is populated
		let temp_locals = collect_variables(node, &mut self.scope);
		self.typed_lists = self.main_typed_lists.take().unwrap_or_else(|| self.find_typed_lists(node));
		self.typed_maps = self.find_typed_maps(node);
		self.typed_structs = self.find_typed_structs(node);
		self.bounded_counters = numbers::big_int::bounded_counters(node);

		// Allocate strings and update Local data pointers
		self.collect_and_allocate_strings(node);
		let var_count = self.scope.local_count();
		self.next_temp_local = var_count; // Temp locals start after variables

		let node_ref = self.node_ref(false);
		let func_type = self.type_manager.function_type(vec![], vec![Ref(node_ref)]);
		self.functions.function(func_type);

		// each variable its own entry, then the temp locals, all i64
		let mut locals = self.local_declarations(0);
		if temp_locals > 0 {
			locals.push((temp_locals, ValType::I64));
		}
		self.int_scratch = var_count + temp_locals;
		locals.push((numbers::big_int::INT_SCRATCH_LOCALS, ValType::I64));
		locals.push((NODE_SCRATCH_LOCALS, Ref(self.node_ref(true)))); // node_scratch, container_scratch
		self.declare_loop_values(&mut locals, node);

		let mut func = Function::new(locals);
		self.emit_node_local_defaults(&mut func, node, 0);
		// a program with `on interrupt {…}` watches for ctrl-c from its start and takes one that came at its end
		let signal_poll = self.ffi_func_index(crate::host::SIGNAL_POLL);
		signal_poll.iter().for_each(|poll| {
			func.instruction(&I::Call(*poll));
		});
		self.emit_node_instructions(&mut func, node);
		self.emit_shown_value(&mut func);
		signal_poll.iter().for_each(|poll| {
			func.instruction(&I::Call(*poll));
		});
		func.instruction(&I::End);

		self.code.function(&func);
		self.exports.export("main", ExportKind::Func, self.next_func_idx);
		self.main_index = Some(self.next_func_idx);
		self.next_func_idx += 1;
		self.emit_installation_start();
		if config::compile_mode().for_wagi {
			self.emit_wagi_start();
		}
	}

	/// The module's start function: the witness tables and the late-called functions installed in each instance,
	/// before main and in the fresh instance a task (`go { … }`, live_loop) runs in, which never runs main
	fn emit_installation_start(&mut self) {
		if self.witness_dispatchers().is_empty() && self.late_calls.is_empty() {
			return;
		}
		let start_type = self.type_manager.function_type(vec![], vec![]);
		self.functions.function(start_type);
		let mut func = Function::new(vec![]);
		self.emit_witness_installation(&mut func);
		self.emit_late_call_installation(&mut func);
		func.instruction(&I::End);
		self.code.function(&func);
		self.start_index = Some(self.next_func_idx);
		self.next_func_idx += 1;
	}

	fn collect_and_allocate_strings(&mut self, node: &Node) {
		self.string_table.collect_from_node(node, &mut self.scope);
	}

	fn emit_call(&mut self, func: &mut Function, name: &'static str) {
		self.ctx.used_functions.insert(name);
		if !self.ctx.func_registry.contains(name) && !self.ctx.user_functions.contains_key(name) {
			let stand_in = UNFORESEEN_CALL_BASE + self.unforeseen_calls.len() as u32;
			if !self.unforeseen_calls.contains_key(name) {
				assert!(!self.ctx.required_functions.contains(name), "Unknown function: {}", name);
			}
			func.instruction(&I::Call(*self.unforeseen_calls.entry(name).or_insert(stand_in)));
			return;
		}
		self.makes_floats |= name == "new_float";
		func.instruction(&I::Call(self.func_index(name)));
	}

	fn emit_node_null(&self, func: &mut Function) {
		func.instruction(&I::RefNull(HeapType::Concrete(self.type_manager.node_type)));
	}

	/// Emit instructions to construct a Node
	fn emit_node_instructions(&mut self, func: &mut Function, node: &Node) {
		self.emit_node_value(func, node);
		// a comparison, `not`, a bool variable…: its Int 1/0 as the bool it is (card bool-type)
		if !matches!(node.drop_meta(), Node::True | Node::False) && crate::analyzer::is_boolean(node, &self.scope) {
			self.emit_call(func, constructors::AS_BOOL);
		}
		if let Some(radix) = self.shown_radix(node) {
			self.emit_call(func, radix.as_function);
		}
	}

	fn emit_node_value(&mut self, func: &mut Function, node: &Node) {
		if let Some(identity) = self.object_identity(node) {
			self.emit_object_identity(func, identity);
			return self.emit_call(func, "new_int");
		}
		if let Some(equality) = self.identity_as_equality(node) {
			return self.emit_node_value(func, &equality);
		}
		self.note_position(node);
		if self.emit_undefined_comparison(func, node) {
			return;
		}
		if self.emit_loop_jump(func, node) || self.emit_tuple_statement(func, node, Self::emit_node_instructions) {
			return;
		}
		if let Some((value, declared)) = declared_values::declared_field_value(node) {
			return self.emit_declared_value(func, Some(declared), value, Kind::Empty);
		}
		if let Some((item, place)) = crate::lowering::data_items::marked_item(node) {
			return self.emit_data_item(func, item, place);
		}
		if let Some((name, fields)) = crate::type_constructor::instance_parts(node) {
			let fields = self.with_declared_field_types(name, fields);
			self.emit_default_key(func, name, &fields, &Op::None); // an instance is no data: its own op code (D4)
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
				self.emit_text_node(func, s);
			}
			Node::Char(c) => {
				func.instruction(&I32Const(*c as i32));
				self.emit_call(func, "new_codepoint");
			}
			Node::Symbol(s) => {
				let variable = self.is_variable(s);
				if let Some(user_fn) = self.ctx.user_functions.get(s).filter(|_| !variable) {
					match user_fn.params.iter().filter(|param| param.default.is_none()).count() {
						0 => self.emit_user_function_call(func, s, &[]),
						count => {
							// P82: the function itself is `function add` (or `&add`); a nested `outer·add` is written `add`
							let written = s.rsplit('·').next().unwrap_or(s);
							self.emit_type_error(
								func,
								format!(
									"{written} needs {count} argument{}; fix: function {written}",
									if count == 1 { "" } else { "s" }
								),
							)
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
						let message = match error {
							Node::Error(reason) => reason.drop_meta().name(),
							other => other.serialize(),
						};
						self.emit_type_error(func, message);
						return;
					}
				}
				if !self.data_context && self.names_a_type_value(s) {
					self.emit_type_value(func, s);
					return;
				}
				self.emit_symbol_node(func, s);
			}
			Node::Key(left, op, right) => {
				self.emit_key_node(func, left, op, right);
			}
			Node::List(items, bracket, separator) => {
				self.emit_list_node(func, items, bracket, separator);
			}
			Node::Data(dada) => {
				self.emit_symbol_node(func, &dada.type_name);
			}
			Node::Meta { .. } => {
				self.emit_call(func, "new_empty");
			}
			// an Error node of the source emits its inner node (AGENTS.md "Serialization")
			Node::Error(inner) => self.emit_node_instructions(func, inner),
			Node::Type { name, body } => {
				// Emit type as a tagged block with name and fields
				self.emit_node_instructions(func, name);
				self.emit_node_instructions(func, body);
				self.emit_call(func, constructors::NEW_TYPE);
			}
			&Node::False => {
				func.instruction(&I::I64Const(0));
				self.emit_call(func, constructors::NEW_BOOL);
			}
			&Node::True => {
				func.instruction(&I::I64Const(1));
				self.emit_call(func, constructors::NEW_BOOL);
			}
		}
	}
}

/// The address of the items of the program's last statement when it is a call or a list, else 0
fn last_statement_items(program: &Node) -> usize {
	let last = match program.drop_meta() {
		Node::List(statements, Bracket::None, Separator::Newline | Separator::Semicolon) => statements.last(),
		statement => Some(statement),
	};
	match last.map(Node::drop_meta) {
		Some(Node::List(items, _, _)) => items.as_ptr() as usize,
		_ => 0,
	}
}

/// The module of `node`; `%` and `//` use the small int_* runtime unless the module makes ratios, which takes a
/// second emission with the ratio-aware exact_* one (card int-i64)
pub(crate) fn emit_module(node: &Node) -> Result<CompiledModule, Node> {
	let kinds = crate::compiler::self_hosted::program_kinds(node)?;
	if let Some(module) = crate::compiler::self_hosted::emit(node, kinds.as_ref()) {
		return module;
	}
	let (compiled, int_division) = crate::compiler::phase_times::timed("emit", || emit_module_with(node, false))?;
	if int_division && makes_ratios(&compiled.bytes) {
		// the first emission already hinted the same program
		return Ok(crate::normalize::without_hints(|| emit_module_with(node, true))?.0);
	}
	Ok(compiled)
}

/// Every wasm ratio is made by ratio_new, so a module without it holds none
fn makes_ratios(module: &[u8]) -> bool {
	let names = crate::compiler::dead_functions::function_names(module).unwrap_or_default();
	names.values().any(|name| name == numbers::exact::RATIO_NEW)
}

fn emit_module_with(node: &Node, ratio_division: bool) -> Result<(CompiledModule, bool), Node> {
	use crate::analyzer::effects::Capability;
	let mut emitter = WasmGcEmitter::new();
	emitter.ratio_division = ratio_division;
	emitter.emit_for_node(node);
	if !emitter.missing_compiler_words.is_empty() {
		let missing = std::mem::take(&mut emitter.missing_compiler_words);
		let message = format!("internal error: {} called but not loaded", missing.join(", "));
		crate::compiler::compile_context::change(|context| context.missing_compiler_words = missing);
		return Err(crate::node::error(&message));
	}
	if let Some(type_error) = emitter.type_error() {
		return Err(type_error);
	}
	let needs_host = emitter.imports(Capability::Host);
	let needs_wasi = emitter.imports(Capability::Wasi);
	let needs_ffi = emitter.imports(Capability::Ffi);
	let int_division = emitter.int_division;
	let bytes = emitter.try_finish().map_err(|message| crate::node::error(&message))?;
	Ok((CompiledModule { bytes, needs_host, needs_wasi, needs_ffi }, int_division))
}

/// Is the first statement that mentions `name` its plain assignment `name = value` (value not reading name)
fn first_mention_assigns(statements: &[Node], name: &str) -> bool {
	let mentions = |node: &Node| node.mentions(name);
	match statements.iter().find(|statement| mentions(statement)).map(Node::drop_meta) {
		Some(Node::Key(target, Op::Assign | Op::Define, value)) => {
			matches!(target.drop_meta(), Node::Symbol(word) if word == name) && !mentions(value)
		}
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

/// The separators of a list by their code above SEPARATOR_SHIFT (None is 0: a list made at run time names none)
pub const SEPARATORS: [Separator; 6] =
	[Separator::None, Separator::Space, Separator::Comma, Separator::Semicolon, Separator::Newline, Separator::Tab];

/// The code of a list's separator, as `separated` and `n.separator` know it
pub fn separator_code(separator: &Separator) -> i64 {
	SEPARATORS.iter().position(|known| known == separator).unwrap_or(0) as i64
}

/// The kind field's info bits (above KIND_BITS) of a list: its bracket and its separator (card keep-list)
pub fn list_info(bracket: &Bracket, separator: &Separator) -> i64 {
	bracket_info(bracket)
		| separator_code(separator) << (crate::type_kinds::SEPARATOR_SHIFT - crate::type_kinds::KIND_BITS)
}

/// Each separator's code and the text `n.separator` gives for it ("" for none)
pub fn separator_texts() -> Vec<(i64, String)> {
	SEPARATORS
		.iter()
		.enumerate()
		.map(|(code, separator)| (code as i64, separator.to_char().map(String::from).unwrap_or_default()))
		.collect()
}

/// The separator of a list's kind field, separator_code read back
pub fn separator_of_kind(kind: i64) -> Separator {
	let code = (kind & crate::type_kinds::SEPARATOR_MASK) >> crate::type_kinds::SEPARATOR_SHIFT;
	SEPARATORS.get(code as usize).cloned().unwrap_or(Separator::None)
}

/// The bracket of a list's kind high bits, bracket_info read back (Other as None)
pub fn bracket_of_info(info: i64) -> Bracket {
	match info {
		0 => Bracket::Curly,
		1 => Bracket::Square,
		2 => Bracket::Round,
		3 => Bracket::Less,
		_ => Bracket::None,
	}
}

/// Where the last compiled module is kept for inspection: one fixed path under the home directory, never the current
/// directory, which would leave build debris beside the sources (card cwd-artifacts); None without a home (the browser)
pub fn debug_module_path() -> Option<std::path::PathBuf> {
	std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(DEBUG_MODULE))
}

/// Keep the last module at `debug_module_path`; a read-only cache must not fail the program
pub(crate) fn write_debug_module(bytes: &[u8]) {
	let Some(path) = debug_module_path() else { return };
	let written = path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&path, bytes));
	if let Err(failure) = written {
		warn!("could not write {}: {failure}", path.display());
	}
}

/// The info of a list of these items: one item stands between no separators (`[a]` parses Space-separated)
pub(crate) fn items_info(items: &[Node], bracket: &Bracket, separator: &Separator) -> i64 {
	let separator = if items.len() > 1 { separator } else { &Separator::None };
	list_info(bracket, separator)
}
