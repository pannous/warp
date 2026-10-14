//! Python's offside rule: the lines indented (tabs or spaces) below a line ending in `:` are its block;
//! Ruby/Lua `do … end` and `then … else … end` enclose all their statements

use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn an_indented_block_after_a_trailing_colon_is_the_body() {
	is!("i = 0\nwhile i < 10:\n\ti += 1\n\tif i == 3: break\ni", 3);
	is!("i = 0\nwhile i < 10:\n    i += 1\n    if i == 3: break\ni", 3);
	is!("x = 5\ny = 0\nif x > 3:\n    y = 1\n    y += 10\ny", 11);
	is!("def f(x):\n    y = x * 2\n    return y + 1\nf(3)", 7);
	is!("def f(x):\n\ty = x * 2\n\ty + 1\nf(3)", 7);
	is!("s = 0\nfor x in [1, 2, 3]:\n    s += x\n    s += 10\ns", 36);
	is!("n = 0\nfor i in 0..4:\n    if i % 2 == 0:\n        n += 1\n        n += 100\n    n += 1000\nn", 4202);
}

#[test]
fn do_end_and_then_end_enclose_all_their_statements() {
	is!("i = 0; while i < 10 do i += 1; if i == 3 then break end end; i", 3);
	is!("i = 0\nwhile i < 10 do\n  i += 1\n  if i == 3 then break end\nend\ni", 3);
	is!("s = 0; i = 0; while i < 3 do s += i; i += 1 end; s", 3);
	is!("x = 0\nif x == 0 then\n  x = 5\n  x += 1\nelse\n  x = 9\nend\nx", 6);
}

#[test]
fn one_end_for_both_then_and_do_is_ambiguous() {
	let result = eval("i = 0; while i < 10 do i += 1; if i == 3 then break end; i").to_string();
	assert!(result.contains("ambiguous `end`"), "{result}");
}
