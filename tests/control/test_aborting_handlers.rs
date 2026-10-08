// Card effect-handlers-abort (step 3, notes/effect_handlers.md): `break value` in a block handler does not resume, the
// `on … in {…}` block ends with that value, however deep the emit was
use crate::is;

#[test]
fn break_in_a_handler_ends_its_block() {
	is!("on fail { break 0 } in { emit fail; 5 }", 0);
	is!("x = on fail { break 7 } in { y = emit fail; y + 100 }; x", 7);
	is!("compute() := { emit fail; 5 }; on fail { break -1 } in { compute() }", -1);
}

#[test]
fn the_abort_skips_the_rest_of_the_block() {
	is!("n = 0; on fail { break 1 } in { n += 10; emit fail; n += 100 }; n", 10);
	is!("deep(k) := if k == 0 then emit fail else deep(k - 1) + 1; on fail { break 42 } in { deep(50) }", 42);
}

#[test]
fn a_resuming_handler_and_the_end_of_the_block_still_work() {
	is!("on fail { break 0 } in { 5 }", 5);
	is!("r = on ask { 3 } in { on fail { break 0 } in { emit ask * 2 } }; r", 6);
}

#[test]
fn the_innermost_aborting_handler_takes_it_and_the_outer_goes_on() {
	is!("a = on fail { break 1 } in { b = on fail { break 2 } in { emit fail; 9 }; b * 10 }; a", 20);
}

#[test]
fn a_try_inside_the_block_does_not_catch_the_abort() {
	is!("on fail { break 3 } in { try { emit fail; 4 } else 5 }", 3);
	is!("on fail { break 3 } in { try { emit fail; 4 } else 5 }; try { raise \"x\" } else 6", 6);
}

#[test]
fn an_abort_for_an_outer_block_passes_the_inner_one() {
	is!("on fail { break 1 } in { on stop { break 2 } in { emit fail; 9 } }", 1);
	is!("f() := on fail { break 1 } in { emit fail; 2 }; f() + f()", 2);
}

#[test]
fn any_value_and_a_loop_break_inside_the_handler() {
	is!("on fail { break \"no\" } in { emit fail; \"yes\" }", "no");
	is!("n = 0; on fail { for i in 1 to 3 { n += i; if i == 2 { break } }; break n } in { emit fail; 99 }", 3);
}

#[test]
fn an_error_after_an_abort_out_of_a_try_keeps_its_message() {
	crate::common::fails_with("on fail { break 3 } in { try { emit fail; 4 } else 5 }; x = [1, 2]; x#5", "index out of range");
}
