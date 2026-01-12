// Function tests
// Migrated from tests_*.rs files

use warp::analyzer::{analyze, collect_functions};
use warp::type_kinds::Kind;
use warp::wasp_parser::parse;
use warp::Node;
use warp::{eq, is};

#[test]
fn test2def() {
	is!("def test1(x){x+1};def test2(x){x+1};test2(3)", 4);
	// Note: test2(3) = 3+1 = 4, not 6 (test1 is not called)
	is!("def test1(x){x+3};def test2(x){x+1};test2(3)", 4);
}

#[test]
fn test_function_declaration() {
	// Working syntaxes: def name(params): body  OR  name(params) = body
	is!("def x(): 42; x()+1", 43);
	is!("def x(a): 42+a; x(1)+1", 44);
	is!("x() = 42; x()+1", 43);
	is!("x(a) = 42+a; x(1)+1", 44);

	// Obscure/unsupported syntaxes - not wasp style:
	// is!("fun x{42} x+1", 43);           // fun keyword, space-call
	// is!("def x{42};x+1", 43);           // braces without colon
	// is!("def x(){42};x+1", 43);         // braces without colon
	// is!("define x={42};x()+1", 43);     // define keyword
	// is!("function x(){42};x()+1", 43);  // function keyword
	// is!("function x(a){42+a};x(1)+1", 44);
	// is!("define x={42+it};x(1)+1", 44);
	// is!("def x(a=3){42+a};x+1", 46);    // default params not yet supported
	// is!("def x(a){42+a};x+1", 43);      // implicit call not yet supported
}

#[test]
fn test_function_declaration_parse() {
	let node2 = analyze(parse("fun test(a:float){return a*2}"));
	eq!(node2.name(), "test");

	let functions = collect_functions(&node2);
	eq!(functions["test"].signature.len(), 1);
	eq!(functions["test"].signature.parameters[0].name, "a");
	eq!(functions["test"].signature.parameters[0].kind, Kind::Float);
	// TODO: once body comparison is implemented
	// eq!(*functions["test"].body.as_ref().unwrap(), analyze(parse("return a*2")));
}

#[test]
fn test_rename_wasm_function() {
	// let module1 = loadModule("samples/test.wasm");
	// module1.functions.at(0).name = "test";
	// module1.save("samples/test2.wasm");
	// todo: assert! by loadModule("samples/test2.wasm");
}

#[test]
#[ignore]
fn test_wit_function() {
	//    funcDeclaration
	// a:b,c vs a:b, c:d
	is!("add: func(a: float32, b: float32) -> float32", 0);
	// let modu : Module = read_wasm("test.wasm");
	// print( modu.import_count);
	// eq!(modu.import_count, 1);
	// eq!(Node().setKind(longs).serialize(), "0");
	// eq!(mod.import_names, List<String>{"add"}); // or export names?
}

// fn read_wasm(p0: &str) -> Module {
//     todo!()
// }

#[test]
fn test_float_return_through_main() {
	//     double
	//     let x = 0.0000001; // 3e...
	//	double x=1000000000.1;// 4...
	//	double x=-1000000000.1;// c1…
	//	double x=9999999999999999.99999999;// 43…
	//	double x=-9999999999999999.99999999;// c3…
	//	double x=1.1;// 3ff199999999999a
	//	double x=-1.1;// bff199999999999a
	//     int64
	//     y = *(int64 *) & x;
	let y: i64 = 0x00FF000000000000; // -> 0.000000 OK
								  // #[cfg(not(feature = "WASM"))]{
								  // printf!("%llx\n", y);
								  // }
								  // x = *(double *) & y;
								  // printf!("%lf\n", x);
	is!(y.to_string().as_str(), 0x00FF000000000000i64);
}

#[test]
// #[ignore]
fn test_graph_params() {
	let result = parse("{\n  empireHero: hero(episode: EMPIRE){\n    name\n  }\n  jediHero: hero(episode: JEDI){\n    name\n  }\n}");
	// let hero : Node = result["empireHero"].clone();
	let hero: &Node = &result["empireHero"];
	hero.print();
	eq!(hero["episode"], "EMPIRE");
}


