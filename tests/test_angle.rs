use warp::smarty::{float_data28, smarty32};
use warp::Node;
use warp::Node::Empty;
use warp::{is, skip, Number};

#[test]
#[ignore]
fn test_function_params() {
	// eq!(parse("f(x)=x*x").param(),"x");
	// is!("f(x)=x*x;f(3)", "9"); // functions => angle!
}

//
//#[test] fn testOperatorBinding() {
//	assert_ast("a and b", "and(a,b)");
//}

//#[cfg(feature = "EMSCRIPTEN")]{
//#define assert_expect(x);
//#define async_yield(y);
//}
//
#[test]
#[ignore]
fn test_call() {
	// #[cfg(feature = "WASMTIME")]{
	// 	warn("square 3  => SIGABRT in WASMTIME! must be bug there!?");
	// 	return ;
	// }
	is!("square 3", 9);
	is!("square(3)", 9);
	//	functionSignatures["square"] = (*new Signature()).add(i32t).returns(i32t).import();
	is!("square(1+2)", 9);
	is!("square 1+2", 9);
	//	preRegisterSignatures();
	is!("1+square 2+3", 26);
	is!("1 + square 1+2", 10);
	skip!(
		// interpreter broken lol
		is!("1+square(2+3)", 26);
		is!("square{i:3}", 9) //todo: match arguments!
	);
}

#[test]
// #[ignore]
fn test_truthy_and() {
	is!("0.0 and 4.0", 0.0);
	is!("0.0 && 4.0", 0.0);
	is!("0.0 & 4.0", 0.0);
	is!("4.0 and 0.0", 0.0);
	is!("4.0 and 5.0", 5.0);
	is!("0.0 and 4", 0.0);
	is!("4.0 and 0", 0);
	is!("4.0 and 5", 5);
	is!("0 and 4.0", 0);
	is!("4 and 0.0", 0.0);
	is!("4 and 5.0", 5.0);
	// skip!( // todo
	is!("4 and 'a'", 'a');
	is!("4 and '🍏'", '🍏');
	is!("4 and '🍏🍏🍏'", "🍏🍏🍏");
	is!("0 and 'a'", 0);
	is!("0 and '🍏'", 0);
	is!("0 and '🍏🍏🍏'", 0);
	is!("2 and 3 or 4", 3);
	is!("false else 3", 3);
	is!("1 and 0 or 4", 4);
	is!("false or 3", 3);
	is!("[] or 3", 3);
	is!("() or 3", 3);
	is!("'' or 3", 3);
	is!("\"\" or 3", 3);
}

#[test]
fn test_if() {
	// Curly brace syntax: if cond { then } else { else }
	is!("if 0 {3} else {4}", 4);
	is!("if 2 {3} else {4}", 3);
	is!("if (2) {3} else {4}", 3);
	is!("if(2){3} else {4}", 3);
	is!("if(0){3} else {4}", 4);
	is!("if (0) {3} else {4}", 4);
	is!("if 2 {3}", 3);
	is!("if (0) {3}", false);
	is!("if 0 {3}", false);
	is!("if (2) {3}", 3);
	is!("if(2){3}", 3);
	is!("if(0){3}", false);
	is!("if 0 {3} else 4", 4);
	is!("if 2 {3} else 4", 3);
	is!("if (2) {3} else 4", 3);
	is!("if(2){3} else 4", 3);
	is!("if(0){3} else 4", 4);
	is!("if (0) {3} else 4", 4);

	// Then/else keyword syntax: if cond then then else else
	is!("if 2 then 3 else 4", 3);
	is!("if 1 then 0 else 4", 0);
	is!("if 0 then 3 else 4", 4);

	// Colon syntax: if cond:then [else else]
	is!("if 2:3", 3);
	is!("if 2:{3}", 3);
	is!("if 2:3 else 4", 3);
	is!("if 2:{3} else 4", 3);
	is!("if 2:{3} else {4}", 3);
	is!("if 2:3 else {4}", 3);
	is!("if 0:3 else 4", 4);
	is!("if 0:{3} else 4", 4);
	is!("if 0:{3} else {4}", 4);
	is!("if 0:3 else {4}", 4);

	// Colon syntax with parens around condition
	is!("if(2):{3}", 3);
	is!("if(2):{3} else 4", 3);
	is!("if(2):{3} else {4}", 3);
	is!("if(0):{3} else {4}", 4);
	is!("if(0):{3} else 4", 4);

	// Falsy non-numeric values - not yet implemented (requires emitter changes)
	// is!("if '':3", false);   // empty string falsy
	// is!("if ():3", false);   // empty parens falsy
	// is!("if ø:3", false);    // null falsy
	// is!("if {}:3", false);   // empty block falsy
	// is!("if x:3", false);    // undefined var falsy

	// Function-call style if - not yet implemented
	// is!("if{2 , 3 , 4}", 3);
	// is!("if(2,3,4)", 3);
	// is!("if({2},{3},{4})", 3);
}

