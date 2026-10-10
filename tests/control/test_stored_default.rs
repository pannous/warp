// Card stored-visits: `stored visits default 0` says what `stored visits = 0` means: the value the last run kept, 0 only
// the first time; `stored x = v` stays its synonym
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_stored_value_has_a_default() {
	is!("stored theme_default default \"dark\"; theme_default", "dark");
	is!("stored list_default default [1 2]; count(list_default)", 2);
}

#[test]
fn a_stored_default_is_only_the_first_runs() {
	eval("stored visits_default default 0; visits_default += 1");
	eval("stored visits_default default 0; visits_default += 1");
	is!("stored visits_default default 0; visits_default", 2);
	is!("stored visits_default = 0; visits_default", 2);
}
