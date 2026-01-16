#![allow(mixed_script_confusables)]

use std::process::exit;
use warp::analyzer::analyze;
use warp::extensions::print;
use warp::type_kinds::NodeKind;
use warp::wasm_gc_emitter::eval;
use warp::wasp_parser::parse;
use warp::Node;
use warp::Node::{Empty, False, True};
use warp::{eq, is, skip};

#[test]
fn test_range() {
	is!("0..3", ints(0, 1, 2));       // exclusive: 0, 1, 2
	is!("0..<3", ints(0, 1, 2));      // Swift-style exclusive
	is!("0...3", ints4(0, 1, 2, 3));  // inclusive: 0, 1, 2, 3
	is!("0…3", ints4(0, 1, 2, 3));    // ellipsis = inclusive
	is!("0 to 3", ints4(0, 1, 2, 3)); // inclusive like math
	is!("range 1 3", ints(1, 2, 3));  // range function = inclusive
	skip!(
		is!("[0 to 3]", ints4(0, 1, 2, 3)); // bracketed range
	);
}

fn ints(p0: i32, p1: i32, p2: i32) -> Node {
	Node::ints(vec![p0, p1, p2])
}
fn ints4(p0: i32, p1: i32, p2: i32, p3: i32) -> Node {
	Node::ints(vec![p0, p1, p2, p3])
}

#[test]
fn test_merge_global() {
	#[cfg(feature = "MICRO")]
	{
		return;
	}
	#[cfg(feature = "INCLUDE_MERGER")]
	{
		return; // LOST files: main_global.wasm, lib_global.wasm :(
		let main: Module = loadModule("test/merge/main_global.wasm");
		let lib: Module = loadModule("test/merge/lib_global.wasm");
		let merged: Code = merge_binaries(main.code, lib.code);
		let i: smart_pointer_64 = merged.save().run();
		eq!(i, 42);
	}
}

#[test]
fn test_merge_memory() {
	return; // LOST files: main_memory.wasm, lib_memory.wasm
	#[cfg(feature = "WAMR")]
	{
		return;
	}
	#[cfg(feature = "INCLUDE_MERGER")]
	{
		let main: Module = loadModule("test/merge/main_memory.wasm");
		let lib: Module = loadModule("test/merge/lib_memory.wasm");
		let merged: Code = merge_binaries(main.code, lib.code);
		let i: int = merged.save().run();
		eq!(i, 42);
	}
}
#[test]
fn test_merge_runtime() {
	return; // LOST file: main_memory.wasm (time machine?);
	#[cfg(feature = "INCLUDE_MERGER")]
	{
		let runtime: Module = loadModule("wasp-runtime.wasm");
		let main: Module = loadModule("test/merge/main_memory.wasm"); // LOST :( time machine?
																// let main : Module = loadModule("test/merge/main_global.wasm");
		main.code.needs_relocate = true;
		runtime.code.needs_relocate = false;
		let merged: Code = merge_binaries(runtime.code, main.code);
		let i: int = merged.save().run();
		eq!(i, 42);
	}
}
#[test]
fn test_merge_own() {
	test_merge_memory();
	test_merge_global();
	#[cfg(feature = "MICRO")]
	{
		return;
	}
	#[cfg(feature = "INCLUDE_MERGER")]
	{
		let main: Module = loadModule("test/merge/main2.wasm");
		let lib: Module = loadModule("test/merge/lib4.wasm");
		let merged: Code = merge_binaries(main.code, lib.code);
		//	let merged : Code = merge_binaries(lib.code,main.code);
		let i: int = merged.save().run();
		eq!(i, 42);
	}
}

// #[test] fn test_wasm_stuff();
#[test]
fn test_emitter() {
	#[cfg(not(feature = "RUNTIME_ONLY"))]
	{
		// clearAnalyzerContext();
		// clearEmitterContext();
		// let node : Node = Node(42);
		// let code : Code = emit(node, "42");
		// let resulti : int = code.run();
		// assert!(resulti == 42);
	}
}
const PI: f64 = std::f64::consts::PI;
// const E: f64 = std::f64::consts::E;

#[test]
#[ignore]
fn test_implicit_multiplication() {
	is!("x=3;2x", 6);
	is!("2π", 2.0 * PI);
	skip!(

		is!("x=9;⅓x", 3);
	);
	//    is!("⅓9", 3);
}

#[test]
#[ignore]
fn test_globals() {
	is!("2*π", 2. * PI);
	is!("dub:=it*2;dub(π)", 2. * PI);

	is!("global x=7", 7);
	is!("global x;x=7;x", 7);

	is!("global x=1;x=7;x+1", 8);

	// only the most primitive expressions are allowed in let initializers : global => move to main!
	// test_wasm_todos
	// is!("global x=1+π", 1 + PI);
	is!("global x=1+2", 3);

	is!("global x=7;x+=1", 8);
	is!("global x;x=7;x+=1", 8);
	is!("global x;x=7;x+=1;x+1", 9);
	skip!(

		is!("global x=π;x=7;x", 7);
	);
	is!("global x;x=7;x", 7);
	is!("global x=1;x=7;x", 7);
}

#[test]
#[ignore]
fn test_get_local() {
	is!("add1 x:=it+1;add1 3", 4);
	skip!(

		is!("add1 x:=$0+1;add1 3",  4); // $0 specially parsed now
	);
}

#[test]
#[ignore]
fn test_wasm_function_definiton() {
	//	eq!("add1 x:=x+1;add1 3",  4);
	is!("fib:=if it<2 then it else fib(it-1)+fib(it-2);fib(7)", 13);
	is!("fac:= if it<=0 : 1 else it * fac it-1; fac(5)", (5 * 4 * 3 * 2));

	is!("add1 x:=x+1;add1 3", 4);
	is!("add2 x:=x+2;add2 3", 5);
	skip!(

		is!("expression_as_return:=y=9;expression_as_return",  9);
		is!("addy x:= y=2 ; x+y ; addy 3",  5);
	);

	is!("grows x:=x*2;grows(4)", 8);
	is!("grows:=it*2; grows 3", 6);
	is!("grows:=it*2; grows 3*4", 24);
	is!("grows:=it*2; grows(3*42) > grows 2*3", 1);
	is!("factorial:=it<2?1:it*factorial(it-1);factorial 5", 120);

	//0 , 1 , 1 , 2 , 3 , 5 , 8 , 13 , 21 , 34 , 55 , 89 , 144
	is!("fib x:=if x<2 then x else fib(x-1)+fib(x-2);fib(7)", 13);
	is!("fib:=if it<2 then it else fib(it-1)+fib(it-2);fib(7)", 13);
	skip!(

		is!("fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)", 13);
		is!("fib:=it<2 then it or fib(it-1)+fib(it-2);fib(7)", 13);
		is!("fib:=it<2 or fib(it-1)+fib(it-2);fib(4)", 5);
		is!("fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)", 5);
	);
}
#[test]
#[ignore]
fn test_wasm_ternary() {
	is!("2>1?3:4", 3);
	is!("1>0?3:4", 3);
	is!("2<1?3:4", 4);
	is!("1<0?3:4", 4);
	//	is!("(1<2)?10:255", 255);

	is!("fac:= it<=0 ? 1 : it * fac it-1; fac(5)", (5 * 4 * 3 * 2));
	skip!(

		// What seems to be the problem?
	);
}
#[test]
#[ignore]
fn test_lazy_evaluation() {
	//	if lazy_operators.has(op) and … !numeric? …
	//	if op==or emitIf(not lhs,then:rhs);
	//	if op==or emitIf(lhs,else:rhs);
	//	if op==and emitIf(lhs,then:rhs);
	is!("fac:= it<=0 or it * fac it-1; fac(5)", (5 * 4 * 3 * 2)); // requires lazy evaluation
}

#[test]
#[ignore]
fn test_wasm_function_calls() {
	// todo put square puti putf back here when it works!!
	skip!(

		is!("puts 'ok'",  0);
	);
	is!("i=1;while i<9:i++;i+1", 10);
	is!("ceil 3.7", 4);

	is!("id(3*42) > id 2*3", 1);
	is!("id 123", 123);
	is!("id (3+3)", 6);
	is!("id 3+3", 6);
	is!("3 + id 3+3", 9);
}

#[test]
fn test_const_return() {
	is!("42", 42);
}