#[test]
fn test_if_call_zero() {
	is!("def six(){6};six()", 6);
	is!("def six(){6};2+six()", 8);
	is!("def zero(){0};zero()", 0);
	is!("def zero(){0};2+zero()", 2);

	// Curly brace syntax with function call condition
	is!("def zero(){0};if(zero()){3} else 4", 4);
	is!("def zero(){0};if(zero()){3} else {4}", 4);
	is!("def zero(){0};if (zero()) {3} else {4}", 4);
	is!("def zero(){0};if (zero()) {3}", false);
	is!("def zero(){0};if (zero()) {3} else 4", 4);
	is!("def zero(){0};if(zero()){3}", false);

	// Colon syntax not yet implemented
	// is!("def zero(){0};if(zero()):{3}", false);
	// is!("def zero(){0};if(zero()):{3} else {4}", 4);
	// is!("def zero(){0};if(zero()):{3} else 4", 4);
}

#[test]
fn test_if_two() {
	is!("def two(){2};two()", 2);
	is!("def two(){2};two()+2", 4);
	is!("def two(){2};two()+two()", 4);
	is!("def two(){2};two()*two()", 4);

	// Curly brace syntax with function call condition
	is!("def two(){2};if two() {3} else {4}", 3);
	is!("def two(){2};if (two()) {3} else 4", 3);
	is!("def two(){2};if(two()){3} else 4", 3);
	is!("def two(){2};if (two()) {3} else {4}", 3);
	is!("def two(){2};if(two()){3} else {4}", 3);
	is!("def two(){2};if two() {3}", 3);
	is!("def two(){2};if (two()) {3}", 3);
	is!("def two(){2};if(two()){3}", 3);
	is!("def two(){2};if two() {3} else 4", 3);
	is!("def two(){2};if (two()) {two()} else {4}", 2);
	is!("def two(){2};if(two()){two()} else {4}", 2);
	is!("def two(){2};if two() {two()}", 2);
	is!("def two(){2};if (two()) {two()}", 2);
	is!("def two(){2};if(two()){two()}", 2);
	is!("def two(){2};if two() {two()} else 4", 2);

	// Then/else keyword syntax
	is!("def two(){2};if two() then 3 else 4", 3);
	is!("def two(){2};if two() then two() else 4", 2);

	// Colon syntax not yet implemented
	// is!("def two(){2};if(two()):{3}", 3);
	// is!("def two(){2};if two() : 3 else 4", 3);
	// is!("def two(){2};if(two()):{3} else 4", 3);
	// is!("def two(){2};if(two()):{3} else {4}", 3);
	// is!("def two(){2};if two():{3} else 4", 3);
	// is!("def two(){2};if two():3 else 4", 3);
	// is!("def two(){2};if two():{3} else {4}", 3);
	// is!("def two(){2};if two():3 else {4}", 3);
	// is!("def two(){2};if two():{3}", 3);
	// is!("def two(){2};if two():3", 3);
	// is!("def two(){2};if(two()):{two()} else 4", 2);
	// is!("def two(){2};if(two()):{two()} else {4}", 2);
	// is!("def two(){2};if two():{two()} else 4", 2);
	// is!("def two(){2};if two():two() else 4", 2);
	// is!("def two(){2};if two():{two()} else {4}", 2);
	// is!("def two(){2};if two():two() else {4}", 2);
	// is!("def two(){2};if two():{two()}", 2);
	// is!("def two(){2};if two():two()", 2);

	// Lisp-style function call syntax not yet implemented
	// is!("def two(){2};if{two() , 3 , 4}", 3);
	// is!("def two(){2};if(two(),3,4)", 3);
	// is!("def two(){2};if({two()},{3},{4})", 3);
	// is!("def two(){2};if{two() , two() , 4}", 2);
	// is!("def two(){2};if(two(),two(),4)", 2);
	// is!("def two(){2};if({two()},{two()},{4})", 2);
}

