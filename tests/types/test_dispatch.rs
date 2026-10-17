// Generic functions (notes/dispatch.md "Methods"): a function name is a set of methods told apart by the type of their
// first argument. `def sort(d:Deck)` adds a method to sort, it never replaces sort for lists and texts; `x.f(args)` is
// `f(x, args)`; the most specific method wins (a class over its parent, a typed one over the library or an untyped one)
use crate::common::fails_with;
use crate::is;

const DECK: &str = "class Deck{n:int}; def sort(d:Deck):=d.n; ";
const SHAPES: &str =
	"class square{side:int}; class rect{w:int h:int}; area(s:square) := s.side*s.side; area(r:rect) := r.w*r.h; ";
const ANIMALS: &str =
	"class Animal{name:text}; class Dog extends Animal{}; def speak(a:Animal):=\"...\"; def speak(d:Dog):=\"woof\"; ";

fn with(prelude: &str, code: &str) -> String {
	format!("{prelude}{code}")
}

#[test]
fn a_method_for_a_class_keeps_the_library_function() {
	is!(&with(DECK, "Deck(3).sort()"), 3);
	is!(&with(DECK, "sort(Deck(5))"), 5);
	is!(&with(DECK, "[3,1].sort()"), warp::ints(vec![1, 3]));
	is!(&with(DECK, "sort([2,1])"), warp::ints(vec![1, 2]));
	is!(&with(DECK, "xs=[3,1]; xs.sort()"), warp::ints(vec![1, 3]));
	is!("class Deck{n:int}; def size(d:Deck):=d.n; size(\"abc\") + size(Deck(2))", 5);
}

#[test]
fn methods_of_several_classes_share_a_name() {
	is!(&with(SHAPES, "area(rect(2,5)) + area(square(3))"), 19);
	is!(&with(SHAPES, "rect(2,5).area() + square(3).area"), 19);
}

#[test]
fn the_most_specific_method_wins() {
	is!(&with(ANIMALS, "speak(Dog(\"rex\"))"), "woof");
	is!(&with(ANIMALS, "speak(Animal(\"x\"))"), "...");
	is!(
		"class Animal{name:text}; class Dog extends Animal{}; def speak(a:Animal):=\"...\"; Dog(\"rex\").speak()",
		"..."
	);
}

#[test]
fn no_method_for_a_type_is_a_welcoming_error() {
	fails_with(&with(SHAPES, "class circle{r:int}; area(circle(1))"), "area has no method for circle");
	fails_with(&with(SHAPES, "class circle{r:int}; area(circle(1))"), "area(s:circle)");
}

#[test]
fn two_equally_specific_methods_are_an_ambiguity_error() {
	let program = "class photo{x:int}; class sketch{x:int}; class image{x:int}; image like photo; image like sketch; f(a:photo):=1; f(b:sketch):=2; f(image(1))";
	fails_with(program, "f of an image is ambiguous");
}

#[test]
fn a_value_of_run_time_type_dispatches_at_run_time() {
	is!(&with(SHAPES, "t=0; for s in [square(3), rect(2, 5)] { t += area(s) }; t"), 19);
	is!(&with(DECK, "def g(x) := sort(x); g(Deck(7))"), 7);
	is!(&with(DECK, "def g(x) := sort(x); g([3,1,2])"), warp::ints(vec![1, 2, 3]));
	is!(&with(ANIMALS, "def g(x) := speak(x); g(Dog(\"rex\"))"), "woof");
}

#[test]
fn sort_keeps_its_types() {
	is!("class Deck{n:int}; def sort(d:Deck):=d.n; def sort(x):=0; sort([2,1]) + sort(Deck(4))", 4);
	is!("def sort(x):=0; sort([2,1]) == [1 2]", true);
}

#[test]
fn a_method_in_a_class_body_is_a_method_of_the_generic_function() {
	is!("class Deck{n:int; def sort():=n*2}; [3,1].sort()", warp::ints(vec![1, 3]));
	is!("class Deck{n:int; def sort():=n*2}; d=Deck(4); sort(d)", 8);
	is!("class Deck{n:int; def sort():=n*2}; def g(x):=x.sort(); g(Deck(5))", 10);
	is!("class Deck{n:int; def sort():=n*2}; def g(x):=x.sort(); g([2,1])", warp::ints(vec![1, 2]));
	is!("class Bag{items; def sum():=7}; Bag([1]).sum + sum([1,2])", 10);
	is!("class Bag{items}; def sum(b:Bag):=7; sum(Bag([1])) + sum([1,2])", 10);
}

#[test]
fn classes_sharing_a_library_named_method_keep_the_library_function() {
	let classes = "class Bag{items; def sum():=7}; class Box{n:int; def sum():=n}; ";
	is!(&with(classes, "Bag([1]).sum() + Box(5).sum() + sum([1,2])"), 15);
	is!(&with(classes, "t=0; for b in [Bag([1]), Box(5)] { t += b.sum() }; t"), 12);
}

