// Type system tests
// Migrated from tests_*.rs files

use crate::{is, skip};
use warp::extensions::assert_throws;
use warp::Node;
use warp::Number;

#[test]
fn test_go_types() {
	is!("func add1(x int) int { return x + 1 };add1(41)", 42);
}

#[test]
fn test_auto_type() {
	// todo redundant test_division_type_upgrade
	is!("-1/6.", -1.0 / 6.0);
	is!("-1/6", -1.0 / 6.0); // Auto-promote int/int division to float
	is!("-1÷6", -1.0 / 6.0); // Auto-promote int/int division to float
}

#[test]
fn test_auto_type_nan() {
	// Float division by zero produces special float values (semantically equal to Nan/Inf); P66: an Int division by
	// zero is the error divide_by_zero, so the divisions here are written with a decimal point
	is!("0.0/0.0", Node::Number(Number::Nan));
	is!("0.0÷0.0", Node::Number(Number::Nan));
	is!("1.0/0.0", Node::Number(Number::Inf));
	is!("-1.0/0.0", Node::Number(Number::NegInf));
}

#[test]
fn test_type_synonyms() {
	// eq!(Type("i32"s),Type("int32"s));
	// eq!(Type("i32"s),Type("int"s));
	// eq!(Type("f32"s),Type("float32"s));
	// eq!(Type("f32"s),Type("float"s));
}

#[test]
#[ignore]
fn test_return_types() {
	is!("fun addier(a,b){b+a};addier(42,1)", 43);
	is!("fun addier(a,b){b+a};addier(42,1)+1", 44);
	is!("fun addi(x,y){x+y};addi(2.2,2.2)", 4.4);
	is!("float addi(x,y){x+y};addi(2.2,2.2)", 4.4);
	is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
	is!("add1 x:=x+1;add1 3", 4);
	is!("int x = $bla", 123);
	is!("`${1+1}`", "2");
	is!("real x = $bla", 123.);
	skip!(

		is!("k=(1,2,3);i=1;k#i=4;k#1", 4) // fails linking _ZdlPvm operator delete(void*, unsigned long);
		is!("i=1;k='hi';k#i", 'h'); // BUT IT WORKS BEFORE!?! be careful with i64 smarty return!
	);

	//==============================================================================
	// STRING TESTS (see string_tests.h);
	//==============================================================================
}


#[test]
fn test_cast_as() {
	is!("123.1 as int", 123);
	is!("123 as string", "123");
	is!("'2.5' as float", 2.5);
}

#[test]
fn test_emit_cast() {
	// Simple casts first
	is!("2 as string", "2");
	is!("'2' as int", 2);
	is!("'2' as long", 2);
	is!("'2.1' as int", 2);
	is!("'2.1' as long", 2);
	is!("'2.1' as real", 2.1);
	is!("'2.1' as float", 2.1);
	is!("'2.1' as double", 2.1);
	// Char and number types
	is!("2 as char", '2'); // ≠ char(0x41) ==  'a'
	is!("'2' as number", 2);
	is!("'2.1' as number", 2.1);
	// Bool casts
	is!("'2' as bool", true);
	is!("2 as bool", true);
	is!("'false' as bool", false);
	is!("'no' as bool", false);
	is!("'ø' as bool", false);
}

#[test]
fn test_emit_cast_tuple() {
	// P227: a comma next to == needs parentheses (tests/operators/test_comma_next_to_comparison.rs)
	is!("(2 as float, 4.3 as int)  == (2.0 ,4)", 1);
	is!("(2 as float, 4.3 as int)  == (2,4)", 1);
}

#[test]
fn test_constructor_cast() {
	// educate user to use as ?
	is!("int('123')", 123);
	is!("str(123)", "123");
	is!("char(0x41)", 'A'); // 0x41 = 65 = 'A'
	is!("string(123)", "123");
	is!("String(123)", "123");
}

#[test]
fn test_bad_type() {
	skip!(

		// TODO strict mode a:b=c => b is type vs data mode a:b => b is data HOW?
		assert_throws("x:yz=1"); // "yz" is not a type
	);
}

#[test]
// #[ignore]
fn test_type_confusion() {
	assert_throws("x=1;x='ok'");
	assert_throws("x=1;x=1.0");
	// assert_throws("number:=it*2"); // number is a type! can't use type as variable name YES we can, see
	// assert_throws("double:=it*2"); // double is type i64! can't use type as variable name
	// todo: get rid of stupid type name double, in C it's float64 OR int64 anyway
}

#[test]
fn test_function_return_types() {
	is!("def f(x):float := 42.0;f(0)", 42.0);
	is!("def f(x):int := 42;f(0)", 42);
	is!("def f(x):float := 42;f(0)", 42.0);
	is!("def f(x) as float := 42.0;f(0)", 42.0);
	is!("def f(x) as int := 42;f(0)", 42);
	is!("def f(x) as float := 42;f(0)", 42.0); // auto-cast
	is!("def f(x) as string := 42;f(0)", "42"); // auto-cast
}

#[test]
fn test_polymorphism3() {
	is!("fun test(string a){return a};\nfun test(float b){return b+1};\ntest('ok')", "ok");
	is!("fun test(string a){return a};\nfun test(int a){return a};\nfun test(float b){return b+1};\ntest(1.0)", 2.0);
}

// a generic type: a list of int16, written [int16] or list<int16> (card vacuous-tests; checking its elements: card
// typed-list-elements)
#[test]
fn test_generics() {
	is!("xs: [int16] = [1, 2, 3]; xs#2", 2);
	is!("xs: list<int16> = [1, 2]; xs#2 + 1", 3);
	is!("xs: [int16] = [1, 2]; type(xs)", "list of int16");
}

#[test]
// P49 (user, 2026-10-05): a float passed to an int parameter is a compile error, never truncated
fn test_function_argument_cast() {
	is!("float addi(int x,int y){x+y};'hello'+5", "hello5");
	is!("float addi(int x,int y){x+y};'hello'+5.9", "hello5.9");
	crate::common::fails_with("float addi(int x,int y){x+y};'hello'+addi(2.2,2.2)", "2.2 is no int: write 2.2 as int");
	//     is!("float addi(int x,int y){x+y};'hello'+addi(2,3)", "hello5.") // OK some float cast going on!

	is!("fun addier(a,b){b+a};addier(42.0,1.0)", 43);
	is!("fun addier(int a,int b){b+a};addier(42,1)+1", 44);
	crate::common::fails_with("fun addi(int x,int y){x+y};addi(2.2,2.2)", "2.2 is no int");
	is!("fun addi(float x,float y){x+y};addi(2.2,2.2)", 4.4);
	crate::common::fails_with("float addi(int x,int y){x+y};addi(2.2,2.2)", "2.2 is no int");
	is!("fun addier(float a,float b){b+a};addier(42,1)+1", 44);
}