#[test]
#[ignore]
fn test_print() {
	// does wasm print? (visual control!!);
	is!("print 42", 42);
	print("OK");
	//	printf!("%llx\n", -2000000000000ll);
	//	printf!("%llx", -4615739258092021350ll);
	/*
	skip!(
		print("a %d c".s() % 3);
		print("a %f c".s() % 3.1);
		print("a %x c".s() % 15);
		printf!("a %d c\n", 3);
		printf!("a %f c\n", 3.1);
		printf!("a %x c\n", 15);
	)
	 */
}

#[test]
#[ignore]
fn test_math_primitives() {
	is!("42.1", 42.1); // todo: let Node : return(42.1) or print value to stdout
	is!("-42.1", 42.1);
	is!("42", 42);
	is!("-42", -42);
	is!("2000000000", 2000000000); // todo stupid smart pointers
	is!("-2000000000", -2000000000);
	is!("2000000000000", 2000000000000i64); // let int64
	is!("-2000000000000", -2000000000000i64);
	is!("x=3;x*=3", 9);
	is!("'hello';(1 2 3 4);10", 10);
	//	data_mode = false;
	is!("i=ø; !i", true);
	is!("0.0", 0); // can't emit float yet
	is!("x=15;x>=14", 1);
	is!("i=1.0;i", 1.0); // works first time but !later in code :(
	is!("i=0.0;i", 0.0); //
	is!("3*-1", -3);
	is!("maxi=3840*2160", 3840 * 2160);
	is!("maxi=3840*2160;maxi", 3840 * 2160);
	is!("blue=255;green=256*255;", 256 * 255);
}

#[test]
#[ignore]
fn test_float_operators() {
	is!("3.0+3.0*3.0", 12);
	is!("42.0/2.0", 21);
	is!("42.0*2.0", 84);
	is!("42.0+2.0", 44);
	is!("42.0-2.0", 40);
	is!("3.0+3.0*3.0", 12);
	is!("3.1>3.0", true);
	is!("2.1<3.0", true);
	is!("i=123.4;i", 123.4); // main returning int
	is!("i=1.0;i", 1.0);
	is!("i=3;i", 3);
	is!("i=1.0;i", 1.0);

	is!("2.1<=3.0", true);

	skip!(

		is!("i=8;i=i/2;i", 4); // make sure i stays a-float
		is!("i=1.0;i=3;i=i/2;i=i*4", 6.0); // make sure i stays a-float
		"BUG IN WASM?? should work!?"
		is!(("3.1>=3.0"), true);
	);

	is!("3.0+3.0*3.0>3.0+3.0+3.0", true);
	is!("3.0+3.0*3.0<3.0*3.0*3.0", true);
	is!("3.0+3.0*3.0<3.0+3.0+3.0", false);
	is!("3.0+3.0*3.0>3.0*3.0*3.0", false); // 0x1.8p+1 == 3.0
	is!("3.0+3.0+3.0<3.0+3.0*3.0", true);
	is!("3.0*3.0*3.0>3.0+3.0*3.0", true);
}

#[test]
#[ignore]
fn test_norm2() {
	is!("1-‖3‖/-3", 2);
	is!("1-‖-3‖/3", 0);
	is!("1-‖-3‖/-3", 2);
	is!("1-‖-3‖-1", -3);
	is!("√9*-‖-3‖/3", -3);
	is!("√9*‖-3‖/-3", -3);
	is!("√9*-‖-3‖/-3", 3);
	is!("f=4;‖-3‖<f", 1);
	is!("i=1;(5-3)>i", 1);
	is!("i=1;‖-3‖>i", 1);
	is!("i=1;‖-3‖<i", 0);
	is!("f=4;‖-3‖>f", 0);
	skip!(

		is!("i=1;x=‖-3‖>i", 1);
		is!("f=4;x=‖-3‖<f", 1);
		is!("i=1;x=‖-3‖<i", 0);
		is!("f=4;x=‖-3‖>f", 0);
	);
}

#[test]
#[ignore]
fn test_norm() {
	test_norm2();
	is!("‖-3‖", 3);
	//    is!("‖3‖-1", 2);
	is!("‖-3‖/3", 1);
	is!("‖-3‖/-3", -1);
	is!("‖3‖/-3", -1);
	is!("-‖-3‖/3", -1);
	is!("-‖-3‖/-3", 1);
	is!("-‖3‖/-3", 1);
	is!("‖-3‖>1", 1);
	is!("‖-3‖<4", 1);
	is!("‖-3‖<1", 0);
	is!("‖-3‖>4", 0);
}

#[test]
#[ignore]
fn test_math_operators() {
	//	is!(("42 2 *"), 84);
	is!("- -3", 3);
	is!("1- -3", 4);
	is!("1 - -3", 4);
	skip!(

		is!("1 - - 3", 4); // -1 uh ok?
		assert_throws("1--3"); // should throw, variable missed by parser! 1 OK'ish
	);

	//	is!("1--3", 4);// should throw, variable missed by parser! 1 OK'ish

	is!("‖-3‖", 3);
	is!("-‖-3‖", -3);
	is!("‖-3‖+1", 4);
	is!("7%5", 2);
	is!("42/2", 21);
	//			WebAssembly.Module doesn't validate: control flow returns with unexpected type. F32 is !a I32, in function at index 0
	is!("42/2", 21);
	is!("42*2", 84);
	is!("42+2", 44);
	is!("42-2", 40);
	is!("3+3*3", 12);
	is!("3+3*3>3+3+3", true);
	is!("3+3*3<3*3*3", true);
	is!("3+3*3<3+3+3", false);
	is!("3+3*3>3*3*3", false);
	is!("3+3+3<3+3*3", true);
	is!("3*3*3>3+3*3", true);
	is!("i=3;i*-1", -3);
	is!("3*-1", -3);
	is!("-√9", -3);

	is!("i=3.7;.3+i", 4);
	is!("i=3.71;.3+i", 4.01);
	#[cfg(feature = "WASM")]
	{
		is!("i=3.70001;.3+i", 4.0000100000000005); // lol todo what?
	}
	#[cfg(not(feature = "WASM"))]
	{
		is!("i=3.70001;.3+i", 4.00001);
	}
	is!("4-1", 3); //
	is!("i=3;i++", 4);
	is!("- √9", -3);
	is!("i=-9;-i", 9);
	#[cfg(feature = "WASM")]
	{
		is!("√ π ²", 3.141592653589793); // fu ;);
	}
	#[cfg(not(feature = "WASM"))]
	{
		is!("√ π ²", std::f64::consts::PI);
	}
	is!("3²", 9);
	skip!(

		is!(("3⁰"), 1); // get UNITY of set (1->e let cast ok?);
		is!(("3¹"), 3);
		is!(("3³"), 27); // define inside wasp!
		is!(("3⁴"), 9 * 9);
	);

	is!("i=3.70001;.3+i", 4);
	is!("i=3.7;.3+i", 4);
}

#[test]
#[ignore]
fn test_math_operators_runtime() {
	is!("3^2", 9);
	is!("3^1", 3);
	is!("42^2", 1764); // NO SUCH PRIMITIVE
	is!("√3^0", 1);
	is!("√3^0", 1.0);
	#[cfg(feature = "WASM")]
	{
		is!("√3^2", 2.9999999999999996); // bad sqrt!?
		eq!("π**2", 9.869604401089358);
	}
	#[cfg(not(feature = "WASM"))]
	{}
	#[cfg(not(feature = "WASM"))]
	{
		is!("√3^2", 3);
		is!("π**2", 9.869604401089358);
	}
}

