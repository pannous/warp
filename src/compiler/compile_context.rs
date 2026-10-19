//! What the compile on this thread is for and what it has learned so far (card cleanup-compile-context): one
//! thread-local instead of one per module. A field set for one run is `scoped`; a field a compile writes and later stages
//! (the emitter, the runner) read is written with `change`. Closures given to `read` and `change` only touch fields:
//! nothing in them compiles, or the context would be borrowed twice.
use crate::analyzer::effects::Capability;
use crate::errors::diagnostic::WarningMode;
use crate::gpu_maps::ProgramFunction;
use crate::lowering::component_worlds::WorldFunctions;
use crate::node::Node;
use crate::normalize::{HintMode, HintPosition, Style};
use crate::numbers::units::static_units::ResultUnits;
use crate::pipeline::CompileMode;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub struct CompileContext {
	/// the nine compile flags (`for_a_page`, `prerendering`, …)
	pub mode: CompileMode,
	/// what the program may call: eval's grant, the sandboxed or the untrusted one
	pub granted: &'static [Capability],
	/// a component world's exports and imports (`warp build --component`)
	pub component: WorldFunctions,
	/// the values the page's calls of server functions gave its prerender, the shipped page's first values
	pub server_values: Vec<Node>,
	/// the port `warp serve` serves a program at that has no `serve PORT {…}` of its own (lowering/web/serve.rs)
	pub serving_port: Option<u16>,
	/// a sandboxed run on this thread (sandbox.rs)
	pub sandboxed: bool,
	/// a self-hosted stage program compiles (self_hosted.rs): no file of the working directory is one of its modules
	pub beyond_working_directory: bool,
	/// the source of the program, for excerpts and its `// got it` comments
	pub source: String,
	/// a line of the source says `use strict` (in_source_mode, the one reader of it)
	pub says_strict: bool,
	pub warning_mode: WarningMode,
	/// the program said `use comments` (P114: off by default)
	pub comments_as_meta: bool,
	/// the program said `use reflection` (decision reflection): function values tell where and how they were made
	pub reflection_mode: bool,
	/// the program said `use fuel` (decision fuel-metering): its runs count fuel steps instead of a time limit
	pub counts_fuel: bool,
	pub style: Style,
	pub hint_mode: HintMode,
	/// a stage that repeats work already hinted (the emitter's rerun) says nothing twice
	pub hints_muted: bool,
	pub hint_position: HintPosition,
	/// the module file being parsed, for the positions the parser records
	pub parsed_file: Option<String>,
	/// the file being compiled; its folder is in scope (D15). None for inline code
	pub program_file: Option<PathBuf>,
	/// the module files the last program resolved used or included: `warp dev` builds anew when one changes
	pub module_files: Vec<PathBuf>,
	/// the names a used module defines: their bodies never see the program's variables
	pub module_definitions: HashSet<String>,
	/// the program began with `fast`, and the functions written `fast f(x) := …` or in a module beginning with `fast`
	/// (compile_mode::is_fast, card fast-scope)
	pub fast_program: bool,
	pub fast_functions: HashSet<String>,
	/// the modules whose classes insert_module_classes put in front of the program: the loader leaves them out
	pub early_class_modules: HashSet<PathBuf>,
	/// the used files' classes it put in front: the loader's passes of a file read their fields (a match arm
	/// `Holds(box) =>` binds box) though the declarations are left out of the file
	pub early_file_classes: Vec<Node>,
	/// the functions of those modules giving an instance of such a class (`note_value(C4)` a Note), with its class:
	/// class_methods reads `n + 2` of `n = note_value(C4)` as Note's plus before the module joins the program
	pub module_returned_classes: HashMap<String, String>,
	/// those modules' variables holding such an instance (sound's `F = PitchClass(5)`), with its class: `F#4` of the
	/// program is PitchClass's index
	pub module_instance_classes: HashMap<String, String>,
	/// the unit words the program being lowered defines itself (`m = 2`): no units while it is lowered
	pub shadowed_units: HashSet<String>,
	/// the program's own unit lines (`unit furlong := 201.168 m`, units/definitions.rs), set before it is parsed and kept
	/// until the next program, so its result reads back in them
	pub program_unit_lines: String,
	/// `Display.density` → `dpi`: each unit field's unit as written, before lower_unit_words expands its words, so
	/// messages and fixes name it (unit_fields.rs)
	pub(crate) field_unit_words: HashMap<String, String>,
	/// the units of the last lowered program's final value, to convert its run-time amount back
	pub(crate) result_units: Option<ResultUnits>,
	/// the program's declared types: `pic{width:5}` constructs and `s:shape` declares, neither names an argument
	pub declared_types: HashSet<String>,
	/// the user functions of the program being emitted whose result is a bool
	pub bool_functions: HashSet<String>,
	/// the float kernels lowering named (shared_arrays.rs): their parameter and body by name, for the emitter
	pub float_kernels: HashMap<String, (String, Node)>,
	/// the problems of the markup as written: the markup a run hands back does not repeat them
	pub written_problems: HashSet<String>,
	/// the functions of the program being lowered, by name: a GPU kernel may call them (card gpu-functions)
	pub(crate) program_functions: HashMap<String, ProgramFunction>,
	/// the number of the next temporary the compile invents across passes (`race·0`, `item·0`)
	pub fresh_number: usize,
	/// how deep in compiles we are: only the outermost one starts its fresh numbers from 0
	pub compile_depth: usize,
	/// the prelude words a compile loads as if the program called them: those code generation found it calls though
	/// the program never named them (pipeline::loading_compiler_words, card lib-from-runtime)
	pub compiler_words: Vec<String>,
	/// the prelude words the last emission called without their definition, which a second compile loads
	pub missing_compiler_words: Vec<String>,
}

