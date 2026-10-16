//! What a compile is for (`CompileMode`): the `for_*` wrappers set it around a compile, the lowering passes and the
//! emitter read it. pipeline.rs re-exports it, the path most users name.

use crate::compiler::compile_context;
use crate::node::Node;

/// What the program compiled on this thread is compiled for: set by the `for_*` wrappers around a compile, read by the
/// lowering passes and (through wasm_emitter/config.rs) the emitter
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompileMode {
	/// for a page (`warp build --site`), where page events happen
	pub for_a_page: bool,
	/// for `warp dev`, whose page keeps the program's state across reloads
	pub for_dev: bool,
	/// a program that renders its own markup (rendering_itself)
	pub rendering_itself: bool,
	/// for WAGI (`warp build --wagi`): its routes answer the one request in its environment
	pub for_wagi: bool,
	/// the program ends in the print of its value that compile_printing_result added: that print shows nothing for ø (a
	/// loop or a call that gives nothing), as `warp <file>` shows nothing; a print of the program's own prints ø
	pub prints_result: bool,
	/// the page is compiled to render its first HTML where the server's code is (site.rs, headless.rs)
	pub prerendering: bool,
	/// a module for any host (`warp compile --wasm`): a browser's host reads its values too
	pub for_any_host: bool,
	/// hosted (`warp deploy`): a program without routes gets them (lowering/web/serve.rs with_default_routes)
	pub for_hosting: bool,
	/// it runs under `warp test`: its tests run and give its value (lowering/control/test_blocks.rs)
	pub for_tests: bool,
}

impl CompileMode {
	/// a plain run: none of the flags
	pub const PLAIN: CompileMode = CompileMode {
		for_a_page: false,
		for_dev: false,
		rendering_itself: false,
		for_wagi: false,
		prints_result: false,
		prerendering: false,
		for_any_host: false,
		for_hosting: false,
		for_tests: false,
	};

	/// Whether the program exports its renderer: a page's, or one rendering itself
	pub fn renders_itself(&self) -> bool {
		self.for_a_page || self.rendering_itself
	}
}

/// `warp --fast`: every program of this process compiles as if it began with `use fast_math` (lib/fast_math.warp, card
/// switch-rebinds)
static FAST_MATH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_fast_math(on: bool) {
	FAST_MATH.store(on, std::sync::atomic::Ordering::Relaxed)
}

pub fn fast_math() -> bool {
	FAST_MATH.load(std::sync::atomic::Ordering::Relaxed)
}

/// `run` with the program granted only what a sandboxed program gets (`warp --sandbox`, Capability::GRANTED_SANDBOXED)
pub fn sandboxed<T>(run: impl FnOnce() -> T) -> T {
	with_granted(&crate::analyzer::effects::Capability::GRANTED_SANDBOXED, || {
		compile_context::scoped(|context| &mut context.sandboxed, true, run)
	})
}

/// `run` with the program granted only `granted` (checked again after lowering, which brings calls of its own:
/// `use python math` → foreign_call)
pub(crate) fn with_granted<T>(granted: &'static [crate::analyzer::effects::Capability], run: impl FnOnce() -> T) -> T {
	compile_context::scoped(|context| &mut context.granted, granted, run)
}

/// What the program compiled now is compiled for
pub fn compile_mode() -> CompileMode {
	compile_context::read(|context| context.mode)
}

pub(crate) fn set_mode_flag(flag: fn(&mut CompileMode) -> &mut bool, value: bool) {
	compile_context::change(|context| *flag(&mut context.mode) = value)
}

/// `run` with one flag of the compile mode set, then that flag as it was (the others keep what `run` set)
pub(crate) fn with_flag<T>(flag: fn(&mut compile_context::CompileContext) -> &mut bool, run: impl FnOnce() -> T) -> T {
	compile_context::scoped(flag, true, run)
}

/// `run` compiling a program as a component exporting and importing `functions`: the emitter adds the canonical-ABI
/// adapters of the exports and imports the imports from the core modules the world names
pub fn for_a_component<T>(functions: crate::lowering::component_worlds::WorldFunctions, run: impl FnOnce() -> T) -> T {
	compile_context::scoped(|context| &mut context.component, functions, run)
}

/// The functions the component being compiled exports, none for a program
pub fn component_exports() -> Vec<(String, crate::lowering::component_worlds::Signature)> {
	compile_context::read(|context| context.component.exports.clone())
}

/// The functions the component being compiled imports, none for a program
pub fn component_imports() -> Vec<(String, crate::lowering::component_worlds::Signature)> {
	compile_context::read(|context| context.component.imports.clone())
}

/// `run` compiling a program for a page: its page event handlers are expected, not warned about; it exports the
/// reflection getters the page's host reads values with
pub fn for_a_page<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_a_page, run)
}

/// `run` compiling a program for WAGI (Spin, Akamai Functions): lowering/web/serve.rs ends it in the answer of its routes
/// to the request (lib/wagi.warp), and it exports `_start`
pub fn for_wagi<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_wagi, run)
}

/// `run` compiling a module for any host (`warp compile --wasm`): it exports the reflection getters a host without GC
/// field access (the browser) reads values with, so a browser program importing it copies values in and out
pub fn for_any_host<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_any_host, run)
}

/// `run` compiling a hosted program (`warp deploy`): without routes of its own, its value is the page at / and each of
/// its functions a route
pub fn for_hosting<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_hosting, run)
}

/// `run` compiling a page that calls its server functions directly, as the server does (lowering/web/serve.rs): for its
/// first HTML, rendered at build time
pub fn prerendering<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.prerendering, run)
}

/// `run` compiling the shipped page, whose calls of server functions start as `values`
pub fn with_server_values<T>(values: Vec<Node>, run: impl FnOnce() -> T) -> T {
	compile_context::scoped(|context| &mut context.server_values, values, run)
}

/// `run` compiling a program `warp serve` serves at the port: its server functions, its top-level `get`/`post` routes
/// and its page, without a `serve PORT {…}` statement
pub fn serving_at<T>(port: u16, run: impl FnOnce() -> T) -> T {
	compile_context::scoped(|context| &mut context.serving_port, Some(port), run)
}

/// `run` compiling a program that renders its own markup (the playground, web.rs renders_itself): it exports page·html
/// and page·render as a page does (lowering/web/page_html.rs); nothing else of a page changes (it may still serve)
pub fn rendering_itself<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.rendering_itself, run)
}

/// `run` compiling a program for `warp dev`: the main-level variables it changes are kept (lowering/data/stored_values.rs)
pub fn for_dev<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_dev, run)
}

/// `run` a program under `warp test`: its `test` lines and blocks run, its value is their summary (P209, P210)
pub fn for_tests<T>(run: impl FnOnce() -> T) -> T {
	with_flag(|context| &mut context.mode.for_tests, run)
}