#[test]
#[ignore]
fn test_comparison_math() {
	// may be evaluated by compiler!
	is!("3*42>2*3", 1);
	is!("3*1<2*3", 1);
	is!("3*42≥2*3", 1);
	is!("3*2≥2*3", 1);
	is!("3*2≤2*3", 1);
	is!("3*2≤24*3", 1);
	is!("3*13!=14*3", 1);
	is!("3*13<=14*3", 1);
	is!("3*15>=14*3", 1);
	is!("3*42<2*3", false);
	is!("3*1>2*3", false);
	is!("3*452!=452*3", false);
	is!("3*13>=14*3", false);
	is!("3*15<=14*3", False);
	is!("3*42≥112*3", false);
	is!("3*2≥112*3", false);
	is!("3*12≤2*3", false);
	is!("3*112≤24*3", false);

	//    is!(("3*452==452*3"), 1) // forces runtime
	//    is!(("3*13==14*3"), False);
}
#[test]
#[ignore]
fn test_comparison_id() {
	// may be evaluated by compiler!
	is!("id(3*42 )> id 2*3", 1);
	is!("id(3*1)< id 2*3", 1);
	skip!(

		is!("id(3*452)==452*3", 1);
		is!("452*3==id(3*452)", 1);
		is!("452*3==id 3*452", 1);
		is!("id(3*452)==452*3", 1);
		is!(("id(3*13)==14*3"), False);
	);
	is!("id(3*42)≥2*3", 1);
	is!("id(3*2)≥2*3", 1);
	is!("id(3*2)≤2*3", 1);
	is!("id(3*2)≤24*3", 1);
	is!("id(3*13)!=14*3", 1);
	is!("id(3*13)<= id 14*3", 1);
	is!("id(3*13)<= id 14*3", 1);

	is!("id(3*15)>= id 14*3", 1);
	is!("id(3*42)< id 2*3", False);
	is!("id(3*1)> id 2*3", False);
	is!("id(3*452)!=452*3", False);
	is!("id(3*13)>= id 14*3", False);
	is!("id(3*15)<= id 14*3", False);
	is!("id(3*13)<= id 14*3", 1);
	is!("id(3*42)≥112*3", false);
	is!("id(3*2)≥112*3", false);
	is!("id(3*12)≤2*3", false);
	is!("id(3*112)≤24*3", false);
}

#[test]
#[ignore]
fn test_comparison_id_precedence() {
	// may be evaluated by compiler!
	skip!(

		is!("id 3*452==452*3", 1) // forces runtime
		is!(("id 3*13==14*3"), False);

		//	Ambiguous mixing of functions `ƒ 1 + ƒ 1 ` can be read as `ƒ(1 + ƒ 1)` or `ƒ(1) + ƒ 1`
		is!("id 3*42 > id 2*3", 1);
		is!("id 3*1< id 2*3", 1);
	);
	is!("id(3*42)> id 2*3", 1);
	is!("id(3*1)< id 2*3", 1);
	is!("id 3*42≥2*3", 1);
	is!("id 3*2≥2*3", 1);
	is!("id 3*2≤2*3", 1);
	is!("id 3*2≤24*3", 1);
	is!("id 3*13!=14*3", 1);
	is!("id 3*13<= id 14*3", 1);
	is!("id 3*13<= id 14*3", 1);

	is!("id 3*15>= id 14*3", 1);
	is!("id 3*42< id 2*3", False);
	is!("id 3*1> id 2*3", False);
	is!("id 3*452!=452*3", False);
	is!("id 3*13>= id 14*3", False);
	is!("id 3*15<= id 14*3", False);
	is!("id 3*13<= id 14*3", 1);
	is!("id 3*42≥112*3", false);
	is!("id 3*2≥112*3", false);
	is!("id 3*12≤2*3", false);
	is!("id 3*112≤24*3", false);
}

#[test]
#[ignore]
fn test_comparison_primitives() {
	is!("42>2", 1);
	is!("1<2", 1);
	is!("42≥2", 1);
	is!("2≥2", 1);
	is!("2≤2", 1);
	is!("2≤24", 1);
	is!("13!=14", 1);
	is!("13<=14", 1);
	is!("15>=14", 1);
	is!("42<2", False);
	is!("1>2", False);
	is!("452!=452", False);
	is!("13>=14", False);
	is!("15<=14", False);
	is!("42≥112", false);
	is!("2≥112", false);
	is!("12≤2", false);
	is!("112≤24", false);
	#[cfg(not(feature = "WASM"))]
	{
		is!("452==452", 1); // forces runtime eq
		is!("13==14", False);
	}
}

#[test]
#[ignore]
fn test_wasm_logic_primitives() {
	skip!(
	// todo: if emit returns Node:
		   is!(("false").name, False.name); // NO LOL emit only returns number
		   is!(("false"), False);
	   );

	is!("true", True);
	is!("true", true);
	is!("true", 1);

	is!("false", false);
	is!("false", False);
	is!("false", 0);

	is!("nil", false);
	is!("null", false);
	is!("null", 0);
	// is!("null",  nullptr);
	is!("ø", false);
	is!("nil", Empty);
}
#[test]
#[ignore]
fn test_wasm_variables0() {
	//	  (func $i (type 0) (result i32)  i32.const 123 return)  NO LOL
	is!("i=123;i", 123);
	is!("i:=123;i+1", 124);
	is!("i=123;i+1", 124);

	is!("i=123;i", 123);
	is!("i=1;i", 1);
	is!("i=false;i", false);
	is!("i=true;i", true);
	is!("i=0;i", 0);
	is!("i:=true;i", true);
	is!("i=true;i", true);
	is!("i=123.4;i", 123); // main returning int
	skip!(

		is!("i=0.0;i", 0.0);
		is!("i=ø;i", nullptr);
		is!("i=123.4;i", 123.4); // main returning int
	);
	is!("8.33333333332248946124e-03", 0); // todo in wasm
	#[cfg(feature = "WASM")]
	{
		is!("8.33333333332248946124e+01", 83.33333333322489);
	}
	#[cfg(not(feature = "WASM"))]
	{
		is!("8.33333333332248946124e+01", 83.333_333_333_224_9);
	}

	is!("8.33333333332248946124e+03", 8_333.333_333_322_49);
	is!("S1  = -1.6666", -1.6666);
	//    is!("grows S1  = -1.6666", -1);
	// may be evaluated by compiler!
}

#[test]
#[ignore]
fn test_wasm_increment() {
	is!("i=2;i++", 3);
	skip!(

		is!("i=0;w=800;h=800;pixel=(1 2 3);while(i++ < w*h){pixel[i]=i%2 };i ", 800 * 800);
		//				assert_error("i:=123;i++", "i is a closure, can't be incremented");
	);
}

#[test]
#[ignore]
fn test_wasm_logic_unary_variables() {
	is!("i=0.0; !i", true);
	is!("i=false; !i", true);
	is!("i=0; !i", true);
	skip!(

		is!("i=true; !i", false);
	);
	is!("i=ø; !i", true);

	is!("i=1; !i", false);
	is!("i=123; !i", false);
}

#[test]
#[ignore]
fn test_self_modifying() {
	is!("i=3;i*=3", 9);
	is!("i=3;i+=3", 6);
	is!("i=3;i-=3", 0);
	is!("i=3;i/=3", 1);
	//	is!("i=3;i√=3",  ∛3); NO i TIMES √
	skip!(

		is!("i=3^1;i^=3",  27);
		assert_throws("i*=3"); // well:
		is!("i*=3",  0);
	);
}

#[test]
#[ignore]
fn test_wasm_logic_unary() {
	is!("not 0.0", true);
	is!("not ø", true);
	is!("not false", true);
	is!("not 0", true);

	is!("not true", false);
	is!("not 1", false);
	is!("not 123", false);
}

#[test]
#[ignore]
fn test_wasm_logic_on_objects() {
	is!("not 'a'", false);
	is!("not {a:2}", false);
	skip!(

		is!("not {a:0}", false); // maybe
	);

	is!("not ()", true);
	is!("not {}", true);
	is!("not []", true);
	is!("not ({[ø]})", true); // might skip :);
}

#[test]
#[ignore]
fn test_wasm_logic() {
	skip!(

		// should be easy to do, but do we really want this?
		is!("true true and", true);
		is!("false true and", false);
		is!("false false and ", false);
		is!("true false and ", false);
		assert!(parse("false and false").length() == 3);
	);
	is!("false and false", false);
	is!("false and true", false);
	is!("true and false", false);
	is!("true and true", true);
	is!("true or false and false", true); // == true or (false);

	is!("false xor true", true);
	is!("true xor false", true);
	is!("false xor false", false);
	is!("true xor true", false);
	is!("false or true", true);
	is!("false or false", false);
	is!("true or false", true);
	is!("true or true", true);

	is!("¬ 1", 0);
	is!("¬ 0", 1);

	is!("0 ⋁ 0", 0);
	is!("0 ⋁ 1", 1);
	is!("1 ⋁ 0", 1);
	is!("1 ⋁ 1", 1);

	is!("1 ∧ 1", 1);
	is!("1 ∧ 0", 0);
	is!("0 ∧ 1", 0);
	is!("0 ∧ 0", 0);

	is!("1 ⋁ 1 ∧ 0", 1);
	is!("1 ⋁ 0 ∧ 1", 1);
	is!("1 ⋁ 0 ∧ 0", 1);
	is!("0 ⋁ 1 ∧ 0", 0);
	is!("0 ⋁ 0 ∧ 1", 0);
	is!("¬ (0 ⋁ 0 ∧ 1)", 1);

	is!("0 ⊻ 0", 0);
	is!("0 ⊻ 1", 1);
	is!("1 ⊻ 0", 1);
	is!("1 ⊻ 1", 0);
}