#[test]
// #[ignore]
fn test_params() {
	let body = parse("body(style='blue'){a(link)}");
	eq!(body["style"], "blue");
	//	let result = parse("a(href='#'){'a link'}");
	//	let result = parse("(markdown link)[www]");
}

#[test]
#[ignore]
fn test_stacked_lambdas() {
	// currently  a:{x:1}  {y:2}  {3}
	let result = parse("a{x:1}{y:2}{3}");
	result.print();
	eq!(result.length(), 3);
	eq!(result[0], parse("{x:1}"));
	eq!(result[0], parse("x:1")); // grouping irrelevant
	eq!(result[1], parse("{y:2}"));
	eq!(result[2], parse("{3}"));
	assert_ne!(result[2], parse("{4}"));

	assert_ne!(parse("a{x}{y z}"), parse("a{x,{y z}}"));
}

#[test]
#[ignore]
fn test_modifiers() {
	is!("public fun ignore(){3}", 3);
	is!("public static export import extern external C global inline virtual override final abstract private protected internal const constexpr volatile mutable thread_local synchronized transient native fun ignore(){3}",3);
}

#[test]
fn test_fibonacci_auto_typed() {
	// TODO: use newline once parser precedence is fixed for := vs newline
	is!("fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);
}

#[test]
fn test_fibonacci_auto_param() {
	// TODO: use newline once parser precedence is fixed for := vs newline
	is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2); fib(10)",55);
}

#[test]
fn test_fibonacci_typed() {
	// TODO: use newline once parser precedence is fixed for := vs newline
	is!("fib(n:int) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)",55);
	is!("fib(n:number) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)",55);
}

#[test]
fn test_fibonacci_typed2() {
	// Working syntaxes (alread in test_fibonacci_typed etc):
	// is!("fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);
	// is!("fib(n:int) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);


	// C-like syntaxes - not wasp style, use n:type instead of type n:
	// is!("int fib(int n){n < 2 ? n : fib(n - 1) + fib(n - 2)}; fib(10)", 55);
	// is!("fib(int n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);
	// is!("fib(number n) = n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);

	// Braces body and := with params - not yet supported:
	// is!("fib(n){n < 2 ? n : fib(n - 1) + fib(n - 2)}; fib(10)", 55);
	// is!("fib(n) := n < 2 ? n : fib(n - 1) + fib(n - 2); fib(10)", 55);

	// Implicit param with = - not yet supported:
	// is!("fib = it < 2 ? 1 : fib(it - 1) + fib(it - 2); fib(10)", 55);

	// Space-separated param - obscure, not wasp style:
	// is!("fib number := if number<2 : 1 else fib(number - 1) + fib it - 2; fib(9)", 55);
}

// From test_new.rs
#[test]
fn test_function_definitions() {
	is!("def add(a,b): a+b; add(2,3)", 5);
	is!("def square(x): x*x; square(4)", 16);
}

#[test]
fn test_variables() {
	// Basic integers
	is!("x=42; x", 42);
	is!("y=3; y", 3);
	is!("z=0; z", 0);
	is!("n=-5; n", -5);
	is!("big=1000000; big", 1000000);

	// Floats
	is!("f=3.14; f", 3.14);
	is!("g=0.0; g", 0.0);
	is!("h=-2.5; h", -2.5);
	is!("tiny=0.001; tiny", 0.001);

	// Variable operations
	is!("a=10; a+5", 15);
	is!("b=20; b-8", 12);
	is!("c=7; c*3", 21);
	is!("d=15; d/3", 5);

	// Multiple variables
	is!("x=1; y=2; x+y", 3);
	is!("a=10; b=20; c=30; a+b+c", 60);
	is!("p=5; q=3; p*q", 15);

	// Using define operator
	is!("x:=42; x", 42);
	is!("y:=3.14; y", 3.14);

	// Expressions as values
	is!("x=2+3; x", 5);
	is!("y=10*2; y", 20);
	is!("z=100/4; z", 25);

	// Chained operations with variables
	is!("a=2; b=a*3; b", 6);
	is!("x=5; y=x+1; z=y*2; z", 12);

	// Edge cases
	is!("zero=0; zero", 0);
	is!("one=1; one", 1);
	is!("neg=-1; neg", -1);
}

#[test]
fn test_variable_reassignment(){
	is!("x=1; x=2; x", 2);
	is!("v=10; v=v+1; v", 11);
}