#[test]
fn a_call_with_more_arguments_is_the_same_call_either_way() {
	is!("class Account{balance:int}; deposit(a:Account, n:int) := a.balance + n; Account(5).deposit(3) * 100 + deposit(Account(5), 3)", 808);
	is!("class Vec2{x:int}; def add(a:Vec2, b:Vec2):=Vec2(a.x+b.x); Vec2(1).add(Vec2(2)).x * 10 + add(Vec2(3), Vec2(4)).x", 37);
	is!(
		"class Deck{n:int; def take(k):=n - k}; Deck(5).take(2) * 100 + take(Deck(7), 2) * 10 + count(take([1,2,3],2))",
		352
	);
	is!(
		"class Deck{n:int; def take(k):=n - k}; def g(x, k) := x.take(k); g(Deck(9), 2) * 10 + count(g([1,2,3], 2))",
		72
	);
	is!("class sq{s:int}; class rc{w:int}; scale(a:sq, k):=a.s*k; scale(b:rc, k):=b.w+k; t=0; for x in [sq(2), rc(3)] { t = t*100 + x.scale(10) }; t", 2013);
}

const SPACE: &str = "class Ship{n:int}; class Rock{n:int}; collide(a:Ship, b:Rock) := \"ship hits rock\"; collide(a:Rock, b:Ship) := \"rock hits ship\"; collide(a:Ship, b:Ship) := \"ships bump\"; ";
const MEETINGS: &str = "class Animal{name:text}; class Dog extends Animal{}; meet(a:Animal, b:Animal) := \"hi\"; meet(a:Dog, b:Animal) := \"woof\"; ";

#[test]
fn every_positional_argument_picks_the_method() {
	is!(&with(SPACE, "collide(Ship(1), Rock(2))"), "ship hits rock");
	is!(&with(SPACE, "collide(Rock(1), Ship(2))"), "rock hits ship");
	is!(&with(SPACE, "Ship(1).collide(Ship(2))"), "ships bump");
	is!("class Ship{n:int}; class Rock{n:int}; describe(k, s:Ship) := k + 1; describe(k, r:Rock) := k + 2; describe(10, Rock(1)) * 100 + describe(10, Ship(1))", 1211);
}

#[test]
fn the_method_closest_at_every_position_wins() {
	is!(&with(MEETINGS, "meet(Dog(\"a\"), Dog(\"b\"))"), "woof");
	is!(&with(MEETINGS, "meet(Animal(\"a\"), Dog(\"b\"))"), "hi");
	fails_with(&with(MEETINGS, "meet(a:Animal, b:Dog) := \"hey\"; meet(Dog(\"a\"), Dog(\"b\"))"), "ambiguous");
	fails_with(&with(SPACE, "collide(Rock(1), Rock(2))"), "collide has no method for (Rock, Rock)");
}

#[test]
fn every_positional_argument_dispatches_at_run_time() {
	let program = "t = \"\"; for a in [Ship(1), Rock(1)] { for b in [Ship(2), Rock(2)] { if not (a is Rock and b is Rock) { t = t + collide(a, b) + \";\" } } }; t";
	is!(&with(SPACE, program), "ships bump;ship hits rock;rock hits ship;");
	is!(
		&with(MEETINGS, "def g(x, y) := meet(x, y); g(Dog(\"a\"), Animal(\"b\")) + g(Animal(\"a\"), Dog(\"b\"))"),
		"woofhi"
	);
}

const SIZED: &str = "class Box{size:int}; size(b:Box) := b.size * 10; b = Box(3); ";

#[test]
fn a_method_named_like_a_field_keeps_both_and_warns() {
	is!(&with(SIZED, "b.size"), 3);
	is!(&with(SIZED, "size(b)"), 30);
	is!(&with(SIZED, "b.size()"), 30);
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval(&with(SIZED, "b.size"));
	let warnings = warp::errors::diagnostic::take_warnings();
	assert!(
		warnings.iter().any(|warning| warning.message.contains("Box has a field size and a method size")),
		"{warnings:?}"
	);
}

#[test]
fn a_method_named_like_any_standard_function_keeps_it() {
	is!("class P{x:int; def max():=x}; P(4).max() + [1,2].max() + max(3, 5)", 11);
	is!("class Rope{s:text; def trim():=\"rope\"}; Rope(\"a\").trim() + \" a \".trim()", "ropea");
	is!("class Coin{v:float; def round():=7}; Coin(1.5).round() + round(2.0)", 9);
	is!("class Box{n:int; def int():=n}; Box(3).int() + \"42\".int()", 45);
	is!("class C{x:int; def double():=x*2}; C(3).double() + 2.double()", 8);
}