#[test]
#[ignore]
fn test_wasm_logic_negated() {
	is!("not true and !true", !true);
	is!("not true and !false", !true);
	is!("not false and !true", !true);
	is!("not false and !false", !false);
	is!("not false or !true and !true", !false); // == !false or (not true);

	is!("not true xor !false", !false);
	is!("not false xor !true", !false);
	is!("not true xor !true", !true);
	is!("not false xor !false", !true);
	is!("not true or !false", !false);
	is!("not true or !true", !true);
	is!("not false or !true", !false);
	is!("not false or !false", !false);
}

#[test]
#[ignore]
fn test_wasm_logic_combined() {
	is!("3<1 and 3<1", 3 < 1);
	is!("3<1 and 9>8", 3 < 1);
	is!("9>8 and 3<1", 3 < 1);
	is!("9>8 and 9>8", 9 > 8);
	is!("9>8 or 3<1 and 3<1", 9 > 8); // == 9>8 or (3<1);

	is!("3<1 xor 9>8", 9 > 8);
	is!("9>8 xor 3<1", 9 > 8);
	is!("3<1 xor 3<1", 3 < 1);
	is!("9>8 xor 9>8", 3 < 1);
	is!("3<1 or 9>8", 9 > 8);
	is!("3<1 or 3<1", 3 < 1);
	is!("9>8 or 3<1", 9 > 8);
	is!("9>8 or 9>8", 9 > 8);
	is!("9>8 or 8>9", 9 > 8);
}

#[test]
#[ignore]
fn test_wasm_if() {
	is!("if 2 : 3 else 4", 3);
	is!("if 2 then 3 else 4", 3);
	skip!(

		is!("if(2){3}{4}", 3);
		is!("if({2},{3},{4})", 3);
		is!("if(2,3,4)", 3); // bad border case EXC_BAD_ACCESS because !anayized!
		is!("if(condition=2,then=3)", 3);
		is!("if(condition=2,then=3,else=4)", 3); // this is what happens under the hood (?);
		is!("fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)", 5);
		is!("fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)", 13);
		is!("fib:=it<2 then it or fib(it-1)+fib(it-2);fib(7)", 13);
		is!("fib:=it<2 or fib(it-1)+fib(it-2);fib(4)", 5);
	);
}

#[test]
fn test_wasm_while() {
	is!("i=1;while(i<9){i++};i+1", 10);
}

#[test]
#[ignore]
fn test_wasm_while2() {
	is!("i=1;while i<9:i++;i+1", 10);
	is!("i=1;while(i<9){i++};i+1", 10);
	is!("i=1;while(i<9 and i > -10){i+=2;i--};i+1", 10);
	is!("i=1;while(i<9)i++;i+1", 10);
	is!("i=1;while i<10 do {i++};i", 10);
	is!("i=1;while i<10 and i<11 do {i++};i", 10);
	is!("i=1;while i<9 or i<10 do {i++};i", 10);
	is!("i=1;while(i<10) do {i++};i", 10);
	skip!(
	// fails on 2nd attempt todo
		   is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);
	   );
	is!("i=1;while(i<9)i++;i+1", 10);
}

#[test]
#[ignore]
fn test_square_precedence() {
	// todo!
	is!("π/2^2", PI / 4.);
	is!("(π/2)^2", PI * PI / 4.);
}

#[test]
#[ignore]
fn test_squares() {
	// occasionally breaks in browser! even though right code is emitted HOW??
	is!("square 3", 9);
	is!("1+2 + square 1+2", 12);
	is!("1+2 + square 3+4", 52);
	is!("4*5 + square 2*3", 56);
	is!("3 + square 3", 12);
	is!("1 - 3 - square 3+4", -51); // OK!
	is!("square(3*42) > square 2*3", 1);
	skip!(

		testSquarePrecedence();
	);
}

// ⚠️ CANNOT USE is!
//  in WASM! ONLY via testRun();
#[test]
fn test_old_random_bugs() {
	// ≈ test_recent_random_bugs();
	// some might break due some testBadInWasm() BEFORE!
	is!("-42", -42); // OK!?!
	skip!(

		is!("x:=41;if x>1 then 2 else 3", 2);
		is!("x=41;if x>1 then 2 else 3", 2);
		is!("x:41;if x>1 then 2 else 3", 2);
		is!("x:41;if x<1 then 2 else 3", 3);
		is!("x:41;x+1", 42);
		is!("grows := it * 2 ; grows(4)", 8);
		is!("grows:=it*2;grows(4)", 8);
	);

	//		testGraphQlQuery();
	//	eq!("x", Node(false));// passes now but !later!!
	//	eq!("x", false);// passes now but !later!!
	//	eq!("y", false);
	//	eq!("x", false);

	//0 , 1 , 1 , 2 , 3 , 5 , 8 , 13 , 21 , 34 , 55 , 89 , 144

	//	exit(1);
	//	const let node1 : Node = parse("x:40;x++;x+1");
	//	assert!(node.length==3);
	//	assert!(node[0]["x"]==40);
	//	exit(1);
}

//#[test] fn testRefactor(){
//	wabt::let module : Module = readWasm("t.wasm");
//	refactor_wasm(module, "__original_main", "_start");
//	module = readWasm("out.wasm");
//	assert!(module->funcs.front()->name == "_start");
//}

#[test]
fn test_merge_wabt() {
	#[cfg(feature = "WABT_MERGE")]
	{
		// merge_files({"test/merge/main.wasm", "test/merge/lib.wasm"});
	}
}
#[test]
fn test_merge_wabt_by_hand() {
	#[cfg(feature = "WABT_MERGE")]
	{
		// ?? ;);
		// merge_files({"./playground/test-lld-wasm/main.wasm", "./playground/test-lld-wasm/lib.wasm"});
		let main: wabt::Module = readWasm("test-lld-wasm/main.wasm");
		let module: wabt::Module = readWasm("test-lld-wasm/lib.wasm");
		refactor_wasm(module, "b", "neu");
		remove_function(module, "f");
		Module * merged = merge_wasm2(main, module);
		save_wasm(merged);
		let ok: int = run_wasm(merged);
		let ok: int = run_wasm("a.wasm");
		assert!(ok == 42);
	}
}
#[test]
#[ignore]
fn test_wasm_runtime_extension() {
	#[cfg(feature = "TRACE")]
	{
		printf!("TRACE mode currently SIGTRAP's in test_wasm_runtime_extension. OK, Switch to Debug mode. WHY though?");
	}

	is!("43", 43);
	is!("strlen('123')", 3); // todo broke
	is!("strlen('123')", 3); // todo broke
	skip!(

		//            todo polymorphism
		is!("len('123')", 3);
		is!("len('1235')", 4);
	);
	is!("parseLong('123')", 123);
	is!("parseLong('123'+'456')", 123456);
	#[cfg(not(feature = "TRACE"))]
	{
		// todo why??
		is!("parseLong('123000') + parseLong('456')", 123456);
		is!("x=123;x + 4 is 127", true);
		is!("parseLong('123'+'456')", 123456);
		is!("'123' is '123'", true);
		is!("'123' + '4' is '1234'", true); // ok
	}
	assert_throws("not_ok"); // error
	skip!(

		// WORKED before we moved these to test_functions.h
		// todo activate in wasp-runtime-debug.wasm instead of wasp-runtime.wasm
		is!("test42+1", 43);
		is!("test42i(1)", 43);

		is!("test42f(1)", 43);
		is!("test42f(1.0)", 43.0);
		is!("42.5", 42.5); // truncation ≠ proper rounding!
		is!("42.6", 42.6); // truncation ≠ proper rounding!
		is!("test42f(1.7)", 43.7);
		is!("test42f", 41.5); //default args don't work in wasm! (how could they?);
		is!("test42f", 41.5); /// … expected f32 but nothing on stack
	);
	//	functionSignatures["int"].returns(int32);
	//	is!("printf!('123')", 123);
	// works with ./wasp but breaks in webapp
	// works with ./wasp but breaks now:

	//	is!("okf(1)", 43);
	//	is!("puts 'hello' 'world'", "hello world");
	//	is!("hello world", "hello world");// unresolved symbol printed as is

	skip!(

		is!("x=123;x + 4 is 127", true);
		//	is!("'123'='123'", true);// parsed as key a:b !?!? todo!
		//	is!("'123' = '123'", true);
	);
	is!("'123' == '123'", true);
	is!("'123' is '123'", true);
	is!("'123' equals '123'", true);
	is!("x='123';x is '123'", true);
	//	is!("string('123') equals '123'", true); // string() makes no sense in angle:
	//	is!("'123' equals string('123')", true);//  it is internally already a string whenever needed
	//	is!("atoi0(str('123'))", 123);
	//	is!("atoi0(string('123'))", 123);

	//	is!("oki(1)", 43);
	//	is!("puts('123'+'456');", 123456);// via import !via wasp!
	//is!("grows := it * 2 ; grows(4)", 8);
	//	assert!(Primitive::charp!=Valtype::pointer);

	skip!(

		is!("'123'", 123); // result printed and parsed?
		is!("printf!('123')", 123); // result printed and parsed?
	);
	skip!(
	// if !compiled as RUNTIME_ONLY library:
		   assert!(functionSignatures.has("tests"));
		   is!("tests", 42);
	   );
}

