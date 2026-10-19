// A list provably of ints (or floats) is held as a wasm GC array: O(1) index and count, no boxing per element.
// Results must be exactly those of the cons-cell list.
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_int_array_index_count_sum_for() {
	is!("xs=[3,1,4]; xs#2", 1);
	is!("xs=[3,1,4]; xs[0]", 3);
	is!("xs=[3,1,4]; #xs", 3);
	is!("xs=[3,1,4]; count xs", 3);
	is!("xs=[3,1,4]; xs.count", 3);
	is!("xs=[3,1,4]; sum xs", 8);
	is!("xs=[3,1,4]; s=0; for x in xs {s+=x}; s", 8);
	is!("xs=[3,1,4]; s=0; for x in xs {s=s*10+x}; s", 314);
	is!("xs=[3,1,4]; i=xs#3; i*2", 8);
}

#[test]
fn test_int_array_assignment_keeps_value_semantics() {
	is!("xs=[1,2,3]; xs#2=7; xs#2", 7);
	is!("xs=[1,2,3]; xs[1]=7; sum xs", 11);
	is!("xs=[1,2,3]; ys=xs; ys#1=9; xs#1", 9); // P200b: ys and xs are one list
	is!("xs=[1,2,3]; ys=xs; ys#1=9; ys#1", 9);
	is!("xs=[1,2,3]; s=0; for x in xs {xs#1=10; s+=x}; s", 6);
	is!("xs=[1,2,3]; xs=[4,5]; #xs", 2);
}

#[test]
fn test_int_array_reads_back_as_the_same_node() {
	assert_eq!(printed("xs=[3,1,4]; xs"), printed("[3,1,4]"));
	assert_eq!(printed("xs=[3,1,4]; xs#2=7; xs"), "[3 7 4]");
	assert_eq!(printed("xs=[3,1,4]; ys=xs; ys"), printed("[3,1,4]"));
	assert_eq!(eval("xs=[3,1,4]; xs"), eval("[3,1,4]"));
}

#[test]
fn test_int_array_errors_like_cons_list() {
	fails_with("xs=[3,1,4]; xs#4", "index");
	fails_with("xs=[3,1,4]; xs#0", "index");
	fails_with("xs=[3,1,4]; xs#4=1", "index");
}

#[test]
fn test_zero_filled_int_list_is_an_array() {
	is!("xs = int[5]; xs[2] = 7; sum xs", 7);
	is!("xs = int[5]; #xs", 5);
	assert_eq!(printed("xs = int[3]; xs[1] = 4; xs"), "[0 4 0]");
}

#[test]
fn test_int_expression_items_are_an_array() {
	is!("a=2; xs=[a, a+1, a*3]; xs#3", 6);
	assert_eq!(printed("a=2; xs=[a, a+1, 7/7]; xs"), "[2 3 1]");
	fails_with("xs=[3,1,4]; xs#-1", "index out of range");
}

#[test]
fn test_appended_int_list_is_an_array() {
	is!("xs=[]; for i in 1..5 {xs.add(i*i)}; xs#3", 9);
	is!("xs=[]; for i in 1..5 {xs.add(i*i)}; sum xs", 30);
	is!("xs=[7]; for i in 1..20 {xs.add(i)}; #xs", 20);
	assert_eq!(printed("xs=[]; for i in 1..5 {xs.add(i)}; xs"), "[1 2 3 4]");
	assert_eq!(printed("xs=[]; xs"), printed("[]"));
	is!("xs=[1]; ys=xs; xs.add(2); #ys", 2); // P200b: ys and xs are one list
	is!("xs=[1]; ys=xs; ys.add(2); #xs", 2);
}

#[test]
fn test_map_and_element_wise_over_int_arrays() {
	is!("xs=[1,2,3]; ys = xs.map(x => x*x); ys#3", 9);
	assert_eq!(printed("xs=[1,2,3]; xs.map(x => x+1)"), "[2 3 4]");
	assert_eq!(printed("xs=[1,2,3]; map xs {it*10}"), "[10 20 30]");
	is!("xs=[1,2,3]; each xs {it*10}", 30);
}