impl Default for CompileContext {
	fn default() -> Self {
		CompileContext {
			mode: CompileMode::PLAIN,
			granted: &Capability::GRANTED_BY_EVAL,
			component: WorldFunctions::default(),
			server_values: vec![],
			serving_port: None,
			sandboxed: false,
			beyond_working_directory: false,
			source: String::new(),
			says_strict: false,
			warning_mode: WarningMode::default(),
			comments_as_meta: false,
			reflection_mode: false,
			counts_fuel: false,
			style: Style::default(),
			hint_mode: HintMode::Always,
			hints_muted: false,
			hint_position: HintPosition::default(),
			parsed_file: None,
			program_file: None,
			module_files: vec![],
			module_definitions: HashSet::new(),
			fast_program: false,
			fast_functions: HashSet::new(),
			early_class_modules: HashSet::new(),
			early_file_classes: vec![],
			module_returned_classes: HashMap::new(),
			module_instance_classes: HashMap::new(),
			shadowed_units: HashSet::new(),
			program_unit_lines: String::new(),
			field_unit_words: HashMap::new(),
			result_units: None,
			declared_types: HashSet::new(),
			bool_functions: HashSet::new(),
			float_kernels: HashMap::new(),
			written_problems: HashSet::new(),
			program_functions: HashMap::new(),
			fresh_number: 0,
			compile_depth: 0,
			compiler_words: vec![],
			missing_compiler_words: vec![],
		}
	}
}

thread_local! {
	static CONTEXT: RefCell<CompileContext> = RefCell::new(CompileContext::default());
}

pub fn read<R>(look: impl FnOnce(&CompileContext) -> R) -> R {
	CONTEXT.with(|context| look(&context.borrow()))
}

pub fn change<R>(write: impl FnOnce(&mut CompileContext) -> R) -> R {
	CONTEXT.with(|context| write(&mut context.borrow_mut()))
}

/// A context field set for a while: dropping it puts back the value the field held before
struct Restore<T: 'static> {
	field: fn(&mut CompileContext) -> &mut T,
	outer: Option<T>,
}

