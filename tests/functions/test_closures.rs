//! Real closures: a lambda or function is a value (a GC struct holding a typed function reference and its captured values, captured
//! by value) that can be stored, passed, returned and called later. Compile-time specialisation stays where the function is known.
use crate::is;
use warp::wasm_emitter::eval;
use crate::common::fails_with;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_lambda_in_a_variable() {
	is!("f = x => x*2; f(3)", 6);
}

#[test]
fn test_returned_closure() {
	is!("make_adder(n) := (x => x+n); add2 = make_adder(2); add2(5)", 7);
	is!("make_adder(n) := (x => x+n); add2 = make_adder(2); add3 = make_adder(3); add2(5) + add3(5)", 15);
	is!("make_adder(n) := x => x+n; make_adder(10)(1)", 11);
}

#[test]
fn test_closure_captures_by_value() {
	is!("k=3; f = make(k); make(n) := (x => x*n); k=100; f(2)", 6);
	is!("mul(n) := (x => x*n); k=3; g = mul(k); k=5; g(2)", 6);
}

#[test]
fn test_capturing_lambda_passed_to_a_function() {
	is!("apply(f, x):=f(x); k=4; apply(y=>y*k, 3)", 12);
	is!("twice(f, x):=f(f(x)); n=1; twice(y=>y+n, 5)", 7);
}

#[test]
fn test_closure_held_in_a_variable_passed_on() {
	is!("apply(f, x):=f(x); make_adder(n) := (x => x+n); a = make_adder(4); apply(a, 1)", 5);
}

#[test]
fn test_map_and_reduce_with_a_closure_variable() {
	assert_eq!(printed("make_adder(n) := (x => x+n); a = make_adder(10); map [1 2 3] a"), "[11 12 13]");
	assert_eq!(printed("k=2; scale = make(k); make(n) := (x => x*n); [1 2 3].map(scale)"), "[2 4 6]");
	is!("sum_with(f, xs) := reduce xs f; k=0; sum_with((a b)->a+b+k, [1 2 3])", 6);
}

#[test]
fn test_function_chosen_at_run_time() {
	is!("twice(x):=x*2; square(x):=x*x; pick(c) := if c then twice else square; f = pick(1); f(5)", 10);
	is!("twice(x):=x*2; square(x):=x*x; pick(c) := if c then twice else square; f = pick(0); f(5)", 25);
}

#[test]
fn test_closures_returning_text() {
	is!("greeter(g) := (name => g + \" \" + name); hi = greeter(\"hi\"); hi(\"bob\")", "hi bob");
}

#[test]
fn test_not_a_function_stays_loud() {
	fails_with("map [1 2 3] 5", "map needs a function, got 5 (an Int); fix: map [1 2 3] (x => …)");
	fails_with("apply(f, x):=f(x); apply(5, 2)", "apply needs a function for parameter f, got 5 (an Int)");
}

#[test]
fn test_float_closure() {
	is!("scale(k) := (x => x*k); s = scale(1.5); s(2)", 3.0);
}

#[test]
fn test_compose_returns_a_closure() {
	is!("twice(x):=x*2; inc(x):=x+1; compose(f, g) := (x => f(g(x))); h = compose(twice, inc); h(3)", 8);
	is!("compose(f, g) := (x => f(g(x))); k=10; h = compose(y=>y+k, y=>y*2); h(3)", 16);
}

#[test]
fn test_curried_closures() {
	is!("add(a) := (b => (c => a+b+c)); g = add(1); h = g(2); h(3)", 6);
}

#[test]
fn per_site_result_kind_when_arity_mixes_function_and_int() {
	// Without per-site kinds, closure_call_1 joins Function-returning (g(2)) and Int-returning (f / h(3))
	// targets to Data; an exact Int binding then fails. Each callee's known targets keep the kind precise.
	is!("add(a) := (b => (c => a+b+c)); g = add(1); h = g(2); f = x => x + 1; x:int = h(3) + f(4); x", 11);
	is!("twice(x):=x*2; inc(x):=x+1; compose(f, g) := (x => f(g(x))); h = compose(twice, inc); f = x => x + 1; x:int = h(3) + f(2); x", 11);
}

#[test]
fn test_a_closure_prints_as_its_function() {
	assert_eq!(printed("twice(x):=x*2; pick(c) := if c then twice else twice; pick(1)"), "twice");
}

#[test]
fn test_chained_calls_of_returned_closures() {
	is!("add(a) := (b => (c => a+b+c)); add(1)(2)(3)", 6);
	is!("add(a) := (b => (c => a+b+c)); g = add(1); h = g(2); h(3)+1", 7);
}

#[test]
fn test_a_lambda_without_parameters() {
	is!("make_counter() := (c=0; () => c+1); k=make_counter(); k()", 1);
}

#[test]
fn test_a_list_of_closures() {
	is!("fs=[x=>x+1, x=>x*2]; f=fs#2; f(5)", 10);
	is!("triple(x):=x*3; fs=[triple, x=>x+1]; g=fs#1; g(7)", 21);
}

#[test]
fn test_a_parameter_declared_a_function() {
	is!("apply(f:function, x) := f(x); k=1; apply(x=>x+k, 1)", 2);
}

#[test]
fn test_calling_a_non_function_or_the_wrong_arity_is_loud() {
	fails_with("make_adder(n) := (x => x+n); a = make_adder(1); a(1, 2)", "wrong number of arguments");
	fails_with("apply(f, x):=f(x); n=3; apply(n, 1)", "not a function");
}

#[test]
fn an_item_of_a_function_list_called_directly() {
	is!("fs=[x=>x*2, x=>x+1]; (fs#2)(5)", 6);
	is!("fs=[x=>x*2, x=>x+1]; fs#2(5)", 6);
	is!("fs=[x=>x*2, x=>x+1]; fs#1(5)", 10);
}

#[test]
fn a_loop_variable_over_function_values_is_called() {
	is!("fs=[x=>x*2, x=>x+1]; s=0; for f in fs { s+=f(1) }; s", 4);
}

#[test]
fn a_function_value_out_of_try_or_a_block_is_called() {
	is!("f = try 1 + [1 2]#5 else (e => e * 2); f(4)", 8);
	is!("f = try (x=>x+1) else (e => e * 2); f(4)", 5);
	is!("f = {x => x+1}; f(2)", 3);
}

#[test]
fn closures_of_ints_are_called_without_boxes() {
	// 300000 calls through a closure parameter: seconds when every call boxed its argument and result
	is!("apply(g, n) := { s=0; for i in 0..n { s = g(s) }; s }; k=1; apply(x => x+k, 300000)", 300000);
	is!("fs=[x=>x+1, x=>x*2.5]; fs#2(2)", 5);
}

#[test]
fn closures_given_floats_take_floats() {
	is!("fs=[x=>x*2.0, x=>x+0.5]; g = fs#1; g(1.5 as float)", 3.0);
	is!("k=2.5 as float; f = (x => x*k); fs=[f]; h=fs#1; h(2.0 as float)", 5.0);
}

#[test]
fn a_float_reassigned_from_its_closure_stays_a_float() {
	is!("apply(g, n) := { s=0.5 as float; for i in 0..n { s = g(s) }; s }; k = 1.5 as float; apply(x => x*k, 3)", 1.6875);
}
