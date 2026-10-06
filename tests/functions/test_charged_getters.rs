//! P71 (user, 2026-10-06): `name := expr` without parameters is always charged, a getter evaluated at every use
//! (wiki/charged.md §2): it reads its free variables as they are at that use, and assigning the name afterwards is an
//! error that names the definition. Object entries `{s := e}` follow the same rule (§4).
use crate::is;
use crate::common;

#[test]
fn a_getter_reads_the_current_value_at_every_use() {
	is!("y=3; z:=y*y; y=4; z", 16);
	is!("y=3; z:=y*y; a=z; y=4; a+z", 25);
	is!("y=3\nz:=y*y\ny=4\nz", 16);
	is!("y=3; z:=y*y; z", 9);
	is!("t=0; for i in 1...3 { t += 1 }; z := t*10; z", 30);
}

#[test]
fn assigning_a_getter_is_an_error_naming_the_definition() {
	common::fails_with("y=3; z:=y*y; z = 6", "z := y*y");
	common::fails_with("z := 5; z = 6; z", "z := 5");
}

#[test]
fn an_object_getter_runs_at_every_read() {
	is!("o = {a: 1, s := 2+3}; o.s", 5);
	is!("i=0; o = {s := i*10}; i=5; o.s", 50);
	is!("i=1; o = {a: 2, s := i+1}; i=7; o.s + o.a", 10);
}

#[test]
fn a_getter_of_now_reads_the_clock_at_every_use() {
	is!("time := now; a = time; b = time; b >= a", true);
	is!("time := now; (time in \"UTC\").year >= 2026", true);
	is!("t = now; later := t; later == t", true);
}

fn hints_of(code: &str) -> Vec<(String, String)> {
	warp::normalize::capture_hints(|| warp::wasm_emitter::eval(code)).1.into_iter().map(|hint| (hint.canonical, hint.reason)).collect()
}

#[test]
fn a_getter_over_constants_gets_the_needless_charging_note() {
	is!("area := 3*4; area+1", 13);
	assert!(hints_of("area := 3*4; area").iter().any(|(canonical, reason)| canonical == "area = 3*4" && reason == "area never changes"));
	assert!(hints_of("y=3; z := y*y; y=4; z").iter().all(|(_, reason)| !reason.contains("never changes")));
}

#[test]
fn an_effectful_getter_warns_at_the_definition() {
	warp::diagnostic::take_warnings();
	is!("t := clock(); t > 0", true);
	warp::diagnostic::take_warnings();
	warp::wasm_emitter::eval("t := clock(); t");
	assert!(warp::diagnostic::take_warnings().iter().any(|warning| warning.message.contains("t runs clock() at every read; write t = clock() for one value")));
}

// a hint shows only code the user wrote: a def returning a lambda (lowered to closure_new) gets none
#[test]
fn a_function_factory_gets_no_hint_of_lowered_code() {
	is!("def mk() { k => { t = k * 2; t } }; g = mk(); g(4)", 8);
	let hints = hints_of("def mk() { k => { t = k * 2; t } }; g = mk(); g(4)");
	assert!(hints.iter().all(|(canonical, _)| !canonical.contains("closure")), "{hints:?}");
}