#[test]
fn test_if_math() {
	is!("2+0", 2);

	// Then/else keyword syntax with math
	is!("if 2+0 then 3 else 4+0", 3);
	is!("if 1 then 0 else 4+0", 0);

	// Curly brace syntax with math expressions
	is!("if 0*2 {3*1} else {4*1}", 4);
	is!("if (2*1) {3*1} else 4+0", 3);
	is!("if(2*1){3*1} else 4+0", 3);
	is!("if(0*2){3*1} else 4+0", 4);
	is!("if (2*1) {3*1} else {4*1}", 3);
	is!("if(2*1){3*1} else {4*1}", 3);
	is!("if(0*2){3*1} else {4*1}", 4);
	is!("if (0*2) {3*1} else {4*1}", 4);
	is!("if (0*2) {3*1}", false);
	is!("if (0*2) {3*1} else 4+0", 4);
	is!("if 0*2 {3*1}", false);
	is!("if (2*1) {3*1}", 3);
	is!("if(2*1){3*1}", 3);
	is!("if(0*2){3*1}", false);

	// Truthy or
	is!("4 or 3*1", 4);
	is!("2+2 or 3*1", 4);

	is!("if 0+2:{3*1} else 4+0", 3);
	is!("if(0*2):{3*1} else {4*1}", 4);
	is!("if 2+0 : 3 else 4+0", 3);
	is!("if 0*2:{3*1} else {4*1}", 4);
	is!("if 0*2:3*1", false);
	is!("if 0*2:3 else {4*1}", 4);
	is!("if {0}:3 else 4+0", 4);
	is!("if 0*2:3 else 4+0", 4);
	is!("if 0*2:{3*1} else 4+0", 4);
	is!("if(2*1):{3*1}", 3);
	is!("if(2*1):{3*1} else 4+0", 3);
	is!("if(2*1):{3*1} else {4*1}", 3);
	is!("if 0+2:3 else 4+0", 3);
	is!("if 0+2:{3*1} else {4*1}", 3);
	is!("if 0+2:3 else {4*1}", 3);
	is!("if(0*2):{3*1}", false);
	is!("if(0*2):{3*1} else 4+0", 4);
	is!("if 0*2:{3*1} else 4+0", 4);
	is!("if 0+2:{3*1}", 3);
	is!("if 0+2:3*1", 3);
}
#[test]
fn test_if_gt() {
	// Truthy or with comparisons
	is!("1<0 or 3", 3);
	is!("1<0 else 3", 3);
	is!("4 or 3", 4);
	is!("2 and 3 or 4", 3);
	is!("1 and 0 or 4", 4);

	// Curly brace syntax with comparisons
	is!("if (1<2) {3} else {4}", 3);
	is!("if (1<0) {3}", false);
	is!("if (0<1) {3}", 3);
	is!("if 0>1 {3} else {4}", 4);
	is!("if (2<3) {3} else 4", 3);
	is!("if(2<4){3} else 4", 3);
	is!("if(3<0){3} else 4", 4);
	is!("if (2<3) {3} else {4}", 3);
	is!("if(2<4){3} else {4}", 3);
	is!("if(3<0){3} else {4}", 4);
	is!("if (0<1) {3} else 4", 3);
	is!("if 0>1 {3}", false);
	is!("if (2<3) {3}", 3);
	is!("if(2<4){3}", 3);
	is!("if(3<0){3}", false);
	is!("if 0>1 {3} else 4", 4);

	// Then/else keyword syntax with comparisons
	is!("if 1<2 then 3 else 4", 3);
	is!("if 1 then 0 else 4", 0);

	// Colon syntax not yet implemented
	// is!("if(2<4):{3}", 3);
	// is!("if(3<0):{3} else {4}", 4);
	// is!("if 0>1 : {3} else {4}", 4);
	// is!("if 0>1 : 3 else {4}", 4);
	// is!("if 0>1 : 3 else 4", 4);
	// is!("if 0>1:3 else 4", 4);
	// is!("if 0>1:{3} else {4}", 4);
	// is!("if 0>1:3 else {4}", 4);
	// is!("if 1<2 : 3 else 4", 3);
	// is!("if 0>1:3", false);
	// is!("if(2<4):{3} else 4", 3);
	// is!("if(2<4):{3} else {4}", 3);
	// is!("if 1<2:{3} else 4", 3);
	// is!("if 1<2:3 else 4", 3);
	// is!("if 1<2:{3} else {4}", 3);
	// is!("if 1<2:3 else {4}", 3);
	// is!("if(3<0):{3}", false);
	// is!("if(3<0):{3} else 4", 4);
	// is!("if 0>1:{3} else 4", 4);
	// is!("if 1<2:{3}", 3);
	// is!("if 1<2:3", 3);
}
#[test]
#[ignore]
fn test_switch_evaluation() {
	is!("{a:1+1 b:2}(a)", 2);
	is!("x=a;{a:1 b:2}(x)", 1);
	// functor switch(x,xs)=xs[x] or xs[default]
}

