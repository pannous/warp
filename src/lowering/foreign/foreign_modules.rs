//! `use python "math"` (also `use python math`, `use python "os.path" as path`, `use js Math`): the module of another runtime, reached
//! through the host word foreign_call (src/modules/foreign.rs, notes/stdlib_connectors.md). `math.sqrt(2)` is the call
//! `foreign_call("python", "math", "sqrt", 1, [2])`, `math.pi` the read `foreign_call("python", "math", "pi", 0, ø)`
//! (an empty argument list is ø on the way, so whether it is a call is said apart).
//! Values cross as JSON; the result is any Node.

use super::nodes::{call, children_rewritten, key, round_call};
use crate::node::{symbol, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::FOR_WORD;
use std::collections::HashMap;
use std::path::Path;

const USE_WORD: &str = "use";
/// The runtimes a `use <runtime> <module>` names: Python, and JavaScript (node natively, the page in the browser)
pub const FOREIGN_RUNTIMES: [&str; 3] = ["python", JS_RUNTIME, COMPONENT_RUNTIME];
/// JavaScript: its browser globals are typed through WebIDL (src/formats/web_idl.rs)
const JS_RUNTIME: &str = "js";
/// The member of a foreign call that constructs its module (`new URL(…)`, src/modules/foreign.rs and host-foreign.js)
const CONSTRUCTOR_MEMBER: &str = "";
/// The member a call sets to its one argument: `e.innerHTML = "x"` is the member "set innerHTML" (src/modules/foreign.rs)
const SETTER_PREFIX: &str = "set ";
/// `use wasm "lib.wasm" as lib`: a WebAssembly component (src/modules/components.rs); its module is the file, found next to the
/// program, and named after the file by default
pub const COMPONENT_RUNTIME: &str = "wasm";
const COMPONENT_EXTENSION: &str = ".wasm";
/// `use cargo regex`, `use crate "regex"`: a crate from crates.io, built as a component (src/modules/cargo_crates/)
pub const CRATE_RUNTIMES: [&str; 2] = ["cargo", "crate"];
/// The version after `\0asm` in a core module's header; a component's names another layer
const CORE_MODULE_VERSION: [u8; 4] = [1, 0, 0, 0];
/// The page's components by name, one per line: build.sh components writes it beside components/<name>.js
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
const PAGE_COMPONENTS: &str = "page:components/names.txt";

pub fn lower(program: Node) -> Node {
	let program = component_uses(program);
	let mut modules = HashMap::new();
	program.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			modules.extend(foreign_use(items));
		}
	});
	if modules.is_empty() {
		return program;
	}
	Foreign {
		modules: &modules,
		values: HashMap::new(),
		component_values: Default::default(),
		component_handles: HashMap::new(),
		web_idl_values: HashMap::new(),
	}
	.rewrite(program)
}

/// The components the program uses, by alias: `use wasm "calc.wasm" as calc`, `use calc.wasm` (their paths beside the
/// program)
pub fn component_modules(program: &Node) -> HashMap<String, String> {
	let mut modules = HashMap::new();
	component_uses(program.clone()).visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			modules.extend(
				foreign_use(items)
					.filter(|(_, (runtime, _))| runtime == COMPONENT_RUNTIME)
					.map(|(alias, (_, path))| (alias, path)),
			);
		}
	});
	modules
}

/// `use rust_demo.wasm`: the component's long form `use wasm "rust_demo.wasm"` (card g-_Xm4), `use rust_demo.wasm as
/// demo` that of `use wasm "rust_demo.wasm" as demo`; a core module file stays the import of its functions (modules.rs)
fn component_uses(node: Node) -> Node {
	let module = match &node {
		Node::List(items, _, _) => component_module(items),
		_ => None,
	};
	match (module, node) {
		(Some(Err(problem)), _) => crate::node::error(&problem),
		(Some(Ok(module)), Node::List(_, bracket, separator)) => {
			Node::List(vec![symbol(USE_WORD), symbol(COMPONENT_RUNTIME), module], bracket, separator)
		}
		(_, node) => node.map_children(component_uses),
	}
}