#[test]
#[ignore]
fn test_string_concat_wasm() {
	is!("'Hello, ' + 'World!'", "Hello, World!");
}


#[test]
#[ignore]
fn test_object_properties_wasm() {
	is!("x={a:3,b:4,c:{d:true}};x.a", 3);
	is!("x={a:3,b:true};x.b", 1);
	is!("x={a:3,b:4,c:{d:true}};x.c.d", 1);
	//is!("x={a:3,b:'ok',c:{d:true}};x.b", "ok");
	is!("x={a:3,b:'ok',c:{d:5}};x.c.d", 5); //deep
}

#[test]
#[ignore]
fn test_array_indices_wasm() {
	#[cfg(not(feature = "WEBAPP"))]
	{
		assert_throws("surface=(1,2,3);i=1;k#i=4;k#i") // no such k!
		                                         //	caught in wrong place?
	}

	//	testArrayIndices(); //	assert! node based (non-primitive) interpretation first
	//	data_mode = true;// todo remove hack
	is!("x={1 2 3}; x#3=4;x#3", 4);
	#[cfg(feature = "WASM")]
	{
		is!("puts('ok');", -1); // todo: fix puts return
	}
	#[cfg(feature = "WASMEDGE")]
	{
		is!("puts('ok');", 8);
	}
	#[cfg(not(feature = "WASM"))]
	{
		is!("puts('ok');", 0);
	}
	is!("puts('ok');(1 4 3)#2", 4);
	is!("{1 4 3}#2", 4);

	is!("x={1 4 3};x#2", 4);
	is!("{1 4 3}[1]", 4);
	is!("(1 4 3)[1]", 4);
	assert_throws("(1 4 3)#0");

	#[cfg(not(feature = "WASM"))]
	{
		// TODO!
		is!("'αβγδε'#3", 'γ');
		is!("i=3;k='αβγδε';k#i", 'γ');
	}
	skip!(

		is!("i=3;k='αβγδε';k#i='Γ';k#i", 'Γ'); // todo setCharAt
		is!("[1 4 3]#2", 4); // exactly one op expected in emitIndexPattern
		eq!("[1 2 3]#2", 2); // assert! node based (non-primitive) interpretation first
		assert_throws("(1 4 3)#4"); // todo THROW!
		// todo patterns as lists
	);
	//	let empty_array : Node = parse("pixel=[]");
	//	assert!(empty_array.kind==patterns);
	//
	//	let construct : Node = analyze(parse("pixel=[]"));
	//	assert!(construct["rhs"].kind == patterns or construct.length==1 and construct.first().kind==patterns);
	//	emit("pixel=[]");
	//	exit(0);
}

pub fn assert_throws(_p0: &str) {
	todo!()
}

// random stuff todo: put in proper tests
#[test]
#[ignore]
fn test_wasm_stuff() {
	//	is!("grows := it * 2 ; grows(4)", 8);
	is!("-42", -42);
	is!("x=41;x+1", 42);
	is!("x=40;y=2;x+y", 42);
	is!("id(4*42) > id 2+3", 1);
	skip!(

		is!("grows x := x * 2 ; grows(4)", 8);
		is!("grows := it * 2 ; grows(4)", 8);
		is!("grows:=it*2; grows 3", 6);
		is!("add1 x:=x+1;add1 3",  4);
		is!("fib x:=if x<2 then x else fib(x-1)+fib(x-2);fib(7)", 13);
		is!("fib x:=if x<2 then x else{fib(x-1)+fib(x-2)};fib(7)", 13);
	);
}

// ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
#[test]
#[ignore]
fn test_recent_random_bugs() {
	// fixed now thank god
	// if (!testRecentRandomBugsAgain){return};
	// testRecentRandomBugsAgain = false;
	is!("-42", -42);
	is!("‖3‖-1", 2);
	#[cfg(not(feature = "WASMTIME"))]
	{
		is!("test42+1", 43); // OK in WASM too? todo
		is!("square 3*42 > square 2*3", 1);
		#[cfg(not(feature = "WASM"))]
		{
			test_squares();
		}
	}
	//			WebAssembly.Module doesn't validate: control flow returns with unexpected type. F32 is !a I32, in function at index 0
	is!("42/2", 21); // in WEBAPP

	is!("42.1", 42.1);
	// main returns int, should be pointer to value! let array_header_32 : result => smart pointer!
	//			Ambiguous mixing of functions `ƒ 1 + ƒ 1 ` can be read as `ƒ(1 + ƒ 1)` or `ƒ(1) + ƒ 1`
	is!("id 3*42 > id 2*3", 1);
	is!("1-‖3‖/-3", 2);
	is!("i=true; !i", false);
	// these fail LATER in tests!!

	skip!(

		testLengthOperator();
		is!("i=3^1;i^=3",  27);
		assert_throws("i*=3"); // well:
		is!("i*=3",  0);
	);
	is!("maxi=3840*2160", 3840 * 2160);
	is!("√π²", 3);
	is!("i=-9;√-i", 3);
	is!("1- -3", 4);
	is!("width=height=400;height", 400);
	skip!(

		assert_throws("1--3"); // should throw, variable missed by parser! 1 OK'ish
		is!("x=0;while x++<11: nop;x", 11);
		assert_throws("x==0;while x++<11: nop;x");
	);
	is!("‖-3‖", 3);
	is!("√100²", 100);
	//    is!("puts('ok');", 0);
	let result = parse("{ç:☺}");
	eq!(result["ç"], "☺");
	#[cfg(not(feature = "WASMTIME"))]
	{
		// and !LINUX // todo why
		is!("x=123;x + 4 is 127", true);
		is!("n=3;2ⁿ", 8);
		//	function attempted to return an incompatible value WHAT DO YOU MEAN!?
	}
	// move to tests() once OK'
	skip!(

		is!("i=ø; !i", true); // i !a setter if value ø
		is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);
	);
	is!("add1 x:=x+1;add1 3", 4);
	// is!("for i in 1 to 5 : {puti i};i", 6);// EXC_BAD_ACCESS TODO _👀!
}
#[test]
fn test_square() {
	let π = PI; //3.141592653589793;
	is!("3²", 9);
	is!("3.0²", 9);
	is!("√100²", 100);
	is!("√ π ²", π);
	is!("√π ²", π);
	is!("√ π²", π);
	is!("√π²", π);
	is!("π²", π * π);
	is!("π", PI);
	skip!(
		// TODO: type-annotated declarations need work
		is!("int i=π*1000000", 3141592);
	);
	is!("π*1000000.", 3141592.653589793);
	is!("i=-9;-i", 9);
	is!("- √9", -3);
	skip!(
		// TODO: parser doesn't handle numbers starting with '.'
		is!(".1 + .9", 1);
		is!("-.1 + -.9", -1);
	);
	is!("√9", 3);
	//	is!("√-9 is -3i", -3);// if «use complex numbers»
	skip!(is!(".1", 0.1));
	#[cfg(not(feature = "WASMTIME"))]
	{
		// and !LINUX // todo why
		skip!(

			is!("i=-9;√-i", 3);
		is!("n=3;2ⁿ", 8);
		is!("n=3.0;2.0ⁿ", 8);
		//	function attempted to return an incompatible value WHAT DO YOU MEAN!?
		);
	}
}

