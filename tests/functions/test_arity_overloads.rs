// Overloading by arity as in C#, Java, Kotlin, Swift and Julia methods: definitions of one name with different
// parameter counts are separate functions, a call takes the one its argument count fits (static dispatch)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_call_takes_the_definition_of_its_argument_count() {
	is!("def f(a){a}; def f(a,b){a+b}; f(1)*10 + f(2,3)", 15);
	is!("f(a) := a; f(a,b) := a+b; f(a,b,c) := a*b*c; f(7) + f(1,2) + f(2,3,4)", 34);
	is!("area(r) := 3*r*r; area(w, h) := w*h; area(2) + area(2, 5)", 22); // Julia area(r), area(w, h)
}

#[test]
fn an_overload_may_call_another_one() {
	is!("def log(x) { log(x, 10) }; def log(x, base) { x/base }; log(50)", 5.0);
	is!("sum3(a, b) := sum3(a, b, 0); sum3(a, b, c) := a+b+c; sum3(1, 2)", 3);
}

#[test]
fn calls_without_parentheses_pick_by_count_too() {
	is!("f(a) := a; f(a,b) := a+b; f 1 2", 3);
}

#[test]
fn defaults_count_for_the_arguments_a_definition_accepts() {
	is!("f(a) := 1; f(a, b, c=0) := 2; f(5)*10 + f(5, 6)", 12);
	is!("f(a) := 1; f(a, b, c=0) := 2; f(5, 6, 7)", 2);
}

#[test]
fn a_count_no_definition_fits_or_two_fit_is_an_error() {
	fails_with("f(a) := a; f(a,b) := a+b; f(1,2,3)", "f takes 1 or 2 arguments, got 3");
	fails_with("f(a) := 1; f(a, b=2) := 2; f(5)", "`f(5)` fits two definitions");
}

// card arity-overloaded: an overloaded name given as a function is the variant the callee calls it as; as a plain value
// it is the call of the one taking nothing
#[test]
fn an_overloaded_name_given_as_a_function_is_the_variant_called() {
	is!("apply(h, x) := h(x)\nf() := 0\nf(x) := x * 2\napply(f, 3)", 6);
	is!("twice(h) := h() + h()\nf() := 5\nf(x) := x\ntwice(f)", 10);
	is!("f() := 0\nf(x) := x * 2\nsum([1, 2].map(f))", 6);
	is!("f() := 1\nf(a, b) := a + b\n[1, 2, 3].reduce(f)", 6);
	is!("f() := 7\nf(x) := x\ny = f\ny + 1", 8);
}

// card point-label: x.f, x.f(y) and x.f = v of f overloaded by arity call the variant counting x too
#[test]
fn an_overloaded_name_called_on_a_receiver_is_the_variant_counting_it() {
	is!("f(x) := x * 2; f(x, y) := x + y; 3.f + 3.f(4)", 13);
	is!("class P{name: text}\nf(it: P) := it.name\nf(it: P, v) := { it.name = v; it }\np = P(\"ab\")\np.f = \"cd\"\np.f", "cd");
	is!("class P{f: int}\nf(x) := x * 2\nf(x, y) := x + y\nP(5).f", 5);
}

// card point-label (reviewer veto): only an instance of the class the setter takes sets through it; a map's field
// named like an overloaded function is the map's own
#[test]
fn a_map_field_named_like_an_overloaded_function_is_set_on_the_map() {
	is!("pick(a, b) := b\npick(a) := a\nm = {pick: 1}\nm.pick = 5\nm.pick", 5);
	is!("pick(a, b) := b\npick(a) := a\nm = {pick: 1, other: 2}\nm.pick = 5\nm.other", 2);
	is!("pick(a, b) := b\npick(a) := a\nm = {pick: 1}\nm.pick", 1);
}

// card point-label (reviewer veto 2): the receiver decides, not the name: a map elsewhere with a key label leaves a
// class's getter and setter of instances in place
#[test]
fn an_unrelated_map_key_leaves_a_class_setter_in_place() {
	let point = "class Point { x: int; name: text }\nPoint.label := it.name + \"!\"\nPoint.label(v) := it.name = v\n";
	is!(&format!("{point}cfg = {{label: 3}}\np = Point(1, \"a\")\np.label = \"b\"\np.label"), "b!");
	is!(&format!("{point}cfg = {{label: 3}}\ncfg.label = 4\ncfg.label"), 4);
}
