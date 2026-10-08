//! What a program needs: required runtime functions, FFI and host-word imports

use super::*;

/// Analyze node tree for non-default required functions.
/// Default functions (new_empty, new_int, new_float, new_text, new_symbol, new_codepoint, new_key, new_list)
/// are always included and don't need to be inserted here.
pub fn analyze_required_functions(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::Number(number) => {
			let exact_decimal = matches!(number, Number::Float(value) if Number::is_exact_decimal(*value));
			if exact_decimal || !matches!(number, Number::Int(n) if crate::wasm_emitter::is_fixnum(*n)) {
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
		}
		Node::Text(text) => {
			if let Some(number) = crate::warp_parser::number_in_text(text) {
				analyze_required_functions(ctx, &Node::Number(number));
			}
		}
		// run_block hands exact numbers back by composing them from fixnums (tasks.rs Builders)
		Node::Symbol(name) if crate::host::VALUE_GIVING_WORDS.contains(&name.as_str()) => ctx.required_functions.extend([crate::wasm_emitter::INT_RUNTIME, "new_int"]),
		Node::Empty | Node::Symbol(_) | Node::Char(_) | Node::True | Node::False => {}
		Node::Key(key, op, value) => {
			if op.is_arithmetic()
				|| op.is_shift()
				|| op.is_compound_assign()
				|| matches!(op, Op::Inc | Op::Dec | Op::Neg | Op::Abs | Op::Square | Op::Cube | Op::Xor)
			{
				ctx.required_functions.insert(crate::wasm_emitter::INT_RUNTIME);
			}
			if matches!(op, Op::Eq | Op::Ne) {
				ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
			}
			if matches!(op, Op::Identical | Op::NotIdentical) {
				ctx.required_functions.extend([crate::wasm_emitter::VALUES_EQUAL, crate::wasm_emitter::SAME_NODE]);
			}
			if matches!(op, Op::If | Op::While | Op::Question | Op::Not | Op::And | Op::Or) {
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
			if *op == Op::As && matches!(value.name().to_lowercase().as_str(), "string" | "str" | "text") {
				ctx.required_functions.insert("list_join"); // `x as string` of a variable joins its text
			}
			if *op == Op::As && value.name().to_lowercase() == "list" {
				ctx.required_functions.extend(["text_chars", "list_reverse", "text_reverse"]); // `x as list` of a text
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
					ctx.required_functions.insert("map_get");
					if let Some(name) = crate::warp_parser::subscript_key(value).and_then(constant_field_name) {
						if name == crate::wasm_emitter::list_ops::MESSAGE_FIELD {
							ctx.required_functions.insert(crate::wasm_emitter::list_ops::ERROR_MESSAGE);
						}
						ctx.missing_field_names.insert(name);
					}
					ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
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
					analyze_required_functions(ctx, key); // the counted value: `s.fs.count` reads the field fs
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
				if fn_name == ZERO_FILL_CALL {
					ctx.required_functions.insert(ZERO_FILL_CALL);
				}
				// a host word that gives a value builds exact numbers from fixnums (tasks.rs Builders)
				if crate::host::VALUE_GIVING_WORDS.contains(&fn_name.as_str()) {
					ctx.required_functions.extend([crate::wasm_emitter::INT_RUNTIME, "new_int"]);
				}
				if let Some(word) = crate::wasm_emitter::cells::CELL_WORDS.iter().find(|word| **word == fn_name) {
					ctx.required_functions.insert(word);
				}
				if fn_name == REMOVED_VALUE_CALL {
					ctx.required_functions.extend([crate::library_words::MAP_GET_OR, crate::library_words::MAP_WITHOUT]);
				}
				if fn_name == INSERT_AT_CALL || fn_name == INSERT_EITHER_CALL {
					ctx.required_functions.insert(INSERT_AT_CALL);
				}
				if fn_name == crate::type_tests::IS_TYPE {
					ctx.required_functions.insert(crate::type_tests::NODE_KIND_IN);
				}
				if fn_name == crate::type_tests::TYPE_WORD {
					ctx.required_functions.insert(crate::type_tests::NODE_TYPE_NAME);
					// node_type_name names an instance by comparing its data with each declared type's name
					if !ctx.type_registry.types().is_empty() {
						ctx.required_functions.insert(crate::wasm_emitter::VALUES_EQUAL);
					}
				}
				if fn_name == crate::switch::NO_CASE_CALL {
					ctx.missing_case_labels.extend(items.get(1).map(|label| label.name()));
				}
				if fn_name == crate::wasm_emitter::text_builtins::TEXT_FORM {
					ctx.required_functions.insert("list_join");
				}
				if fn_name == crate::library_words::FIELD_WITH {
					ctx.required_functions.extend([crate::library_words::FIELD_WITH, crate::wasm_emitter::VALUES_EQUAL]);
				}
				if fn_name == crate::library_words::VALUES_SIMILAR {
					ctx.required_functions.insert(crate::wasm_emitter::NUMBERS_SIMILAR); // values_similar when needed
				}
				if fn_name == crate::library_words::INSTANCE_COPY {
					ctx.required_functions.insert(crate::library_words::INSTANCE_COPY);
				}
				if ctx.ffi_imports.contains_key(fn_name.as_str()) {
					for item in items.iter().skip(1) {
						analyze_required_functions(ctx, item);
					}
					return;
				}
				if items.len() == 2 {
					if let Some(counter) = counting_function(fn_name, ctx) {
						require_counter(ctx, counter);
						analyze_required_functions(ctx, &items[1]);
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

/// The callee of `name(args)`: a call is a symbol applied with round brackets and no space before the arguments.
/// `(name args)`, `(name, args)` and `[name args]` are data.
pub fn call_name<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator) -> Option<&'a str> {
	match (items, bracket, separator) {
		// `f()` is a call too (user, P92): the empty parentheses are glued to the name, unlike the group `(f)`
		([head, ..], Bracket::Round, Separator::None) => match head.drop_meta() {
			Node::Symbol(name) => Some(name),
			_ => None,
		},
		_ => None,
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

/// The kind of an FFI call's result: a C string is a text (ffi.rs text_node), a host word's Node is known at run time
/// only (task_await_value), a void or integer result an Int
pub(super) fn ffi_call_kind(name: &str) -> Kind {
	crate::ffi::get_ffi_signature(name).map_or(Kind::Int, |signature| signature_kind(&signature))
}

/// The kind of a foreign function's result: a float, a text (a C string), a host word's Node, else an int
pub(crate) fn signature_kind(signature: &crate::ffi::FfiSignature) -> Kind {
	match signature.results.first() {
		Some(wasm_encoder::ValType::F64 | wasm_encoder::ValType::F32) => Kind::Float,
		Some(wasm_encoder::ValType::Ref(_)) if signature.library != crate::host::HOST_LIBRARY => Kind::Text,
		Some(wasm_encoder::ValType::Ref(_)) => Kind::Empty,
		_ => Kind::Int,
	}
}

/// Extract FFI imports from "import X from Y" and "use Y" statements, and the libm functions called without one
pub fn extract_ffi_imports(ctx: &mut Context, node: &Node) {
	extract_declared_ffi_imports(ctx, node);
	ctx.ffi_imports.extend(crate::wasm_emitter::component_adapters::imported_signatures());
	add_implicit_libm_imports(ctx, node);
	// `use c` makes every libc function the program calls importable (getenv, toupper), not only the built-in few
	if uses_library(node, "c") {
		add_called_library_imports(ctx, node, "c", &|name| crate::ffi::get_ffi_signature_from_lib(name, "c").is_some());
	}
}

/// Does the program say `use <library>` (`use c`, `use libc`)
pub(super) fn uses_library(node: &Node, library: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| {
		if let Node::List(items, _, _) = part {
			if let [word, named] = items.as_slice() {
				found |= is_word(word, "use") && crate::ffi::resolve_library_alias(&library_name(named)) == library;
			}
		}
	});
	found
}

/// libm functions the emitter does not implement itself (rounding and √ are builtins): a call of one the program neither
/// imports nor defines links it from libm, as `import f from 'm'` would; without that it compiled to its argument
pub(super) fn add_implicit_libm_imports(ctx: &mut Context, node: &Node) {
	let is_builtin = |name: &str| crate::wasm_emitter::ROUNDING_FUNCTIONS.contains(&name) || name == "sqrt";
	let mut implicit: Vec<&str> = crate::ffi::LIBM_F64_FUNCTIONS.iter().map(|(name, _)| *name).filter(|name| !is_builtin(name)).collect();
	implicit.push(LIBM_LN); // ffi.rs signs it as libm's log
	add_called_library_imports(ctx, node, "m", &|name| implicit.contains(&name) || (!is_builtin(name) && is_f64_header_function(name)));
}

/// A function math.h declares with f64 parameters and an f64 result (exp2, cbrt, erf): it links from libm like the
/// listed ones (card call-name: it compiled to its last argument)
fn is_f64_header_function(name: &str) -> bool {
	use wasm_encoder::ValType::F64;
	crate::ffi::get_signatures_from_headers("m").get(name).is_some_and(|signature| {
		!signature.params.is_empty() && signature.params.iter().all(|param| *param == F64) && signature.results == [F64]
	})
}

/// Import from `library` every function the program calls that `is_candidate` accepts, unless the program imports or
/// defines it itself
pub(super) fn add_called_library_imports(ctx: &mut Context, node: &Node, library: &str, is_candidate: &dyn Fn(&str) -> bool) {
	let mut called = HashSet::new();
	node.visit(&mut |part| {
		if let Node::List(items, bracket, separator) = part {
			if let Some(name) = call_name(items, bracket, separator).filter(|name| is_candidate(name)) {
				called.insert(name.to_string());
			}
		}
	});
	called.retain(|name| !ctx.ffi_imports.contains_key(name));
	if called.is_empty() {
		return;
	}
	let mut defined = Context::new();
	extract_user_functions(&mut defined, node);
	for name in called.iter().filter(|name| !defined.user_functions.contains_key(*name)) {
		add_ffi_import(ctx, name, library);
	}
}

/// The natural logarithm under its usual name, libm's log
pub(super) const LIBM_LN: &str = "ln";

pub(super) fn extract_declared_ffi_imports(ctx: &mut Context, node: &Node) {
	let node = node.drop_meta();
	match node {
		Node::List(items, _, _) => {
			if !items.is_empty() {
				match items[0].drop_meta() {
					Node::Symbol(first_sym) => {
						if first_sym == "import" && items.len() >= 2 {
							if items.len() == 2 {
								let lib = library_name(&items[1]);
								add_ffi_lib(ctx, &lib);
								return;
							}
							let func_names = imported_names(&items[1]);
							if items.len() >= 3 {
								if let Node::Key(ref key, _, ref value) = items[2].drop_meta() {
									if key.name() == "from" {
										let lib = library_name(value);
										func_names.iter().for_each(|func_name| add_ffi_import(ctx, func_name, &lib));
										return;
									}
								}
							}
							if items.len() >= 4 && items[2].name() == "from" {
								let lib = library_name(&items[3]);
								func_names.iter().for_each(|func_name| add_ffi_import(ctx, func_name, &lib));
								return;
							}
						} else if first_sym == "use" && items.len() >= 4 && items[2].name() == "from" {
							let lib = library_name(&items[3]);
							imported_names(&items[1]).iter().for_each(|name| add_named_import(ctx, name, &lib));
							return;
						} else if first_sym == "use" && items.len() >= 2 {
							let lib = library_name(&items[1]);
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
				extract_declared_ffi_imports(ctx, item);
			}
		}
		Node::Key(ref key, _, ref value) => {
			if key.name() == "import" {
				if let Node::Key(ref from_key, _, ref lib) = value.drop_meta() {
					if from_key.name() == "from" {
						let func_name = key.name();
						let lib_name = library_name(lib);
						add_ffi_import(ctx, &func_name, &lib_name);
						return;
					}
				}
			}
			extract_declared_ffi_imports(ctx, key);
			extract_declared_ffi_imports(ctx, value);
		}
		Node::Meta { ref node, .. } => {
			extract_declared_ffi_imports(ctx, node);
		}
		_ => {}
	}
}

/// Calls of the host words (`sleep(ms)`, `random()` …) import them, unless the program defines a function of that name
pub fn extract_host_words(ctx: &mut Context, node: &Node) {
	match node.drop_meta() {
		Node::List(items, _, _) => {
			if let Some(Node::Symbol(name)) = items.first().map(Node::drop_meta) {
				if crate::wasm_emitter::linear_arrays::is_linear_word(name) && !ctx.user_functions.contains_key(name) {
					add_ffi_import(ctx, name, crate::wasm_emitter::linear_arrays::LINEAR_LIBRARY);
				}
				// a float map kernel (shared_arrays.rs): the address of its source block in, of the new one out
				if name.starts_with(crate::wasm_emitter::linear_arrays::LINEAR_MAP_PREFIX) {
					use crate::wasm_emitter::linear_arrays::{LINEAR_LIBRARY, LINEAR_NEW};
					let kernel: &'static str = Box::leak(name.clone().into_boxed_str());
					let signature = crate::ffi::FfiSignature::new(kernel, LINEAR_LIBRARY, vec![wasm_encoder::ValType::I64], vec![wasm_encoder::ValType::I64]);
					ctx.ffi_imports.insert(name.clone(), signature);
					add_ffi_import(ctx, LINEAR_NEW, LINEAR_LIBRARY);
				}
				if crate::host::HOST_WORDS.contains(&name.as_str()) && !ctx.user_functions.contains_key(name) {
					add_ffi_import(ctx, name, crate::host::HOST_LIBRARY);
					// the host builds a caught stack overflow's Error with the module's own error_of
					if name == crate::host::GUARDED_CALL {
						ctx.required_functions.insert(crate::wasm_emitter::text_builtins::ERROR_OF);
					}
					// a program that controls tasks polls at its loops, where a paused task waits (browser)
					if name == crate::host::TASK_CONTROL {
						add_ffi_import(ctx, crate::host::TASK_POLL, crate::host::HOST_LIBRARY);
					}
				}
			}
			items.iter().for_each(|item| extract_host_words(ctx, item));
		}
		Node::Key(left, _, right) => {
			extract_host_words(ctx, left);
			extract_host_words(ctx, right);
		}
		_ => {}
	}
}

/// A program with `on interrupt {…}` or `on every … {…}` polls for them at its loop starts (host signal_poll,
/// notes/system_signals.md)
pub fn extract_signal_polls(ctx: &mut Context) {
	if handles_system_signals(ctx) {
		add_ffi_import(ctx, crate::host::SIGNAL_POLL, crate::host::HOST_LIBRARY);
	}
}

pub fn handles_system_signals(ctx: &Context) -> bool {
	ctx.user_functions.keys().any(|name| name == crate::host::INTERRUPT_HANDLER || name == crate::host::SHARED_HANDLER || name.starts_with(crate::host::TIMER_HANDLER_PREFIX) || name.starts_with(crate::host::FILE_HANDLER_PREFIX) || name.starts_with(crate::host::FETCH_HANDLER_PREFIX))
}

/// The library an import names: `"z"` and `'m'` are one-character texts, which parse as characters
pub(super) fn library_name(library: &Node) -> String {
	match library.drop_meta() {
		Node::Char(letter) => letter.to_string(),
		other => other.name(),
	}
}

/// The function names of `import sin from 'm'` and of the group `import (sin, floor, fabs) from 'm'`
pub(super) fn imported_names(names: &Node) -> Vec<String> {
	match names.drop_meta() {
		Node::List(items, _, _) => items.iter().map(Node::name).collect(),
		single => vec![single.name()],
	}
}

/// `use { memory, table, puts } from "env"`: the module's memory and table are imported (crate::wasm_emitter::IMPORTABLE_ENTITIES),
/// any other name is a function of the library
fn add_named_import(ctx: &mut Context, name: &str, library: &str) {
	match crate::wasm_emitter::IMPORTABLE_ENTITIES.contains(&name) {
		true => ctx.imported_entities.push((library.to_string(), name.to_string())),
		false => add_ffi_import(ctx, name, library),
	}
}

/// Add an FFI import by function name
pub(super) fn add_ffi_import(ctx: &mut Context, name: &str, library: &str) {
	use crate::ffi::{get_ffi_signature, get_ffi_signature_from_lib};

	let sig = get_ffi_signature_from_lib(name, library)
		.or_else(|| get_ffi_signature(name));

	match sig {
		Some(sig) => {
			ctx.ffi_imports.insert(name.to_string(), sig);
		}
		None => {
			ctx.unresolved_imports.insert(name.to_string(), library.to_string());
		}
	}
}

/// Add all common functions from a library
pub(super) fn add_ffi_lib(ctx: &mut Context, lib: &str) {
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
pub(super) fn add_ffi_lib_dynamic(ctx: &mut Context, lib: &str) {
	use crate::ffi::get_signatures_from_headers;

	if crate::wasm_modules::is_module_path(lib) {
		for (name, export) in crate::wasm_modules::exports(lib) {
			ctx.ffi_imports.insert(name.clone(), export.signature.clone());
			ctx.ffi_imports.insert(crate::wasm_modules::qualified(lib, name), export.signature.clone());
		}
		return;
	}

	let signatures = get_signatures_from_headers(lib);
	if signatures.is_empty() {
		// the program is analysed several times (effects, emission), the library is reported once
		static WARNED_LIBRARIES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
		let mut warned = WARNED_LIBRARIES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		if !warned.iter().any(|known| known == lib) {
			eprintln!("[FFI] Warning: No functions found for library '{}'", lib);
			warned.push(lib.to_string());
		}
		return;
	}

	for (name, sig) in signatures {
		ctx.ffi_imports.insert(name.clone(), sig.clone());
	}
}