#[test]
#[ignore]
fn test_round_floor_ceiling() {
	is!("ceil 3.7", 4);
	is!("floor 3.7", 3); // todo: only if «use math» namespace
					  //	is!("ceiling 3.7", 4);// todo: only if «use math» namespace
	is!("round 3.7", 4);
	//	is!("i=3.7;.3+i", 4);// floor
	// lol "⌊3.7⌋" is cursed and is transformed into \n\t or something in wasm and IDE!
	//	is!("⌊3.7", 3);// floor
	//	is!("⌊3.7⌋", 3);// floor
	//	is!("3.7⌋", 3);// floor
	//	//is!("i=3.7;.3 + ⌊i", 3);// floor
	//	//is!("i=3.7;.3+⌊i⌋", 3);// floor
	//	is!("i=3.7;.3+i⌋", 3);// floor
	//	is!("i=3.7;.3+ floor i", 3);// floor
}
#[test]
#[ignore]
fn test_wasm_typed_globals() {
	//    is!("global int k", 7);//   empty global initializer for int
	is!("global long k=7", 7);
	//    is!("global int k=7", 7); // type mismatch
	is!("global const int k=7", 7); //   all globals without value are imports??
	is!("global mutable int k=7", 7); //   all globals without value are imports??
	is!("global mut int k=7", 7); //   all globals without value are imports??
}

#[test]
#[ignore]
fn test_wasm_mutable_global() {
	//	is!("$k=7",7);// ruby style, conflicts with templates `hi $name`
	//    is!("k::=7", 7);// global variable !visually marked as global, !as good as:
	is!("global k=7", 7); // python style, as always the best
	is!("global k:=7", 7); //  global or function?
	is!("global k;k = 7", 7); // python style, as always the best
						   //    is!("global.k=7", 7);//  currently all globals are exported
	skip!(testWasmMutableGlobal2());
	skip!(testWasmTypedGlobals());
	//    test_wasm_mutable_global_imports();
}

#[test]
#[ignore]
fn test_wasm_mutable_global2() {
	is!("export k=7", 7); //  all exports are globals, naturally.
	is!("export k=7", 7); //  all exports are globals, naturally.
	is!("export f:=7", 7); //  exports can be functions too.
	is!("global export k=7", 7); //  todo warn("redundant keyword global: all exports are globals");
	is!("global int k=7", 7); // python style, as always the best
	is!("global int k:=7", 7); //  global or function?
	is!("export int k=7", 7); //  all exports are globals, naturally.
	is!("export int k=7", 7); //  all exports are globals, naturally.
	is!("export int f:=7", 7); //  exports can be functions too.
	is!("global int k", 0); // todo error without init value?
	is!("export int k", 0); //
}

#[test]
#[ignore]
fn test_wasm_mutable_global_imports() {
	is!("import int k", 7); //  all imports are globals, naturally.
	is!("import const int k", 7); //  all imports are globals, naturally.
	is!("import mutable int k", 7); //  all imports are globals, naturally.

	is!("import int k=7", 7); //  import with initializer
	is!("import const int k=7", 7); //  import with initializer
	is!("import mutable int k=7", 7); //  import with initializer

	is!("import int k=7.1", 7); //  import with cast initializer
	is!("import const int k=7.1", 7); //  import with cast initializer
	is!("import mutable int k=7.1", 7); //  import with cast initializer

	is!("import k=7", 7); //  import with inferred type
	is!("import const k=7", 7); //  import with inferred type
	is!("import mutable k=7", 7); //  import with inferred type
	                           // remember that the concepts of functions and properties shall be IDENTICAL to the USER!
	                           // this does !impede the above, as global exports are !properties, but something to keep in mind
}

#[test]
#[ignore]
fn test_custom_operators() {
	is!("suffix operator ⁰ := 1; 3⁰", 1); // get UNITY of set (1->e let cast ok?);
	is!("suffix ⁰ := 1; 3⁰", 1); // get UNITY of set (1->e let cast ok?);
	is!("suffix operator ³ := it*it*it; 3³", 27); // define inside wasp!
	is!("suffix operator ³ := it*it*it; .5³", 1 / 8);
	is!("suffix ³ := it*it*it; 3³", 27); // define inside wasp!

	//	is!(("alias to let third : the = ³"),1);
	//	is!(("3⁴"),9*9);
}


#[test]
#[ignore]
fn test_import_wasm() {
	//	Code fourty_two=emit(analyze(parse("ft=42")));
	//	fourty_two.save("fourty_two.wasm");
	is!("import fourty_two;ft*2", 42 * 2);
	is!("import fourty_two", 42);
	is!("include fourty_two", 42);
	is!("require fourty_two", 42);
	is!("include fourty_two;ft*2", 42 * 2);
	is!("require fourty_two;ft*2", 42 * 2);
}

