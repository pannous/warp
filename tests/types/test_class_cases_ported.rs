// Class cases ported from other languages (notes/classes.md "Ported cases"): what Python, Java, Kotlin, Swift, C#
// and Ruby programmers write first, in warp
use crate::is;

const ANIMALS: &str = "class animal{name; speak() := name + \" makes a sound\"}";

#[test]
fn fields_have_defaults() {
	// Kotlin `class Point(val x: Int = 0, val y: Int = 0)`, Python `def __init__(self, x=0, y=0)`
	is!("class point{x:int=0; y:int=0}; point().x + point(3).x", 3);
	is!("class point{x:int=1; y:int=2}; p = point(); p.y", 2);
}

#[test]
fn methods_read_fields_with_or_without_self() {
	is!("class rect{w:int; h:int; area() := w * h}; rect(3, 4).area()", 12);
	is!("class rect{w:int; h:int; area() := self.w * self.h}; rect(3, 4).area()", 12);
	is!("class account{balance:int; after(amount:int) := balance + amount}; account(10).after(5)", 15);
}

#[test]
fn a_method_calls_another_method_without_self() {
	// Java/C#/Kotlin: an unqualified call inside a method is a call on this
	is!("class rect{w:int; h:int; area() := w * h; twice() := 2 * area()}; rect(3, 4).twice()", 24);
	is!("class rect{w:int; h:int; area() := w * h; twice() := 2 * self.area()}; rect(3, 4).twice()", 24);
	is!("class square{side:int; area := side * side; twice() := 2 * area}; square(3).twice()", 18);
}

#[test]
fn a_subclass_method_calls_an_inherited_method_without_self() {
	is!("class a{x:int; f() := x}; class b extends a{g() := f() + 1}; b(1).g()", 2);
	is!("class a{x:int; f() := x}; class b extends a{g() := f() + 1}; class c extends b{h() := g() + 1}; c(1).h()", 3);
}

#[test]
fn an_override_calls_the_parent_method_with_super() {
	// Java `super.speak()`, Python `super().speak()`, Kotlin `super.speak()`
	is!(&format!("{ANIMALS}; class dog extends animal{{speak() := super.speak() + \" (woof)\"}}; dog(\"Rex\").speak()"), "Rex makes a sound (woof)");
	is!(&format!("{ANIMALS}; class dog extends animal{{speak() := super.speak() + \"!\"}}; class puppy extends dog{{speak() := super.speak() + \"!\"}}; puppy(\"Bo\").speak()"), "Bo makes a sound!!");
}

#[test]
fn a_subclass_instance_is_its_parent_type() {
	// Java `instanceof`, Python `isinstance`
	is!("class animal{name}; class dog extends animal{}; dog(\"Rex\") is animal", true);
	is!("class animal{name}; class dog extends animal{}; d = dog(\"Rex\"); d is dog", true);
	is!("class animal{name}; class dog extends animal{}; animal(\"Cat\") is dog", false);
}

#[test]
fn each_subclass_overrides_the_shared_method() {
	is!("class shape{area() := 0}; class sq extends shape{s:int; area() := s * s}; class ci extends shape{r:int; area() := 3 * r * r}; sq(2).area() + ci(1).area()", 7);
}

#[test]
fn instances_compare_by_value() {
	// Kotlin data class, Python @dataclass __eq__
	is!("class point{x:int; y:int}; point(1, 2) == point(1, 2)", true);
	is!("class point{x:int; y:int}; point(1, 2) == point(2, 1)", false);
}

#[test]
fn objects_change_and_copy() {
	is!("class counter{n:int; inc() := n += 1}; c = counter(0); c.inc(); c.inc(); c.n", 2);
	is!("class person{name; age:int}; p = person(\"Ann\", 40); p.age = 41; p.age", 41);
	// Kotlin `copy`, a method giving a changed new instance
	is!("class point{x:int; y:int; moved(dx:int) := point(x + dx, y)}; point(1, 2).moved(3).x", 4);
}

#[test]
fn objects_hold_objects() {
	is!("class engine{hp:int}; class car{e:engine}; car(engine(90)).e.hp", 90);
	is!("class person{name; age:int}; person{name:\"Ann\" age:3}.age", 3);
}

#[test]
fn a_method_reads_a_field_with_a_default() {
	is!("class Point{x:int=1; f() := x}; Point().f() + Point(5).f()", 6);
	is!("class Circle{r:int; k = 3; f() := k * r}; Circle(2).f()", 6);
}

#[test]
fn a_stack_class_pushes_onto_its_list() {
	// the classic Stack of Java/Python: a method that adds to a list field changes the object
	is!("class Stack{items:[int]; push(x:int) := items.add(x); size() := count(items)}; s = Stack([]); s.push(1); s.push(2); s.size()", 2);
	is!("class Stack{items:[int]; push(x:int) := items.add(x); top() := items#count(items)}; s = Stack([]); s.push(1); s.push(7); s.top()", 7);
	is!("class Bag{items:[text]; put(x) := self.items.add(x)}; b = Bag([]); b.put(\"a\"); b.put(\"b\"); count(b.items)", 2);
}

#[test]
fn a_list_field_grows_by_add() {
	is!("class Holder{items:[int]}; p = Holder([]); p.items.add(1); p.items.add(5); count(p.items)", 2);
	is!("class Holder{items:[int]}; p = Holder([3]); p.items.insert(9, at:1); p.items#2", 9);
}

#[test]
fn static_members_belong_to_the_class() {
	// P122 (user, 2026-10-06): `static k = 3` is a member of the class, read as C.k and c.k, never stored per instance
	is!("class Circle{r:int; static k = 3}; Circle.k + Circle(1).k", 6);
	is!("class Circle{r:int; static k = 3; area() := k * r}; Circle(2).area()", 6);
	is!("class Cm{r:int; static unit = \"cm\"}; class M{s:int; static unit = \"m\"}; Cm.unit + M.unit", "cmm");
	crate::common::fails_with("class Circle{r:int; static k = 3}; Circle(1, 2)", "Circle takes 1 fields, got 2");
}

#[test]
fn a_static_counter_changes_from_methods_and_outside() {
	is!("class Clicker{r:int; static clicks = 0; click() := clicks += 1}; x = Clicker(1); x.click(); x.click(); Clicker.clicks", 2);
	is!("class Clicker{r:int; static clicks = 0}; Clicker.clicks = 5; Clicker.clicks", 5);
}

#[test]
fn a_static_method_is_called_on_the_class() {
	// Java static factory, Python @classmethod / @staticmethod
	is!("class Circle{r:int; static doubled(x:int) := Circle(x * 2)}; Circle.doubled(4).r", 8);
}
