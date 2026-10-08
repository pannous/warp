// Card effect-handlers step 1 (notes/effect_handlers.md): `on ask {…} in {…}` handles ask while the block runs, in it
// and in the functions it calls; the innermost handler wins; the handler's value is what emit gives (tail-resumptive)
use crate::is;

#[test]
fn a_block_handler_answers_emits_in_the_block() {
	is!("on ask { 42 } in { emit ask }", 42);
	is!("x = on ask { 42 } in { y = emit ask; y + 1 }; x", 43);
}

#[test]
fn a_block_handler_answers_the_functions_the_block_calls() {
	is!("compute() := (emit ask) * 2; on ask { 21 } in { compute() }", 42);
	is!("compute() := emit ask{n: 4}; on ask { event.n * 10 } in { compute() }", 40);
}

#[test]
fn the_innermost_handler_wins_and_ends_with_its_block() {
	is!("compute() := emit ask; on ask { 1 } in { on ask { 2 } in { compute() } }", 2);
	is!("compute() := emit ask; on ask { 1 } in { a = on ask { 2 } in { compute() }; a * 10 + compute() }", 21);
}

#[test]
fn outside_any_block_the_program_handler_answers() {
	is!("on ask { 7 }; compute() := emit ask; a = on ask { 2 } in { compute() }; a * 10 + compute()", 27);
}

#[test]
fn a_handler_emitting_its_own_event_reaches_the_next_handler_out() {
	is!("on ask { 1 } in { on ask { y = emit ask; y + 10 } in { emit ask } }", 11);
}