#[test]
#[ignore]
fn test_math_library() {
	// todo generic power i as builtin
	#[cfg(not(feature = "WASMTIME"))]
	{
		skip!(

			// REGRESSION 2023-01-20 variable x-c in context wasp_main emitted as node data:
			is!("x=3;y=4;c=1;r=5;((‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);
		);
	}
	is!("i=-9;√-i", 3);
	is!("i=-9;√ -i", 3);
	//		is!("use math;√π²", 3);
}

#[test]
#[ignore]
fn test_smart_return_harder() {
	is!("'a'", 'a');
	//    is!("'a'", 'a'); // … should be 97
	//    is!("'a'", 'a');
	//    is!("'a'", 'a');
	is!("10007.0%10000.0", 7);
	is!("10007.0%10000", 7);
	#[cfg(not(feature = "WASM"))]
	{
		is!("x='abcde';x#4='f';x[3]", 'f');
		is!("x='abcde';x#4='x';x[3]", 'x');
		is!("x='abcde';x[3]", 'd');
	}
	//    is!("x='abcde';x[3]", (int) 'd');// currently FAILS … OK typesafe!
}
#[test]
#[ignore]
fn test_smart_return() {
	#[cfg(not(feature = "WASM"))]
	{
		test_smart_return_harder(); // todo
	}

	is!("1", 1);
	is!("-2000000000000", -2000000000000i64);
	is!("2000000000000", 2000000000000i64); // let int64
	is!("42.0/2.0", 21);
	is!("42.0/2.0", 21.);
	is!("- √9", -3);
	is!("42/4.", 10.5);
	skip!(

		is!("42/4", 10.5);
	);

	is!("42.0/2.0", 21);

	is!("-1.1", -1.1);
	is!("'OK'", "OK");
}
#[test]
fn test_multi_value() {
	#[cfg(feature = "MULTI_VALUE")]
	{
		is!("1,2,3", Node(1, 2, 3, 0));
		is!("1;2;3", 3);
		is!("'OK'", "OK");
	}
}

#[test]
#[ignore]
fn test_is() {
	// all these have been tested with is!
	// before. now assert! that it works with runtime
	//    test_wasm_runtime_extension();

	is!("42", 42);
	is!("x=123;x + 4 is 127", true); //  is! sometimes causes Heap corruption! test earlier
	is!("x='123';x is '123'", true); // ok
	is!("'hello';(1 2 3 4);10", 10); // -> data array […;…;10] ≠ 10
	#[cfg(not(feature = "TRACE"))]
	{
		is!("x='123';x + '4' is '1234'", true); // ok
		is!("'123' + '4' is '1234'", true); // ok needs runtime for concat();
		is!("x='123';x=='123'", true); // ok needs runtime for eq!();
	}
}
#[test]
fn test_logarithm() {
	skip!(

		is!("use log; log10(100)", 2.);
	);
}

#[test]
#[ignore]
fn test_logarithm2() {
	//	float ℯ = 2.7182818284590;

	// let function : Function = functions["log10"];
	// assert!(function.is_import);
	is!("use math; log10(100)", 2.);
	is!("use math; 10⌞100", 2.); // read 10'er Logarithm
	is!("use math; 100⌟10", 2.); // read 100 lowered by 10's
	is!("use math; 10⌟100", 2.);
	is!("use math; ℯ⌟", 2.);
	is!("use math; ℯ⌟", 2.);
	is!("log10(100)", 2.); // requires pre-parsing lib and dictionary lookup
	is!("₁₀⌟100", 2.); // requires pre-parsing lib and dynamic operator-list extension OR 10⌟ as function name
	is!("10⌟100", 2.); // requires pre-parsing lib and dynamic operator-list extension OR 10⌟ as function name

	//    eq!(ln(e),abs(1));
	is!("use log;ℯ = 2.7182818284590;ln(ℯ)", 1.);
	is!("use log;ℯ = 2.7182818284590;ln(ℯ)", 1.);
	is!("ℯ = 2.7182818284590;ln(ℯ*ℯ)", 2.);
	is!("ln(1)", 0.);
	is!("log10(100000)", 5.);
	is!("log10(10)", 1.);
	is!("log(1)", 0.);
	skip!(

		eq!(-ln(0), Infinity);
		eq!(ln(0), -Infinity);
		is!("ln(ℯ)", 1.);
	);
}

#[test]
#[ignore]
fn test_for_loop_classic() {
	is!("for(i=0;i<10;i++){puti i};i", 10);
	is!("sum = 0; for(i=0;i<10;i++){sum+=i};sum", 45);
}

#[test]
#[ignore]
fn test_for_loops() {
	#[cfg(not(feature = "WASM"))]
	{
		// todo: fix for wasm
		test_for_loop_classic();
	}
	// is!("for i in 1 to 5 : {print i};i", 6);
	// todo: generic dispatch print in WasmEdge
	#[cfg(feature = "WASM")]
	{
		// cheat!
		is!("for i in 1 to 5 : {print i};i", 6);
		is!("for i in 1 to 5 : {print i};i", 6); // EXC_BAD_ACCESS as of 2025-03-06 under SANITIZE
		is!("for i in 1 to 5 {print i}", 5);
		is!("for i in 1 to 5 {print i};i", 6); // after loop :(
		is!("for i in 1 to 5 : print i", 5);
		is!("for i in 1 to 5\n  print i", 5);
		// is!("for i in 1 to 5\n  print i\ni", 6);
	}
	#[cfg(not(feature = "WASM"))]
	{
		// # else // todo : why puti !in WASM??
		// is!("for i in 1 to 5 : {put(i)};i", 6);
		is!("for i in 1 to 5 : {puti(i)}", 5);
		is!("for i in 1 to 5 : {puti i};i", 6); // after loop :(
		is!("for i in 1 to 5 : puti i", 5);
		is!("for i in 1 to 5\n  puti i", 5); // unclosed pair  	<control>: SHIFT OUT
									   // is!("for i in 1 to 5\n  puti i\ni", 6);
		is!("for i in 1…5 : puti i", 5);
		is!("for i in 1 … 5 : puti i", 5);
		// is!("for i in 1 .. 5\n  puti i", 4);// exclusive!
		// is!("for i in 1 ..< 5\n  puti i", 4);// exclusive!
		is!("for i in 1 ... 5\n  puti i", 5);
	}
	skip!(

		is!("sum=0\nfor i in 1…3 {sum+=i}\nsum", 6); // todo range
		is!("sum=0\nfor i in 1 to 3 : sum+=i\nsum", 6); // todo range
		is!("sum=0\nfor i in (1 ... 3) {sum+=i}\nsum", 6); // todo range
		is!("sum=0\nfor i in (1..3) {sum+=i}\nsum", 6); // todo (1. 0.3) range
		is!("sum=0;for i in (1..3) {sum+=i};sum", 6);
		is!("sum=0;for i=1..3;sum+=i;sum", 6);
	);
}
//#[test] fn testDwarf();
//#[test] fn testSourceMap();
#[test]
#[ignore]
fn test_assert() {
	is!("assert 1", 1);
	assert_throws("assert 0"); // todo make wasm throw, !compile error?
}
// test once by looking at the output wasm/wat
#[test]
#[ignore]
fn test_named_data_sections() {
	is!("fest='def';test='abc'", "abc");
	exit(0);
}

#[test]
#[ignore]
fn test_auto_smarty() {
	is!("11", 11);
	is!("'c'", 'c');
	is!("'cc'", "cc");
	is!("π", PI);
	//    is!("{a:b}", new Node{.name="a"));
}

#[test]
#[ignore]
fn test_arguments() {
	is!("#params", 0); // no args, but create empty List anyway
	                // todo add context to wasp variable $params
}

#[test]
#[ignore]
fn test_host_download() {
	#[cfg(not(feature = "WASMEDGE"))]
	{
		is!("download https://pannous.com/files/test", "test 2 5 3 7");
	}
}
#[test]
#[ignore]
fn test_sinus2() {
	is!(
		r#"double sin(double x){
    x = modulo_double(x,tau);
    let z : double = x*x
    let w : double = z*z
    S1  = -1.66666666666666324348e-01,
    S2  =  8.33333333332248946124e-03,
    S3  = -1.98412698298579493134e-04,
    S4  =  2.75573137070700676789e-06,
    S5  = -2.50507602534068634195e-08,
    S6  =  1.58969099521155010221e-10
    if(x >= PI) return -sin(modulo_double(x,PI));
    let r : double = S2 + z*(S3 + z*S4) + z*w*(S5 + z*S6);
    return x + z*x*(S1 + z*r);
}; sin π/2"#,
		1
	); // IT WORKS!!!
}

#[test]
#[ignore]
fn test_sinus() {
	is!(
		r#"double sin(double x){
    x = modulo_double(x,tau)
    let z : tdouble = x*x
    let w : tdouble = z*z
    S1  = -1.66666666666666324348e-01, /* 0xBFC55555, 0x55555549 */
    S2  =  8.33333333332248946124e-03, /* 0x3F811111, 0x1110F8A6 */
    S3  = -1.98412698298579493134e-04, /* 0xBF2A01A0, 0x19C161D5 */
    S4  =  2.75573137070700676789e-06, /* 0x3EC71DE3, 0x57B1FE7D */
    S5  = -2.50507602534068634195e-08, /* 0xBE5AE5E6, 0x8A2B9CEB */
    S6  =  1.58969099521155010221e-10  /* 0x3DE5D93A, 0x5ACFD57C */
    //	            tau =  6.283185307179586 // 2π
    if(x >= PI) return -sin(modulo_double(x,PI))
    let r : tdouble = S2 + z*(S3 + z*S4) + z*w*(S5 + z*S6)
    return x + z*x*(S1 + z*r)
    "};sin π/2"#,
		1.0000000002522271
	); // IT WORKS!!! todo: why imprecision?
}

#[test]
#[ignore]
fn test_emit_basics() {
	is!("true", true);
	is!("false", false);
	is!("8.33333333332248946124e-03", 8.333_333_333_322_49e-3);
	is!("42", 42);
	is!("-42", -42);
	is!("3.3415", 3.3415);
	is!("-3.3415", -3.3415);
	is!("40", 40);
	is!("41", 41);
	is!("1 ∧ 0", 0);
	skip!(

		// see test_smart_return
		is!("'ok'", "ok"); // BREAKS wasm !!
		is!("'a'", "a");
		is!("'a'", 'a');
	);
}
#[test]
#[ignore]
fn test_math_extra() {
	is!("15÷5", 3);
	is!("15÷5", 3);
	is!("3⋅5", 15);
	is!("3×5", 15);
	skip!(

		is!("3**3", 27);
		is!("√3**2", 3);
		is!("3^3", 27);
		is!("√3^2", 3); // in test_squares
		eq!("one plus two times three", 7);
	);
}

#[test]
fn test_root() {
	skip!(

		eq!("40+√4", 42, 0);
		eq!("√4", 2);
		eq!("√4+40", 42);
		eq!("40 + √4", 42);
	); // todo tokenized as +√
}

