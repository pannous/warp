// Optional parameters as TypeScript `x?: number`, Swift/Kotlin `x: Int? = nil`, C# `int? x = null` write them: a
// missing argument is ø, and `a ?? b` (Swift, C#, JS) is a unless a is ø (0 and false stay, unlike `or`)
use crate::is;

#[test]
fn a_missing_optional_argument_is_empty() {
	is!("def f(x?){ x ?? 5 }; f()", 5); // TS function f(x?: number) { return x ?? 5 }
	is!("def f(x?){ x ?? 5 }; f(2)", 2);
	is!("def f(x?){ x ?? 5 }; f() + f(2)", 7);
	is!("def f(a, b?){ a + (b ?? 10) }; f(1)", 11);
	is!("def f(x:int?){ x ?? 5 }; f()", 5); // Swift func f(x: Int? = nil)
	is!("def f(x=ø){ x ?? 5 }; f()", 5);
}

#[test]
fn coalescing_keeps_zero_and_false() {
	is!("x = 0; x ?? 5", 0);
	is!("x = ø; x ?? 5", 5);
	is!("x = ø; y = ø; x ?? y ?? 3", 3);
}

#[test]
fn maybe_marks_an_optional_parameter() {
	// P125 (user): "x=ø, maybe x, or x?  Same as with optional types."
	is!("def f(maybe x){ x ?? 5 }; f() + f(2)", 7);
	is!("def f(a, maybe b){ b ?? a }; f(1) + f(1, 3)", 4);
	is!("def f(maybe int x){ x ?? 5 }; f()", 5);
	is!("def f(x: maybe int){ x ?? 5 }; f()", 5);
}