/// The component of `use rust_demo.wasm`, as the text of its path, or its path `as` an alias; a `.wasm` file that is
/// not there, or an aliased name nothing resolves, the error naming the components that are
fn component_module(items: &[Node]) -> Option<Result<Node, String>> {
	if let [word, runtime, used] = items {
		return crate_module(word, runtime, used);
	}
	let [word, used] = items else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if crate::modules::USE_KEYWORDS.contains(&word.as_str())) {
		return None;
	}
	let (file, alias) = match used.drop_meta() {
		Node::Key(file, Op::As, alias) => (file.as_ref(), Some(alias)),
		_ => (used, None),
	};
	let written = crate::modules::path_of(file)?;
	let path = match (component_path(&written), alias) {
		(Some(path), _) => path,
		// `use nothere as x`: no C library without functions
		(None, Some(_)) if !crate::modules::resolves(&written) => Err(format!(
			"no module, component or library {written}; {}",
			listing(&known_components(&crate::modules::beside_program(&format!("{written}{COMPONENT_EXTENSION}"))))
		)),
		(None, _) => return None,
	};
	Some(path.map(|path| match alias {
		Some(alias) => Node::Key(Box::new(Node::Text(path)), Op::As, alias.clone()),
		None => Node::Text(path),
	}))
}

/// `use cargo regex`, `use crate "regex" as re`: the component src/modules/cargo_crates/ builds of the crate, `as` the crate's
/// name by default
fn crate_module(word: &Node, runtime: &Node, used: &Node) -> Option<Result<Node, String>> {
	if !word.is_symbol(USE_WORD)
		|| !matches!(runtime.drop_meta(), Node::Symbol(runtime) if CRATE_RUNTIMES.contains(&runtime.as_str()))
	{
		return None;
	}
	let (named, alias) = match used.drop_meta() {
		Node::Key(named, Op::As, alias) => (named.as_ref(), Some(alias.as_ref().clone())),
		_ => (used, None),
	};
	let name = match named.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => name.clone(),
		_ => return None,
	};
	let alias = alias.unwrap_or_else(|| symbol(&name.replace('-', "_")));
	Some(crate_component(&name).map(|path| Node::Key(Box::new(Node::Text(path)), Op::As, Box::new(alias))))
}

#[cfg(feature = "native")]
fn crate_component(name: &str) -> Result<String, String> {
	crate::modules::cargo_crates::component(name)
}

#[cfg(not(feature = "native"))]
fn crate_component(name: &str) -> Result<String, String> {
	Err(format!("use cargo {name} builds the crate with cargo: only the native warp can (card use-cargo-playground)"))
}

/// `rust_demo.wasm` as written, `rust_demo` where there is a component of that name: natively the file the loader finds,
/// in the page one build.sh made
fn component_path(written: &str) -> Option<Result<String, String>> {
	let path = match written.ends_with(COMPONENT_EXTENSION) {
		true => written.to_string(),
		false => bare_component(written)?,
	};
	let found = crate::modules::beside_program(&path);
	if !found.ends_with(COMPONENT_EXTENSION) || is_core_module(&found) {
		return None;
	}
	let known = known_components(&found);
	match has_component(&found, &known) {
		true => Some(Ok(path)),
		false => Some(Err(format!("no component {written}; {}", listing(&known)))),
	}
}

fn listing(components: &[String]) -> String {
	match components.is_empty() {
		true => "there are no components here".to_string(),
		false => format!("the components here: {}", components.join(", ")),
	}
}

