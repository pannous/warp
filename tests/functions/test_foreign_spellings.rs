//! Decision foreign-spellings (user, 2026-10-13): other languages' names for warp's words are aliases with a note
//! naming warp's word; R's and PHP's words come with `use R` / `use PHP`; OCaml let … in, `fun x ->`, Elixir's
//! `def …, do:` and C++'s `[](…){…}` lambdas are gone
use crate::is;
use warp::parse;

/// The notes a program's compilation gives, as `written → preferred`
fn notes_of(code: &str) -> Vec<String> {
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code));
	hints.iter().map(|hint| format!("{} → {}", hint.original, hint.canonical)).collect()
}

#[test]
fn foreign_method_names_are_warps_methods() {
	is!("xs = [1, 2, 3]; xs.Select(x => x * 2).Where(x => x > 2)", parse("[4 6]"));
	is!("\"42\".to_i + 1", 43);
	is!("xs = [1, 2, 3].map(&:to_s); xs#2 + \"!\"", "2!");
	is!("xs = [1, 2, 3]; xs.sumOf { it * 2 }", 12);
	assert!(notes_of("x = 5; x.to_s").contains(&".to_s → .string".to_string()));
}

#[test]
fn a_field_of_a_foreign_method_name_stays_the_field() {
	is!("type P { Count: int; b: int }; p = P(3, 1); p.Count", 3);
	is!("xs = [1, 2, 3]; xs.Count()", 3);
	is!("last_index(list) := list.length - 1; last_index([4, 5, 6])", 2);
}

#[test]
fn module_qualified_iterations_are_warps_words() {
	is!("Enum.map([1, 2, 3], &(&1 * 2))", parse("[2 4 6]"));
	is!("Enum.sum([1, 2, 3])", 6);
	is!("List.map (x => x * 2) [1, 2, 3]", parse("[2 4 6]"));
	is!("Seq.sum [1, 2, 3]", 6);
}

#[test]
fn auto_is_var() {
	is!("auto x = 3; x = x + 1; x", 4);
	assert!(notes_of("auto x = 3; x").contains(&"auto → var".to_string()));
}

#[test]
fn r_and_php_words_come_with_their_use() {
	is!("use R; sapply(c(1, 2, 3), function(x) x * 2)", parse("[2 4 6]"));
	is!("use R; lapply(c(1, 2), function(x) x + 1)", parse("[2 3]"));
	is!("use R; vapply(c(1, 2), function(x) x * 3)", parse("[3 6]"));
	is!("use PHP; array_map(fn($x) => $x * 2, [1, 2, 3])", parse("[2 4 6]"));
	is!("use PHP; array_filter([1, 2, 3], fn($x) => $x > 1)", parse("[2 3]"));
	is!("use PHP; array_sum([1, 2, 3])", 6);
	let error = warp::wasm_emitter::eval("sapply([1, 2], x => x)").serialize();
	assert!(error.contains("use R"), "{error}");
}

#[test]
fn key_event_aliases_listen_to_key() {
	is!("count = 0; on keydown { count += 1 }; emit key; count", 1);
	is!("count = 0; on key_down { count += 1 }; emit key; emit key; count", 2);
	is!("count = 0; once keypress { count += 1 }; emit key; emit key; count", 1);
	assert!(notes_of("on keydown { print key }; 1").contains(&"keydown → key".to_string()));
}

/// reviewer: every listener word and the DOM attribute form read the same table
#[test]
fn dom_event_names_listen_to_warps_events() {
	is!("count = 0; when keyup { count += 1 }; emit key; count", 1);
	is!("count = 0; on mousedown { count += 1 }; emit click; count", 1);
	is!("count = 0; onclick { count += 1 }; emit click; count", 1);
	is!("count = 0; onkeydown { count += 1 }; emit key; count", 1);
	assert!(notes_of("onclick { print 1 }; 1").contains(&"onclick → on click".to_string()));
}

/// reviewer: a release alias says the event fires on the press; a program's own `onclick` stays its function
#[test]
fn release_aliases_say_press_and_own_names_stay() {
	assert!(notes_of("on keyup { print 1 }; 1").contains(&"keyup → key".to_string()));
	is!("onclick(block) := 7; onclick { 1 }", 7);
}
