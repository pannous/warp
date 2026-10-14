// Card on-error-abort, step 1 of notes/scoped_handlers.md (leave on throw): a block left by an error or by another
// event's abort no longer leaves its handler active
use crate::is;

const OUTER: &str = "on ask { 1 }; ";

#[test]
#[ignore = "cleanup-edge: cleanup-drop-scoped-handlers: block-scoped handlers `on e {…} in {…}` were dropped"]
fn an_error_caught_outside_the_block_leaves_its_handler() {
	is!(&format!("{OUTER}x = try (on ask {{ 5 }} in {{ raise oops }}) else 0; emit ask"), 1);
	is!(&format!("{OUTER}f() := on ask {{ 5 }} in {{ [1]#3 }}; y = try f() else 0; emit ask"), 1);
	is!(&format!("{OUTER}g() := on ask {{ 5 }} in {{ raise oops }}; try g() catch e {{ emit ask }}"), 1);
	is!("g() := on ask { 5 } in { raise oops }; try g() catch e { \"got \" + e }", "got oops");
}

#[test]
#[ignore = "cleanup-edge: cleanup-drop-scoped-handlers: block-scoped handlers `on e {…} in {…}` were dropped"]
fn another_events_abort_leaves_the_inner_handler() {
	is!(&format!("{OUTER}r = on fail {{ break 0 }} in {{ on ask {{ 5 }} in {{ emit fail }} }}; emit ask + r"), 1);
}

#[test]
#[ignore = "cleanup-edge: cleanup-drop-scoped-handlers: block-scoped handlers `on e {…} in {…}` were dropped"]
fn a_block_that_ends_still_answers_inside() {
	is!(&format!("{OUTER}try (on ask {{ 5 }} in {{ emit ask }}) else 0"), 5);
}
