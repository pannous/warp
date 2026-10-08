use crate::is;
use std::fs;
use std::path::{Path, PathBuf};
use warp::modules::resolve_in;
use warp::wasm_emitter::eval;
use warp::wasp_parser::WaspParser;
use warp::*;

fn module_directory(test: &str, modules: &[(&str, &str)]) -> PathBuf {
	let directory = crate::common::scratch_directory(&format!("warp_modules_{test}"));
	fs::create_dir_all(&directory).expect("create module directory");
	for (file, source) in modules {
		fs::write(directory.join(file), source).expect("write module");
	}
	directory
}

fn resolved_text(source: &str, directory: &Path) -> String {
	let resolved = resolve_in(WaspParser::parse(source), &[directory.to_str().expect("utf8 path")]);
	format!("{resolved:?}")
}

#[test]
fn test_use_loads_module_from_samples() {
	is!("use square;square 7", 49);
}

#[test]
fn test_use_imports_globals_and_functions() {
	is!("use square;answer + square 2", 46);
}

#[test]
fn test_use_twice_is_a_no_op() {
	is!("use square;use square;square 3", 9);
}

#[test]
fn test_use_missing_module_is_an_error_value() {
	assert_eq!(eval("use no_such_module;1"), error("module not found: no_such_module"));
}

#[test]
fn test_use_math_stays_builtin() {
	is!("use math;floor(4.5)", 4);
}

#[test]
fn test_module_statements_are_not_imported() {
	let directory = module_directory("statements", &[("noisy.wasp", "double(x):=x*2\n99")]);
	let resolved = resolved_text("use noisy;1", &directory);
	assert!(resolved.contains("double"), "definition imported: {resolved}");
	assert!(!resolved.contains("99"), "the module's own expression must not run: {resolved}");
}

#[test]
fn test_warp_extension_is_found() {
	let directory = module_directory("warp_extension", &[("shifted.warp", "shift(x):=x+100")]);
	assert!(resolved_text("use shifted;1", &directory).contains("shift"));
}

#[test]
fn test_search_order_prefers_earlier_directory() {
	let first = module_directory("order_first", &[("same.wasp", "from_first:=1")]);
	let second = module_directory("order_second", &[("same.wasp", "from_second:=2")]);
	let resolved = resolve_in(WaspParser::parse("use same;1"), &[first.to_str().unwrap(), second.to_str().unwrap()]);
	let text = format!("{resolved:?}");
	assert!(text.contains("from_first") && !text.contains("from_second"), "{text}");
}

#[test]
fn test_module_using_module_imports_transitively_and_survives_cycles() {
	let directory = module_directory("cycle", &[("ping.wasp", "use pong\nping:=1"), ("pong.wasp", "use ping\npong:=2")]);
	let resolved = resolved_text("use ping;1", &directory);
	assert!(resolved.contains("ping") && resolved.contains("pong"), "{resolved}");
}

/// card module-locals: a used module's functions never see the program's variables, so their own locals ask nothing
/// (`items` of markup's html_element in a page)
#[test]
fn module_function_locals_are_not_the_programs_variables() {
	warp::diagnostic::take_warnings();
	let compiled = warp::pipeline::for_a_page(|| warp::pipeline::compile("use markup\nlet items = 3\nhtml{ p{ \"x\" } }"));
	let warnings: Vec<String> = warp::diagnostic::take_warnings().iter().map(|warning| warning.to_string()).collect();
	assert!(compiled.is_ok());
	assert!(warnings.iter().all(|warning| !warning.contains("new local")), "{warnings:?}");
}

/// A file module's own code calls its own words, whatever the program names its variables (card module-scope)
#[test]
fn a_program_variable_leaves_a_file_module_its_words() {
	let directory = module_directory("module_scope", &[("mytext.wasp", "words(t) := split(t, \" \")\ntitled(t) := join([upper(w) for w in words(t)], \" \")")]);
	let program = directory.join("app.wasp");
	let run = |code: &str| warp::modules::with_program_file(&program, || eval(code));
	assert_eq!(run("use mytext; words = [\"ab\", \"cd\"]; [titled w for w in words]"), texts(vec!["AB", "CD"]));
	assert_eq!(run("use mytext; words = [\"a b\"]; titled \"hello world\""), Node::Text("HELLO WORLD".into()));
}