fn component_name(path: &str) -> String {
	Path::new(path).file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// A file that is no component, else none or unreadable here
fn is_core_module(path: &str) -> bool {
	crate::web::read_bytes(path).is_some_and(|bytes| bytes.get(4..8) == Some(CORE_MODULE_VERSION.as_slice()))
}

/// The absolute path of the component file `use rust_demo` finds (the loader's find is no path beside the program to be
/// joined again)
#[cfg(not(all(target_arch = "wasm32", not(feature = "native"))))]
fn bare_component(name: &str) -> Option<String> {
	let found = crate::modules::wasm_module_file(name)?;
	Some(found.canonicalize().unwrap_or(found).to_string_lossy().into_owned())
}

/// The components in the folder of `path`
#[cfg(not(all(target_arch = "wasm32", not(feature = "native"))))]
fn known_components(path: &str) -> Vec<String> {
	let folder = Path::new(path).parent().filter(|folder| !folder.as_os_str().is_empty()).unwrap_or(Path::new("."));
	let files = std::fs::read_dir(folder)
		.into_iter()
		.flatten()
		.flatten()
		.map(|entry| entry.path().to_string_lossy().into_owned());
	let mut names: Vec<String> = files
		.filter(|file| file.ends_with(COMPONENT_EXTENSION) && !is_core_module(file))
		.map(|file| component_name(&file))
		.collect();
	names.sort();
	names
}

#[cfg(not(all(target_arch = "wasm32", not(feature = "native"))))]
fn has_component(path: &str, _known: &[String]) -> bool {
	crate::web::file_exists(path)
}

/// `use rust_demo` in the page: a component build.sh made, unless a warp or standard module has that name
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
fn bare_component(name: &str) -> Option<String> {
	let path = format!("{name}{COMPONENT_EXTENSION}");
	let made = known_components(&path).contains(&name.to_string());
	(made && !crate::modules::names_a_module(name)).then_some(path)
}

/// The page's components, by name (components.js loads one by the name of its file)
#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
fn known_components(_path: &str) -> Vec<String> {
	crate::web::read_text(PAGE_COMPONENTS).map_or_else(Vec::new, |names| {
		names.lines().map(str::trim).filter(|name| !name.is_empty()).map(String::from).collect()
	})
}

#[cfg(all(target_arch = "wasm32", not(feature = "native")))]
fn has_component(path: &str, known: &[String]) -> bool {
	known.contains(&component_name(path))
}

/// `use python "math"` → (alias math, (python, math)); `use python "os.path" as path` → (path, (python, os.path))
fn foreign_use(items: &[Node]) -> Option<(String, (String, String))> {
	let [word, runtime, module] = items else { return None };
	if !word.is_symbol(USE_WORD) {
		return None;
	}
	let Node::Symbol(runtime) = runtime.drop_meta() else {
		return None;
	};
	if !FOREIGN_RUNTIMES.contains(&runtime.as_str()) {
		return None;
	}
	let name = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		_ => None,
	};
	let (module, alias) = match module.drop_meta() {
		Node::Key(module, Op::As, alias) => (name(module)?, name(alias)?),
		other if runtime == COMPONENT_RUNTIME => {
			let file = name(other)?;
			let stem = std::path::Path::new(&file)
				.file_stem()
				.map(|stem| stem.to_string_lossy().into_owned())
				.unwrap_or_else(|| file.clone());
			(file, stem)
		}
		other => {
			let module = name(other)?;
			(module.clone(), module.rsplit('.').next().unwrap_or(&module).to_string())
		}
	};
	let module = if runtime == COMPONENT_RUNTIME { crate::modules::beside_program(&module) } else { module };
	Some((alias, (runtime.clone(), module)))
}

/// The modules `use` names, and the variables holding a value of another runtime (`d = datetime.date(2020, 1, 2)`):
/// a member of either is reached through foreign_call
struct Foreign<'a> {
	modules: &'a HashMap<String, (String, String)>,
	values: HashMap<String, String>,
	/// variables assigned a component's result: plain values, or handles of its resources whose methods it runs
	component_values: std::collections::HashSet<String>,
	/// variables holding a handle of a component's resource: the component's path and the resource (`counter`)
	component_handles: HashMap<String, (String, String)>,
	/// variables assigned a JavaScript value WebIDL declares (`c = navigator.clipboard`): their interface
	web_idl_values: HashMap<String, String>,
}

/// `foreign_call(runtime, …)`: its runtime, when its value can stay in that runtime (a component's results are plain
/// values: a record is an object, `s.words` its field, `n * 2` warp's arithmetic)
fn foreign_runtime(node: &Node) -> Option<String> {
	called_runtime(node).filter(|runtime| runtime != COMPONENT_RUNTIME)
}

