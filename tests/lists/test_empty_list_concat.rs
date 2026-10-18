// card null-typed: `[]` is ø, the empty list: concatenating it to a list needs no `if ys {…}` check
use crate::common::fails_with;
use crate::is;

#[test]
fn an_empty_list_variable_concatenates() {
	is!("xs = [1, 2]; ys = []; c = xs + ys; count(c)", 2);
	is!("xs = [1, 2]; ys = []; c = ys + xs; count(c)", 2);
	is!("xs = [1, 2]; count(xs.concat(xs))", 4);
	is!("count(concat([1], [2, 3]))", 3);
	fails_with("n = 5; m = ø; n + m", "m may be ø");
}

/// card traps-wasm: two empty lists concatenate to ø, not a null trapping in the caller
#[test]
fn two_empty_lists_concatenate_to_the_empty_list() {
	is!("count([1][0..0] + [])", 0);
	is!("xs = []; ys = [1][0..0]; xs = xs + ys; count xs", 0);
	is!("xs = []; for i in [1, 2] { xs = xs + [i][0..0] }; count(xs + [3])", 1);
}

/// a melody whose notes round to no samples is an empty sound (probes/sound/empty_melody.warp)
#[cfg(feature = "native")]
#[test]
fn a_melody_too_fast_for_any_sample_plays_nothing() {
	is!("use sound\ntempo=1000000\nmelody([C5, G5], beat(1/8))\n7", 7);
}