#[test]
fn test_int_arrays_inside_functions() {
	assert_eq!(printed("def squares(n){out=[]; for i in 1..n {out.add(i*i)}; out}; squares(4)"), "[1 4 9]");
	is!("def total(){xs=int[5]; for i in 1..5 {xs[i]=i}; sum xs}; total()", 10);
	is!("def second(){xs=[5,6,7]; xs#2}; second()", 6);
}

#[test]
fn test_float_list_is_a_float_array() {
	assert_eq!(eval("xs=[1.5f, 2.5f]; xs#2"), eval("2.5f"));
	is!("xs=[1.5f, 2.5f]; #xs", 2);
	fails_with("xs=[1.5f, 2.5f]; xs#3", "index out of range");
}

#[test]
fn test_float_array_elements_are_numbers() {
	assert_eq!(eval("xs=[1.5f, 2.5f]; xs#1*2"), eval("3.0f"));
	assert_eq!(eval("xs=[1.5f, 2.5f]; y=xs#2; y"), eval("2.5f"));
	assert_eq!(eval("xs=[1.5f, 2.5f]; sum xs"), eval("4.0f"));
	assert_eq!(eval("xs=[1.5f, 2.5f]; s=0; for x in xs {s = s + x}; s"), eval("4.0f"));
	assert_eq!(eval("xs=[1.5f, 2.5f]; xs#1=0.25f; xs"), eval("[0.25f, 2.5f]"));
	assert_eq!(eval("xs=[]; xs.add(1.5f); xs.add(2.5f); xs"), eval("[1.5f, 2.5f]"));
	assert_eq!(eval("xs=[1.5f, 2.5f]; xs.map(x=>x*2)"), eval("[3.0f, 5.0f]"));
}

#[test]
fn test_sum_of_a_typed_list_is_one_operation() {
	is!("xs=[1,2,3]; sum xs", 6);
	is!("xs=[]; sum xs", 0);
	// leaving the fixnum range takes the exact loop
	assert_eq!(eval("xs=[4611686018427387904, 4611686018427387904]; sum xs"), eval("9223372036854775808"));
	assert_eq!(eval("xs=[1.5f,2.5f]; sum xs"), eval("4.0f"));
	assert_eq!(eval("xs=[1.5f,2.5f]; y = sum xs; y * 2"), eval("8.0f"));
	assert_eq!(printed("xs=[1,2,3]; [sum xs]"), "[6]");
}

/// card int-list: `[0.0, 0.0]` is a list of exact ints; a float written into it later widens its element type from the
/// start, so a read before that write (max's temporary) is a float too
#[test]
fn test_float_written_into_exact_decimal_list() {
	assert_eq!(
		printed("shown = [0.0, 0.0]; for i in 0..2 { shown[i] = max(sqrt(2.0), shown[i]) }; shown"),
		"[1.4142135623730951 1.4142135623730951]"
	);
	assert_eq!(
		eval("shown = [0.0, 0.0]; for i in 0..2 { shown[i] = max(sqrt(2.0), shown[i]) * 0.75 }; shown[0] * 0.75"),
		eval("sqrt(2.0) * 0.75 * 0.75")
	);
	// the same for a plain variable read before its float write
	assert_eq!(eval("x = 0.0; y = 0.0; for i in 0..3 { y = x * 2; x = sqrt(2.0) }; y"), eval("sqrt(2.0) * 2"));
}

#[test] // card list-parameter: `xs: list` declares no element type, so its elements are whatever each caller passes
fn a_list_parameter_holds_elements_of_any_type() {
	crate::is!("g(ls: list) := ls.map(p => p.n); g([{n: \"a\"}])", warp::texts(vec!["a"]));
	crate::is!("g(ls: list) := ls.filter(p => p.n == \"a\"); #g([{n: \"a\"}, {n: \"b\"}])", 1);
	crate::is!("g(ls: list) := { out = \"\"; for c in ls { out += c }; out }; g([\"a\" \"b\"])", "ab");
	crate::is!("g(xs: list) := { s = 0; for x in xs { s += x }; s }; g([1 2 3]) + 1", 7);
}