/// `foreign_call(runtime, …)`, also in a group of one: its runtime
fn called_runtime(node: &Node) -> Option<String> {
	match foreign_call_items(node)?.get(1)?.drop_meta() {
		Node::Text(runtime) => Some(runtime.clone()),
		_ => None,
	}
}

/// The items of `foreign_call(runtime, module, member, call, arguments)`, also in a group of one: `(a * 2)`
fn foreign_call_items(node: &Node) -> Option<&[Node]> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[grouped] => foreign_call_items(grouped),
		[call, ..] if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::host::FOREIGN_CALL) => {
			Some(items)
		}
		_ => None,
	}
}

/// `semver.Version` read from a component (`foreign_call("wasm", path, "Version", 0, ø)`, capitalized like a type): its
/// path and the type's name
fn type_read(node: &Node) -> Option<(Node, String)> {
	let [_, runtime, module, member, is_call, _] = foreign_call_items(node)? else {
		return None;
	};
	let is_read = *is_call.drop_meta() == Node::int(0);
	match (runtime.drop_meta(), member.drop_meta()) {
		(Node::Text(runtime), Node::Text(member))
			if runtime == COMPONENT_RUNTIME && is_read && member.starts_with(|c: char| c.is_ascii_uppercase()) =>
		{
			Some((module.clone(), member.clone()))
		}
		_ => None,
	}
}

/// The method an operator of a component's resource calls: `a + b` is `a.add(b)`, `a / b` `a.div(b)`
fn component_operator(op: &Op) -> Option<&'static str> {
	match op {
		Op::Div => Some("div"),
		other => operator_member(other),
	}
}

/// The items of a method call `name(arguments…)` after a dot
fn called_method(member: &Node) -> Option<&Vec<Node>> {
	match member.drop_meta() {
		Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
			Some(items)
		}
		_ => None,
	}
}

/// A word warp's own values answer as a method: `xs.reverse()`, `t.upper()`; on a component's plain result it stays
/// warp's, any other method is the component's (a resource's: `counter.increment(2)`)
fn is_warp_method(name: &str) -> bool {
	crate::library_words::is_library_word(name)
		|| crate::list_mutations::is_list_mutating_method(name)
		|| (1..=3).any(|arity| crate::wasm_emitter::text::text_builtins::text_builtin_kind(name, arity).is_some())
}

/// The resource a component's function or method gives (src/modules/components.rs); the browser build reads no component
#[cfg(feature = "native")]
fn result_kind(path: &str, member: &str, kind: Option<&str>) -> Option<String> {
	crate::modules::components::result_kind(path, member, kind)
}

#[cfg(not(feature = "native"))]
fn result_kind(_path: &str, _member: &str, _kind: Option<&str>) -> Option<String> {
	None
}

/// Does the resource `kind` of a component have the method `member` taking no arguments?
#[cfg(feature = "native")]
fn has_method(path: &str, kind: &str, member: &str) -> bool {
	crate::modules::components::takes_no_arguments(path, member, kind)
}

#[cfg(not(feature = "native"))]
fn has_method(_path: &str, _kind: &str, _member: &str) -> bool {
	false
}

/// The module of operators every runtime offers (Python's operator plus len and list, the loops' own in JS)
const OPERATOR_MODULE: &str = "operator";

/// The operator function an infix operator forwards to: `a * 2` of a handle is `operator.mul(a, 2)`
fn operator_member(op: &Op) -> Option<&'static str> {
	Some(match op {
		Op::Add => "add",
		Op::Sub => "sub",
		Op::Mul => "mul",
		Op::Div => "truediv",
		Op::Mod => "mod",
		Op::Pow => "pow",
		Op::Lt => "lt",
		Op::Gt => "gt",
		Op::Le => "le",
		Op::Ge => "ge",
		Op::Eq => "eq",
		Op::Ne => "ne",
		_ => return None,
	})
}

fn foreign_call(runtime: &str, module: Node, member: &str, is_call: bool, arguments: Node) -> Node {
	call(crate::host::FOREIGN_CALL, vec![text(runtime), module, text(member), Node::int(i64::from(is_call)), arguments])
}

