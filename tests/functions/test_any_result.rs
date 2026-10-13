//! `def f(w) as any { … }`: a function returning a value of any kind, a number on one path, a text or list on another
//! (card def, found by self-parser; `->` still works, with a hint toward `as`: decision return-as)
use crate::is;

#[test]
fn a_result_of_any_kind() {
	is!(r#"def f(w) as any { if w == "1" { return 1 }; w }; f("12")"#, "12");
	is!(r#"def f(w) as any { if w == "1" { return 1 }; w }; f("1")"#, 1);
	is!(r#"def f(w) -> any { if w == "1" { return 1 }; w }; f("12")"#, "12");
	is!(r#"def f(w) as any { if w == "l" { return [1, 2] }; w }; #f("l")"#, 2);
}