#[test]
#[ignore]
fn test_root_float() {
	//	skip!(
	// include <cmath> causes problems, so skip
	is!("√42.0 * √42.0", 42.);
	is!("√42 * √42.0", 42.);
	is!("√42.0*√42", 42);
	is!("√42*√42", 42); // round AFTER! ok with f64! f32 result 41.99999 => 41
}
#[test]
#[ignore]
fn test_node_data_binary_reconstruction() {
	eq!(parse("y:{x:2 z:3}").serialize(), "y{x:2 z:3}"); // todo y:{} vs y{}
	is!("y:{x:2 z:3}", parse("y:{x:2 z:3}")); // looks trivial but is epitome of binary (de)serialization!
}
#[test]
#[ignore]
fn test_wasm_string() {
	#[cfg(feature = "WASM")]
	{
		return; // todo!
	}
	is!("“c”", 'c');
	is!("“a”", "a");
	is!("“b”", "b");
	is!("\"d\"", 'd');
	is!("'e'", 'e');
	#[cfg(feature = "WASM")]
	{
		is!("'f'", 'f');
		is!("'g'", 'g');
	}
	is!("'h'", "h");
	is!("\"i\"", "i");
	is!("'j'", Node::Text("j".into()));
	#[cfg(not(feature = "WASM"))]
	{
		// todo
		// let x : wasm_string = reinterpret_cast<wasm_string>("\03abc");
		// let y : String = String(x);
		// assert!(y == "abc");
		// assert!(y.length() == 3);
		is!("“hello1”", "hello1"); // Invalid typed array length: 12655
	}
}
#[test]
#[ignore]
fn test_fixed_in_browser() {
	test_math_operators_runtime(); // 3^2
	// test_string_indices(); // removed - function doesn't exist
	is!("(2+1)==(4-1)", true); // suddenly passes !? !with above line commented out BUG <<<
	is!("(3+1)==(5-1)", true);
	is!("(2+1)==(4-1)", true);
	is!("3==2+1", 1);
	is!("3 + √9", 6);
	is!("puti 3", 3);
	is!("puti 3", 3); //
	is!("puti 3+3", 6);
	// #[cfg(feature = "WASM")]{
	//     return;
	// }

	test_wasm_string(); // with length as header
	is!("x='abcde';x[3]", 'd');
	// testCall();
	test_array_indices_wasm();
	test_square_precedence();
}
//testWasmControlFlow

// #[test] fn testBadInWasm();

// SIMILAR AS:
#[test]
#[ignore]
fn test_todo_browser() {
	test_fixed_in_browser();
	test_old_random_bugs(); // currently ok

	skip!(
	// still breaking! (some for good reason);
		   // OPEN BUGS
		   testBadInWasm(); // NO, breaks!
	   );
}
// ⚠️ ALL tests containing is!
//  must go here! testCurrent() only for basics
#[test]
#[ignore] // NEVER TEST ALL again ;)
fn test_all_wasm() {
	// called by testRun() OR synchronously!
	is!("42", 42);
	is!("42+1", 43);
	// is!("test42+2", 44); // OK in WASM too ? deactivated for now
	test_sinus(); // still FRAGILE!

	test_todo_browser(); // TODO!
	skip!(

		is!("putf 3.1", 3);
		is!("putf 3.1", 3.1);
	);

	skip!(

		testWasmGC(); // WASM EDGE Error message: type mismatch
		testStruct(); // TODO get pointer of node on stack
		testStruct2();
	);
	#[cfg(feature = "WEBAPP")]
	{
		// or MY_WASM
		test_host_download();
	}
	// Test that IMPLICITLY use runtime /  is!
	// is!("x=(1 4 3);x#2", 4);
	// is!("n=3;2ⁿ", 8);
	// is!("k=(1,2,3);i=1;k#i=4;k#i", 4);

	is!("√9*-‖-3‖/-3", 3);
	skip!(
		is!("x=3;y=4;c=1;r=5;((‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);
		is!("i=3;k='αβγδε';k#i='Γ';k#i", 'Γ'); // todo setCharAt
		testGenerics();
	);
	test_implicit_multiplication(); // todo in parser how?
	test_for_loops();
	test_globals();
	// test_fibonacci();
	test_auto_smarty();
	test_arguments();
	skip!(

		testWasmGC();
		is!("τ≈6.2831853", true);
		eq!("τ≈6.2831853", true);
		is!("a = [1, 2, 3]; a[1] == a#1", false);
		is!("a = [1, 2, 3]; a[1] == a#1", 0);
	);
	//	data_mode = false;
	// test_wasm_memory_integrity();
	#[cfg(feature = "RUNTIME_ONLY")]
	{
		puts("RUNTIME_ONLY");
		puts("NO WASM emission...");
		//	return;
	}

	//	is! !compatible with Wasmer, don't ask why, we don't know;);
	//    skip!(

	//            test_custom_operators();
	//            test_wasm_mutable_global();
	//    );

	test_math_operators();
	test_wasm_logic_primitives();
	test_wasm_logic_unary();
	test_wasm_logic_unary_variables();
	test_wasm_logic();
	test_wasm_logic_negated();
	test_square();
	test_globals();

	test_comparison_id_precedence();
	test_wasm_stuff();
	test_float_operators();
	test_const_return();
	test_wasm_if();
	test_math_primitives();
	test_self_modifying();
	test_norm();
	test_comparison_primitives();
	test_comparison_math();
	test_comparison_id();
	test_wasm_ternary();
	test_square();
	test_round_floor_ceiling();
	test_wasm_ternary();
	test_wasm_function_calls();
	test_wasm_function_definiton();
	test_wasm_while();

	// the following need MERGE or RUNTIME! todo : split
	test_wasm_variables0();
	test_logarithm();
	test_merge_wabt_by_hand();
	test_merge_wabt();
	test_math_library();
	test_wasm_logic_combined();
	test_merge_wabt();

	//	exit(21);
	test_wasm_increment();
	// TRUE TESTS:
	test_recent_random_bugs();
	// test_old_random_bugs();
	is!("١٢٣", 123); //  numerals are left-to-right (LTR) even in Arabic!

	skip!(

		testMergeOwn();
		testMergeRelocate();
	);
	test_get_local();
	skip!(
	// new stuff :
		   testObjectPropertiesWasm();
		   testWasmLogicOnObjects();
		   testCustomOperators();
	   );
}

#[test]
#[ignore]
fn test_get_element_by_id() {
	let _result = analyze(parse("$result"));
	// eq!(result.kind, externref);
	let _nod = eval("$result");
	// print(nod);
}

#[test]
#[ignore]
fn test_canvas() {
	let _result = analyze(parse("$canvas"));
	// eq!(result.kind(), externref);
	let _nod = eval(
		r#"    ctx = $canvas.getContext('2d');
                       ctx.fillStyle = 'red';
                       ctx.fillRect(10, 10, 150, 100);"#,
	);
	// print(nod);
}

// run in APP (or browser?);
#[test]
fn test_dom() {
	print("test_dom");
	// preRegisterFunctions();
	let mut _result = analyze(parse("getElementById('canvas')"));
	// eq!(result.kind, call);
	_result = eval("getElementById('canvas');");
	//	print(typeName(result.kind));
	//	eq!(result.kind, strings); // why?
	//	eq!(result.kind, longs); // todo: can't use smart pointers for elusive externref
	//	eq!(result.kind, bools); // todo: can't use smart pointers for elusive externref
	// print(typeName(30));
	// print(typeName(9));
	//	eq!(result.kind, 30);//
	//	eq!(result.kind,9);//
	//	eq!(result.kind,  externref); // todo: can't use smart pointers for elusive externref
	//	result = eval("document.getElementById('canvas');");
	//	result = analyze(parse("$canvas"));
	//	eq!(result.kind,  externref);
}

#[test]
#[ignore]
fn test_dom_property() {
	// #[cfg(not(feature = "WEBAPP"))]{
	//     return;
	// }
	let mut result = eval("getExternRefPropertyValue($canvas,'width')"); // ok!!
	eq!(result.value(), &300); // only works because String "300" gets converted to BigInt 300
							//	result = eval("width='width';$canvas.width");
	result = eval("$canvas.width");
	eq!(result.value(), &300);
	//	return;
	result = eval("$canvas.style");
	eq!(result.kind(), NodeKind::Text);
	//	eq!(result.kind, stringp);
	// if (result.value().string);
	// is!(*result.value().string, "dfsa");
	//	getExternRefPropertyValue OK  [object HTMLCanvasElement] style [object CSSStyleDeclaration]
	// ⚠️ But can't forward result as smarti or stringref:  SyntaxError: Failed to parse String to BigInt
	// todo : how to communicate new string as RETURN type of arbitrary function from js to wasp?
	// call Webview.getString(); ?

	//	embedder.trace('canvas = document.getElementById("canvas");');
	//	print(nod);
}

#[test]
#[ignore]
fn test_host_integration() {
	#[cfg(feature = "WASMTIME")]
	{
		//         WASMEDGE
		return;
	}
	#[cfg(not(feature = "WASM"))]
	{
		test_host_download(); // no is!
	}
	// test_get_element_by_id();
	// test_dom();
	// test_dom_property();
	// testInnerHtml();
	// testJS();
	// testFetch();
	skip!(
		testCanvas(); // attribute setter missing value breaks browser
	);
}
