// Extension methods on an existing type (card functions-extension): Kotlin `fun Int.twice() = this * 2`,
// called as a method `3.twice()` or a function `twice(3)`
use crate::is;

#[test]
fn an_extension_method_extends_a_builtin_type() {
	is!("fun Int.twice() = this * 2; 3.twice()", 6);
	is!("fun Int.twice(): Int { return this * 2 }; twice(4)", 8);
	is!("fun Int.plus(k) = this + k; 3.plus(4)", 7);
	is!("fun String.shout() = this + \"!\"; \"hi\".shout()", "hi!");
}

#[test]
fn an_extension_method_extends_a_class() {
	is!("class Point{x:int; y:int}; fun Point.sum() = this.x + this.y; Point(3, 4).sum()", 7);
}

#[test]
fn a_swift_extension_block_adds_methods() {
	is!("extension Int { func twice() -> Int { return self * 2 } }; 3.twice()", 6);
	is!("extension Int { func twice() -> Int { self * 2 }; func plus(k: Int) -> Int { self + k } }; 3.twice().plus(1)", 7);
}

#[test]
fn a_smart_scope_defines_methods_of_a_type() {
	// wiki/inventions.md "Smart scopes": in the scope of a type, `it` is the value the method is called on
	is!("Number {\n Square = it*it\n}\n3.Square", 9);
	is!("Number { Square = it*it }; Square(4)", 16);
}