#[test]
#[ignore]
fn test_switch() {
	//	todo if(1>0) ... innocent groups
	is!("{a:1 b:2}[a]", 1);
	is!("{a:1 b:2}[b]", 2);
}

#[test]
fn test_smart_types() {
	// smarty32  is pretty useless but serves as nice demonstration of smart64,
	// which   is pretty useless but serves as nice demonstration of multi return
	// which  is pretty useless but serves as nice demonstration of node as wit/gc type
	// which is pretty useless but serves as nice demonstration of emitted structs
	assert_eq!(smarty32(0xC000221a), '√');
	assert_eq!(smarty32(0xC000221a), "√");
	assert_eq!(smarty32(0xC0000020), ' ');
	assert_eq!(smarty32(0x00000000), Empty);
	assert_eq!(smarty32(0x00000009), 9);
	assert_eq!(smarty32((-9i32) as u32), -9);
	assert_eq!(smarty32(0xFFFFFFFFu32), -1);
	// assert_eq!(smarty32(0x00000000), 0);
	// char* hi="Hello";
	// strcpy2(&memoryChars[0x1000], hi);
	// printf!(">>>%s<<<", &memoryChars[0x1000]);
	// assert!(Node(0x90001000)==hi);
	// short typ=getSmartType(string_header_32);
	// assert!(typ==0x1);
	// printf!("%08x", '√');// ok 0x221a
	println!("{}", '𒈚'); // too small: character too large for enclosing character literal type
	assert_eq!(smarty32(0xC001221A), '𒈚');
	assert_eq!(smarty32(0xC001221A), "𒈚");
	//	assert!(Node(0xD808DE1A)=='𒈚'); // utf-8-bytes

	// let node = smarty32(0.1f32.to_bits()); accident!
	let node = smarty32(float_data28(0.1f32)); // safe!
	if let Node::Number(n) = node {
		if let Number::Float(f) = n {
			assert!(f - 0.1 < 0.00001);
		} else {
			panic!("Expected Float number");
		}
	} else {
		panic!("Expected Number node");
	}
}
// #[test] fn nl() {
//     put_char('\n');
// }

//Prescedence type for Precedence
#[test]
// #[ignore]
fn test_logic_precedence() {
	#[cfg(not(feature = "WASM"))]
	{
		// assert!(precedence("and") > 1);
		// assert!(precedence("and") < precedence("or"));
	}
	is!("true", true);
	is!("false", false);
	is!("true or true", true);
	is!("true or false", true);
	is!("true and false", false);
	is!("1 ⋁ 1 ∧ 0", 1);
	is!("1 ⋁ 0 ∧ 1", 1);
	is!("1 ⋁ 0 ∧ 0", 1);
	is!("0 ⋁ 1 ∧ 0", 0);
	is!("0 ⋁ 0 ∧ 1", 0);
	is!("true or true and false", true);
	is!("true or false and true", true);
	is!("true or false and false", true);
	is!("false or true and false", false);
	is!("false or false and true", false);
}
#[test]
#[ignore]
fn test_all_angle() {
	// emmitting or not
	//	test_smart_types();
	test_if();
	// test_call(); in testTodoBrowser();
	skip!(
		testSwitch();
		testFunctionParams(); // TODO!
	);
}

#[test]
#[ignore]
fn test_angle() {
	test_all_angle();
}