impl<T> Restore<T> {
	fn set(field: fn(&mut CompileContext) -> &mut T, value: T) -> Self {
		let outer = change(|context| std::mem::replace(field(context), value));
		Restore { field, outer: Some(outer) }
	}
}

impl<T> Drop for Restore<T> {
	fn drop(&mut self) {
		if let Some(outer) = self.outer.take() {
			// try_with: a thread's locals may already be gone when it unwinds at exit
			let _ = CONTEXT.try_with(|context| *(self.field)(&mut context.borrow_mut()) = outer);
		}
	}
}

/// A switch of the context, such as the one a pragma turns on
pub type Flag = fn(&mut CompileContext) -> &mut bool;

/// `run` with the context's `field` holding `value`, then the value it held before, also when `run` panics
pub fn scoped<T: 'static, R>(field: fn(&mut CompileContext) -> &mut T, value: T, run: impl FnOnce() -> R) -> R {
	let _restore = Restore::set(field, value);
	run()
}

/// `run` with each flag holding its value, then the values they held before, also when `run` panics
pub fn scoped_flags<R>(flags: &[(Flag, bool)], run: impl FnOnce() -> R) -> R {
	let _restores: Vec<Restore<bool>> = flags.iter().map(|&(field, value)| Restore::set(field, value)).collect();
	run()
}

/// What a used module's source passes may read of the compile so far and do not change, as text: a module lowered
/// under the same inputs lowers the same (modules/lowered_modules.rs keys its kept lowerings with it and with
/// LoweredEffects)
pub fn lowering_inputs() -> String {
	read(|context| {
		format!(
			"{:?}\n{:?}\n{} {} {} {} {} {} {}\n{:?}\n{:?}\n{:?}\n{}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{}",
			context.mode,
			context.granted,
			context.sandboxed,
			context.beyond_working_directory,
			context.comments_as_meta,
			context.reflection_mode,
			context.counts_fuel,
			context.fast_program,
			context.says_strict,
			sorted(&context.fast_functions),
			sorted(&context.module_definitions),
			sorted(&context.shadowed_units),
			context.program_unit_lines,
			sorted(&context.declared_types),
			context.compiler_words,
			context.serving_port,
			context.server_values,
			context.program_file,
			crate::pipeline::fast_math(),
		)
	})
}

/// What a used module's source passes write in the compile context: kept with a lowered module, so a kept module
/// leaves the context as its lowering did
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct LoweredEffects {
	early_class_modules: Vec<PathBuf>,
	module_returned_classes: Vec<(String, String)>,
	module_instance_classes: Vec<(String, String)>,
	field_unit_words: Vec<(String, String)>,
	bool_functions: Vec<String>,
}

impl LoweredEffects {
	pub fn now() -> Self {
		read(|context| LoweredEffects {
			early_class_modules: sorted(&context.early_class_modules).into_iter().cloned().collect(),
			module_returned_classes: sorted_entries(&context.module_returned_classes),
			module_instance_classes: sorted_entries(&context.module_instance_classes),
			field_unit_words: sorted_entries(&context.field_unit_words),
			bool_functions: sorted(&context.bool_functions).into_iter().cloned().collect(),
		})
	}

	pub fn apply(self) {
		change(|context| {
			context.early_class_modules = self.early_class_modules.into_iter().collect();
			context.module_returned_classes = self.module_returned_classes.into_iter().collect();
			context.module_instance_classes = self.module_instance_classes.into_iter().collect();
			context.field_unit_words = self.field_unit_words.into_iter().collect();
			context.bool_functions = self.bool_functions.into_iter().collect();
		});
	}
}

fn sorted<T: Ord>(items: impl IntoIterator<Item = T>) -> Vec<T> {
	let mut items: Vec<T> = items.into_iter().collect();
	items.sort();
	items
}

fn sorted_entries(map: &HashMap<String, String>) -> Vec<(String, String)> {
	sorted(map.iter().map(|(key, value)| (key.clone(), value.clone())))
}
