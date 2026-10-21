//! card lib-compiler: a text in an `and`/`or` condition is tested by its emptiness, as `if name` tests it alone (decision
//! empty-is-not-missing: has items is plain `if x`); a typed text parameter failed with "not an int"
use crate::is;

#[test]
fn a_text_in_a_logical_condition_is_true_when_it_has_characters() {
	is!("g(k: text): int := { if k and yes { return 1 }; 2 }; g(\"y\")", 1);
	is!("g(k: text): int := { if k and yes { return 1 }; 2 }; g(\"\")", 2);
	is!("g(k: text): int := { if yes and k { return 1 }; 2 }; g(\"y\")", 1);
	is!("g(k: text): int := { if k or no { return 1 }; 2 }; g(\"\")", 2);
	is!("g(k: text): int := { if k or yes { return 1 }; 2 }; g(\"\")", 1);
	is!("g(n: int): int := { if n and n > 2 { return 1 }; 2 }; g(3)", 1);
	is!("g(n: int): int := { if n and n > 2 { return 1 }; 2 }; g(0)", 2);
}