/// `foreign_call("js", receiver, member, call, arguments)`, a JavaScript attribute read or method call: its receiver,
/// member, whether it is a call and its arguments
fn javascript_member(node: &Node) -> Option<(&Node, &str, bool, &Node)> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else {
		return None;
	};
	match items.as_slice() {
		[call, runtime, receiver, member, is_call, arguments]
			if matches!(call.drop_meta(), Node::Symbol(name) if name == crate::host::FOREIGN_CALL)
				&& matches!(runtime.drop_meta(), Node::Text(runtime) if runtime == JS_RUNTIME) =>
		{
			match member.drop_meta() {
				Node::Text(member) => Some((receiver, member, *is_call == Node::int(1), arguments)),
				_ => None,
			}
		}
		_ => None,
	}
}

/// `operator.member(arguments…)` in `runtime`
fn operator_call(runtime: &str, member: &str, arguments: Vec<Node>) -> Node {
	foreign_call(runtime, text(OPERATOR_MODULE), member, true, Node::List(arguments, Bracket::Square, Separator::Space))
}

impl Foreign<'_> {
	/// The runtime of a value of another runtime: a variable holding one, or a foreign call
	fn foreign_value(&self, node: &Node) -> Option<String> {
		match node.drop_meta() {
			Node::Symbol(name) => self.values.get(name).cloned(),
			other => foreign_runtime(other),
		}
	}

	/// Operators, indexing, counting and iteration of a foreign value forward to its runtime
	fn forwarded(&mut self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			// `-a`, `#a`
			Node::Key(empty, op @ (Op::Neg | Op::Sub | Op::Hash), operand) if empty.is_empty_node() => {
				let operand = self.rewrite(operand.as_ref().clone());
				let runtime = self.foreign_value(&operand)?;
				Some(operator_call(&runtime, if *op == Op::Hash { "len" } else { "neg" }, vec![operand]))
			}
			// `a#2`, 1-based: the item at 0-based 1
			Node::Key(indexed, Op::Hash, index) => {
				let indexed = self.rewrite(indexed.as_ref().clone());
				let runtime = self.foreign_value(&indexed)?;
				let zero_based = key(self.rewrite(index.as_ref().clone()), Op::Sub, Node::int(1));
				Some(operator_call(&runtime, "getitem", vec![indexed, zero_based]))
			}
			Node::Key(left, op, right)
				if operator_member(op).is_some() && !(*op == Op::Eq && crate::type_tests::begins_type_test(right)) =>
			{
				let (left, right) = (self.rewrite(left.as_ref().clone()), self.rewrite(right.as_ref().clone()));
				if let (Some(_), Some(member)) = (self.component_handle(&left), component_operator(op)) {
					let arguments = Node::List(vec![right], Bracket::Square, Separator::Space);
					return Some(foreign_call(COMPONENT_RUNTIME, left, member, true, arguments));
				}
				let runtime = self.foreign_value(&left).or_else(|| self.foreign_value(&right))?;
				Some(operator_call(&runtime, operator_member(op).expect("guarded"), vec![left, right]))
			}
			// `count a`, `len(a)`
			Node::List(items, _, _) if matches!(items.as_slice(), [word, _] if matches!(word.drop_meta(), Node::Symbol(word) if crate::analyzer::is_counting_word(word))) =>
			{
				let counted = self.rewrite(items[1].clone());
				let runtime = self.foreign_value(&counted)?;
				Some(operator_call(&runtime, "len", vec![counted]))
			}
			// `for x in a { … }`: over the items of a, as a list
			Node::List(items, bracket, separator)
				if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if word == FOR_WORD)
					&& items.len() >= 4 =>
			{
				let iterated = self.rewrite(items[3].clone());
				let runtime = self.foreign_value(&iterated)?;
				let mut items: Vec<Node> = items.iter().cloned().map(|item| self.rewrite(item)).collect();
				items[3] = operator_call(&runtime, "list", vec![iterated]);
				Some(Node::List(items, bracket.clone(), separator.clone()))
			}
			_ => None,
		}
	}

	/// The component and resource of a handle: a variable holding one, or a call giving one (`r.counter(5)`, `a.add(b)`)
	fn component_handle(&self, node: &Node) -> Option<(String, String)> {
		if let Node::Symbol(name) = node.drop_meta() {
			return self.component_handles.get(name).cloned();
		}
		let [_, runtime, module, member, ..] = foreign_call_items(node)? else {
			return None;
		};
		let (Node::Text(runtime), Node::Text(member)) = (runtime.drop_meta(), member.drop_meta()) else {
			return None;
		};
		if runtime != COMPONENT_RUNTIME {
			return None;
		}
		let (path, kind) = match module.drop_meta() {
			Node::Text(path) => (path.clone(), None),
			receiver => {
				let (path, kind) = self.component_handle(receiver)?;
				(path, Some(kind))
			}
		};
		let result = result_kind(&path, member, kind.as_deref())?;
		Some((path, result))
	}

	/// A variable assigned a component's result, or a component's call itself
	fn holds_component_value(&self, receiver: &Node) -> bool {
		match receiver.drop_meta() {
			Node::Symbol(name) => self.component_values.contains(name),
			other => called_runtime(other).is_some_and(|runtime| runtime == COMPONENT_RUNTIME),
		}
	}

	/// The WebIDL interface of a JavaScript receiver and its path as written: a global `use js` names, a variable assigned
	/// such a value, or what a member of one gives (`navigator.clipboard`: Clipboard, `document.body`: HTMLElement,
	/// `document.getElementById(id)`: Element); Err for a global only a page has
	fn web_idl_receiver(&self, receiver: &Node) -> Result<Option<(String, String)>, String> {
		let global = |name: &str, path: &str| {
			Ok(crate::formats::web_idl::global_interface(name)?.map(|interface| (interface, path.to_string())))
		};
		match receiver.drop_meta() {
			Node::Symbol(name) => match self.modules.get(name) {
				Some((runtime, module)) if runtime == JS_RUNTIME => global(module, name),
				_ => Ok(self.web_idl_values.get(name).map(|interface| (interface.clone(), name.clone()))),
			},
			// the global inside a lowered read
			Node::Text(name) => global(name, name),
			other => {
				let Some((inner, member, is_call, arguments)) = javascript_member(other) else {
					return Ok(None);
				};
				if let (CONSTRUCTOR_MEMBER, Node::Text(interface)) = (member, inner.drop_meta()) {
					return Ok(crate::formats::web_idl::constructible(interface)
						.then(|| (interface.clone(), format!("{interface}(…)"))));
				}
				let Some((interface, path)) = self.web_idl_receiver(inner)? else {
					return Ok(None);
				};
				let read = if is_call { format!("{path}.{member}(…)") } else { format!("{path}.{member}") };
				let context = match arguments.drop_meta() {
					Node::List(items, _, _) => match items.first().map(Node::drop_meta) {
						Some(Node::Text(context_id)) => {
							crate::formats::web_idl::context_interface(&interface, member, context_id)
						}
						_ => None,
					},
					_ => None,
				};
				Ok(context
					.or_else(|| crate::formats::web_idl::result_interface(&interface, member, is_call))
					.map(|interface| (interface, read)))
			}
		}
	}

	/// The warp type WebIDL declares for what the member gives (`crypto.randomUUID()`: text); Err when WebIDL declares
	/// something else for the receiver (`crypto.randomUuid()`)
	fn web_idl_result(
		&self,
		receiver: &Node,
		member: &str,
		is_call: bool,
		arguments: &Node,
	) -> Result<Option<String>, String> {
		let count = match arguments {
			Node::List(items, _, _) => items.len(),
			_ => 0,
		};
		let Some((interface, path)) = self.web_idl_receiver(receiver)? else {
			return Ok(None);
		};
		crate::formats::web_idl::check_member(&interface, &path, member, is_call.then_some(count))?;
		Ok(crate::formats::web_idl::optional_result_type(&interface, member, is_call))
	}

	/// `URL("https://…")` of `use js URL`: the constructor called, JavaScript's `new`, checked by WebIDL when it declares
	/// the interface
	fn constructed(&mut self, node: &Node) -> Option<Node> {
		let (name, arguments) = round_call(node)?;
		let (runtime, module) = self.modules.get(name).filter(|(runtime, _)| runtime == JS_RUNTIME)?.clone();
		if let Err(problem) = crate::formats::web_idl::check_constructor(&module, arguments.len()) {
			return Some(crate::errors::diagnostic::Diagnostic::at(node, problem).into_error());
		}
		let arguments = arguments.iter().cloned().map(|argument| self.rewrite(argument)).collect();
		Some(foreign_call(
			&runtime,
			Node::Text(module),
			CONSTRUCTOR_MEMBER,
			true,
			Node::List(arguments, Bracket::Square, Separator::Space),
		))
	}

	/// `receiver.member = value` of a foreign receiver: the member set in its runtime, checked by WebIDL when it declares
	/// the receiver (`document.getElementById("r").innerHTML = "x"`)
	fn member_set(&mut self, target: &Node, value: &Node) -> Option<Node> {
		let Node::Key(receiver, Op::Dot, member) = target.drop_meta() else {
			return None;
		};
		let Node::Symbol(member) = member.drop_meta() else {
			return None;
		};
		let receiver = self.rewrite(receiver.as_ref().clone());
		let (runtime, module) = self.receiver(&receiver)?;
		let checked = match runtime == JS_RUNTIME {
			true => self.web_idl_receiver(&receiver).and_then(|declared| match declared {
				Some((interface, path)) => crate::formats::web_idl::check_member(&interface, &path, member, None),
				None => Ok(()),
			}),
			false => Ok(()),
		};
		if let Err(problem) = checked {
			return Some(crate::errors::diagnostic::Diagnostic::at(target, problem).into_error());
		}
		Some(foreign_call(
			&runtime,
			module,
			&format!("{SETTER_PREFIX}{member}"),
			true,
			Node::List(vec![value.clone()], Bracket::Square, Separator::Space),
		))
	}

	/// Does the node name a used foreign module or a variable holding a foreign value
	fn mentions_foreign_name(&self, node: &Node) -> bool {
		let mut found = false;
		node.visit(&mut |part| {
			if let Node::Symbol(name) = part {
				found |= self.modules.contains_key(name) || self.values.contains_key(name);
			}
		});
		found
	}

	/// The runtime and the module (its name as text) or the handle (the value itself) a receiver names
	fn receiver(&self, receiver: &Node) -> Option<(String, Node)> {
		if let Node::Symbol(name) = receiver.drop_meta() {
			if let Some((runtime, module)) = self.modules.get(name) {
				return Some((runtime.clone(), Node::Text(module.clone())));
			}
			if let Some(runtime) = self.values.get(name) {
				return Some((runtime.clone(), receiver.clone()));
			}
		}
		foreign_runtime(receiver).map(|runtime| (runtime, receiver.clone()))
	}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Node::List(items, _, _) = node.drop_meta() {
			if foreign_use(items).is_some() {
				return Node::Empty;
			}
		}
		if let Some(forwarded) = self.forwarded(&node) {
			return forwarded;
		}
		if let Some(constructed) = self.constructed(&node) {
			return constructed;
		}
		// `"id \(crypto.randomUUID())"`: the holes are calls of this pass, the text is built later (interpolation.rs)
		if let Some(text) =
			crate::text::interpolation::interpolated_mentioning(&node, |hole| self.mentions_foreign_name(hole))
		{
			return self.rewrite(text);
		}
		if let Node::Key(receiver, Op::Dot, member) = node.drop_meta() {
			if let Some(read) = self.member_read(&node, receiver, member) {
				return read;
			}
		}
		match node {
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => self.assigned(target, op, *value),
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `receiver.member` of a foreign module, value or component handle: the call asking its runtime
	fn member_read(&mut self, node: &Node, receiver: &Node, member: &Node) -> Option<Node> {
		let receiver = self.rewrite(receiver.clone());
		if let Some((runtime, module)) = self.receiver(&receiver) {
			return Some(self.runtime_member(node, &receiver, &runtime, module, member));
		}
		// `counter.increment(2)` of a handle a component gave: the method of its resource
		if let Some(items) = called_method(member)
			.filter(|items| self.holds_component_value(&receiver) && !is_warp_method(&items[0].name()))
		{
			let arguments = self.arguments_rewritten(&items[1..]);
			// `semver.Version.parse(t)`: the static function `[static]version.parse` of the component
			return Some(match type_read(&receiver) {
				Some((path, resource)) => {
					foreign_call(COMPONENT_RUNTIME, path, &format!("{resource}.{}", items[0].name()), true, arguments)
				}
				None => foreign_call(COMPONENT_RUNTIME, receiver, &items[0].name(), true, arguments),
			});
		}
		// `v.major` of a handle whose resource has a method `major` without arguments (a Rust struct's field)
		if let (Node::Symbol(field), Some((path, kind))) = (member.drop_meta(), self.component_handle(&receiver)) {
			if has_method(&path, &kind, field) {
				return Some(foreign_call(COMPONENT_RUNTIME, receiver, field, true, self.arguments_rewritten(&[])));
			}
		}
		None
	}

	/// `module.member` or `module.member(arguments)` of the runtime holding the module or value; a member it can't
	/// call stays the `node` it is
	fn runtime_member(&mut self, node: &Node, receiver: &Node, runtime: &str, module: Node, member: &Node) -> Node {
		let (member, is_call, arguments) = match member.drop_meta() {
			Node::Symbol(member) => (member.clone(), false, Node::Empty),
			Node::List(items, Bracket::Round, _) => match items.split_first() {
				Some((head, arguments)) if matches!(head.drop_meta(), Node::Symbol(_)) => {
					(head.name(), true, self.arguments_rewritten(arguments))
				}
				_ => return node.clone(),
			},
			_ => return node.clone(),
		};
		let result_type = match runtime == JS_RUNTIME {
			true => self.web_idl_result(receiver, &member, is_call, &arguments),
			false => Ok(None),
		};
		let call = foreign_call(runtime, module, &member, is_call, arguments);
		// a declared text, bool, int or float is a warp value: `crypto.randomUUID().upper()` is warp's upper; `text?` keeps ø
		match result_type {
			Err(problem) => crate::errors::diagnostic::Diagnostic::at(node, problem).into_error(),
			Ok(Some(warp_type)) => key(call, Op::As, Node::Symbol(warp_type)),
			Ok(None) => call,
		}
	}

	fn arguments_rewritten(&mut self, arguments: &[Node]) -> Node {
		let arguments = arguments.iter().cloned().map(|argument| self.rewrite(argument)).collect();
		Node::List(arguments, Bracket::Square, Separator::Space)
	}

	/// `d = datetime.date(2020, 1, 2)`: d holds a value of that runtime, `d.isoformat()` asks it
	fn assigned(&mut self, target: Box<Node>, op: Op, value: Node) -> Node {
		let value = self.rewrite(value);
		if let Some(set) = (op == Op::Assign).then(|| self.member_set(&target, &value)).flatten() {
			return set;
		}
		if let Node::Symbol(name) = target.drop_meta() {
			self.remember_value(name, &value);
		}
		Node::Key(target, op, Box::new(value))
	}

	/// What the variable `name` holds now: a value of a runtime, of a Web IDL interface, of a component, a handle
	fn remember_value(&mut self, name: &str, value: &Node) {
		match foreign_runtime(value) {
			Some(runtime) => self.values.insert(name.to_string(), runtime),
			None => self.values.remove(name),
		};
		match self.web_idl_receiver(value) {
			Ok(Some((interface, _))) => self.web_idl_values.insert(name.to_string(), interface),
			_ => self.web_idl_values.remove(name),
		};
		match called_runtime(value).is_some_and(|runtime| runtime == COMPONENT_RUNTIME) {
			true => self.component_values.insert(name.to_string()),
			false => self.component_values.remove(name),
		};
		match self.component_handle(value) {
			Some(handle) => self.component_handles.insert(name.to_string(), handle),
			None => self.component_handles.remove(name),
		};
	}
}
