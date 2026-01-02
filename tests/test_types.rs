// Type system tests
// Migrated from tests_*.rs files

use std::iter::Map;
use syn::Signature;
use wasm_ast::Function;
use wasp::analyzer::analyze;
use wasp::extensions::assert_throws;
use wasp::node::Node;
use wasp::node::Node::False;
use wasp::type_kinds::NodeKind;
use wasp::wasp_parser::parse;
use wasp::{eq, is, skip, Number};

// const functions : Map<String, Function> = wasp::analyzer::FUNCTIONS;

// TODO: Stub types - these need proper implementation
#[allow(non_camel_case_types)]
type Type = Node;
#[allow(dead_code)]
struct AST;
#[allow(dead_code)]
struct Generics {
    kind: Node,
    value_type: Node,
}
#[allow(dead_code)]
fn clearAnalyzerContext() {} // stub
#[allow(dead_code)]
fn ByteCharType() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn IntegerType() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn StringType() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn DoubleType() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn int16t() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn int32t() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn float32t() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn stringp() -> Node {
    Node::Empty
}
#[allow(dead_code)]
fn array() -> Node {
    Node::Empty
}

#[test]
fn testGoTypes() {
    is!("func add1(x int) int { return x + 1 };add1(41)", 42);
}

#[test]
fn testAutoType() {
    is!("0/0", False);
    is!("0÷0", Node::Number(Number::Nan));
    is!("-1/6.", -1.0 / 6.0);
    is!("-1/6", -1.0 / 6.0); // Auto-promote int/int division to float
    is!("-1÷6", -1.0 / 6.0); // Auto-promote int/int division to float
}

#[test]
fn testTypeSynonyms() {
    // eq!(Type("i32"s),Type("int32"s));
    // eq!(Type("i32"s),Type("int"s));
    // eq!(Type("f32"s),Type("float32"s));
    // eq!(Type("f32"s),Type("float"s));
}

#[test]
fn testReturnTypes() {
    is!("fun addier(a,b){b+a};addier(42,1)", 43);
    is!("fun addier(a,b){b+a};addier(42,1)+1", 44);
    is!("fun addi(x,y){x+y};addi(2.2,2.2)", 4.4);
    is!("float addi(x,y){x+y};addi(2.2,2.2)", 4.4);
    is!(
        "fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)",
        55
    );
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

// fn cast(node: Node, to_type: &Type) -> Node {
fn cast(node: Node, to_type: NodeKind) -> Node {
    // stub
    // in real code this would do actual casting
    match to_type {
        NodeKind::Text => Node::Text(node.to_string()),
        NodeKind::Number => match node {
            Node::Number(n) => node,
            Node::Symbol(s) => match s.parse::<i64>() {
                Ok(v) => Node::Number(Number::Int(v)),
                _ => match s.parse::<f64>() {
                    Ok(f) => Node::Number(Number::Float(f)),
                    _ => Node::Number(Number::Nan),
                },
            },
            _ => Node::Number(Number::Nan),
        },
        _ => todo!("cast not implemented for type {:?}", to_type),
    }
}

#[test]
fn testCast() {
    // is!("2", cast(Node(2),  NodeKind::Text).value.string);
    // eq!(cast(Node(2), longs), 2); // trivial
    // eq!(cast(Node(2.1), longs), 2);
    // eq!(cast(Node(2), reals).value.real, 2.0);
    // eq!(cast(Node(2.1), reals).value.real, 2.1);
    // is!("2.1", cast(Node(2.1), NodeKind::Text).value.string);
    // is!("a", cast(Node('a'), NodeKind::Text).value.string);
    // no need to cast!
    // eq!(false, cast(Node('0'), bools));
    // eq!(false, cast(Node('ø'), bools));
    // eq!(false, cast(Node("False", false), bools));
    // eq!(false, cast(Node("ø", false), bools));
    // eq!(true, cast(Node("True", false), bools));
    // eq!(true, cast(Node("1", false), bools));
    // eq!(true, cast(Node(1), bools));
    // eq!(true, cast(Node("abcd", false), bools));
}

#[test]
fn testEmitCast() {
    is!("(2 as float, 4.3 as int)  == 2.0 ,4", 1);
    is!("(2 as float, 4.3 as int)  == 2,4", 1);
    // advanced, needs cast() to be implemented in wasm
    is!("2 as char", '2'); // ≠ char(0x41) ==  'a'
    is!("2 as string", "2");
    is!("'2' as number", 2);
    is!("'2.1' as number", 2.1);
    is!("'2' as bool", true);
    is!("2 as bool", true);
    is!("'false' as bool", false);
    is!("'no' as bool", false);
    is!("'ø' as bool", false);
    is!("'2' as int", 2);
    is!("'2' as long", 2);
    is!("'2.1' as int", 2);
    is!("'2.1' as long", 2);
    is!("'2.1' as real", 2.1);
    is!("'2.1' as float", 2.1);
    is!("'2.1' as double", 2.1);
}

#[test]
fn testConstructorCast() {
    is!("int('123')", 123);
    is!("str(123)", "123");
    is!("'a'", 'a');
    is!("char(0x41)", 'a');
    is!("string(123)", "123");
    is!("String(123)", "123");
}

#[test]
fn testBadType() {
    skip!(

        // TODO strict mode a:b=c => b is type vs data mode a:b => b is data HOW?
        assert_throws("x:yz=1"); // "yz" is not a type
    );
}

#[test]
fn testDeepType() {
    parse("a=$canvas.tagName");
    //    // eq!(result.kind(), smarti64);
    //    // eq!(result.kind(), AST::NodeKind::Text);
}

#[test]
fn testTypeConfusion() {
    assert_throws("x=1;x='ok'");
    assert_throws("x=1;x=1.0");
    assert_throws("double:=it*2"); // double is type i64!
                                   // todo: get rid of stupid type name double, in C it's float64 OR int64 anyway
}

#[test]
fn testTypesSimple() {
    // clearAnalyzerContext();
    let result = analyze(parse("chars a"));
    // eq!(result.kind(), Type::reference);
    // // eq!(result.typo, &ByteCharType); // todo char ≠ char* !
    // // eq!(result.name, "a");
    let _result = analyze(parse("int a"));
    // eq!(result.kind(), AST::reference);
    // // eq!(result.typo, &IntegerType); // IntegerType
    // // eq!(result.name, "a");

    let _result = analyze(parse("string b"));
    // eq!(result.kind(), AST::reference);
    // // eq!(result.typo, &StringType);
    // // eq!(result.name, "b");

    let _result = analyze(parse("float a,string b"));
    // let result0 = result[0];
    // eq!(result0.kind(), AST::reference);
    //	eq!(result0.kind(), AST::declaration);
    //	todo at this stage it should be a declaration?

    // eq!(result0.typo, &DoubleType);
    // eq!(result0.name, "a");
    // let result1 = result[1];
    // eq!(result1.kind(), AST::reference);
    // eq!(result1.typo, &StringType);
    // eq!(result1.name, "b");
}

#[test]
#[ignore] // TODO: requires AST and Type implementation
fn testTypesSimple2() {
    let _result = analyze(parse("a:chars"));
    //    // eq!(result.kind(), AST::reference);
    // eq!(result.kind(), AST::key);
    // // eq!(result.typo, &ByteCharType);
    // // eq!(result.name, "a");
    let _result = analyze(parse("a:int"));
    // eq!(result.kind(), AST::reference);
    // // eq!(result.typo, &IntegerType); // IntegerType
    // // eq!(result.name, "a");

    let _result = analyze(parse("b:string"));
    // eq!(result.kind(), AST::reference);
    // // eq!(result.typo, &StringType);
    // // eq!(result.name, "b");

    let result = analyze(parse("a:float,b:string"));
    // let result0 = result[0];
    // eq!(result0.kind(), AST::reference);
    //	eq!(result0.kind(), AST::declaration);
    //	todo at this stage it should be a declaration?
    // eq!(result0.typo, &DoubleType);
    // eq!(result0.name, "a");
    // let result1 = result[1];
    // eq!(result1.kind(), AST::reference);
    // eq!(result1.typo, &StringType);
    // eq!(result1.name, "b");
}

#[test]
#[ignore] // TODO: requires complete type system and Signature implementation
fn testTypedFunctions() {
    // todo name 'id' clashes with 'id' in preRegisterFunctions();
    clearAnalyzerContext();
    let _result = analyze(parse("int tee(float b, string c){b}"));
    // eq!(result.kind(), AST::declaration);
    // // eq!(result.name, "tee");
    // let signature_node = result["@signature"];
    //	let signature_node = result.metas()["signature"];
    // if (not signature_node.data_value());
    // error("no signature");
    // let signature : Signature = signature_node.data_value();
    // eq!(signature.functions.first(), "tee");
    // eq!(signature.parameters.size(), 2);
    // eq!(signature.parameters.first().name, "b");
    // eq!(signature.parameters.first().typo, reals); // use real / number for float64  float32
    // eq!(signature.parameters.last().name, "c");
    // eq!(signature.parameters.last().typo, NodeKind::Text);
    // let params = signature.parameters.map(+[](Arg f) { return f.name; });
    // eq!(params.first(), "b");
}

#[test]
#[ignore] // TODO: requires complete type system
fn testEmptyTypedFunctions() {
    // todo int a(){} should be compiler error
    // todo do we really want / need int a(); #[test] fn a(){} ?
    //	if(ch=='{' and next=='}' and previous==')'){
    //		actual.setType(declaration, false);// a(){} => def a!
    //		proceed();
    //		proceed();
    //		break;
    //	}
    let result = analyze(parse("int a(){}"));
    // eq!(result.kind(), AST::declaration);
    // // eq!(result.name, "a");
    // let signature_node = result["@signature"];
    // let signature : Signature = signature_node.data_value().downcast_ref::<Signature>().unwrap().clone();
    // eq!(signature.functions.first(), "a");
    // let names2 = signature.functions.map < String > ( + [](Function * f)
    // { return f; ; });
    // eq!(names2.size(), 1);
    // eq!(names2.first(), "a");

    let _result = analyze(parse("int a();"));
    // eq!(result.kind(), AST::declaration); // header signature
    // // eq!(result.typo, IntegerType);
    // // eq!(result.name, "a");
}

#[test]
#[ignore] // TODO: requires complete type system
fn testTypes() {
    testBadType();
    testDeepType();
    testTypedFunctions();
    testTypesSimple();
    testTypeConfusion();
    skip!(
        testTypesSimple2();
        testEmptyTypedFunctions();
    );
}

#[test]
#[ignore] // TODO: requires complete type system
fn testPolymorphism() {
    // debug:
    //	let debug_node = parse("string aaa(string a){return a};\nfloat bbb(float b){return b+1}");
    //	let debug_fun = analyze(debug_node);
    let node = parse("string test(string a){return a};\nfloat test(float b){return b+1}");
    let fun = analyze(node);
    // let function = functions["test"];
    // eq!(function.is_polymorphic, true);
    // eq!(function.variants.size(), 2);
    // eq!(function.variants[0].signature.size(), 1);
    //	eq!(function.variants[0].signature.parameters[0].typo, (Type) NodeKind::Text); todo
    // eq!(function.variants[0].signature.parameters[0].typo, stringp);
    // let variant = function.variants[1];
    // eq!(variant.signature.size(), 1);
    // eq!(variant.signature.parameters[0].typo, float32t);
}

#[test]
#[ignore] // TODO: requires complete type system
fn testPolymorphism2() {
    clearAnalyzerContext();
    let node = parse("fun test(string a){return a};\nfun test(float b){return b+1}");
    let fun = analyze(node);
    // let function = functions["test"];
    // eq!(function.is_polymorphic, true);
    // eq!(function.variants.size(), 2);
    // eq!(function.variants[0].signature.size(), 1);
    // eq!(function.variants[0].signature.parameters[0].typo, int32t);
    // eq!(function.variants[1].signature.size(), 1);
    // eq!(function.variants[1].signature.parameters[0].typo, float32t);
}

#[test]
#[ignore] // TODO: requires complete type system
fn testPolymorphism3() {
    is!(
        "fun test(string a){return a};\nfun test(float b){return b+1};\ntest('ok')",
        "ok"
    );
    is!("fun test(string a){return a};\nfun test(int a){return a};\nfun test(float b){return b+1};\ntest(1.0)",2.0);
}

#[test]
#[ignore] // TODO: requires Generics implementation
fn testGenerics() {
    // let typ = Type(Generics { kind: array, value_type: int16t });
    //    let header= typ.let array : value;
    //    let header= typ.let 0xFFFF0000 : value; // todo <<
    // let header = typ.let 0x0000FFFF : value; //todo ?? - invalid Rust syntax
    //     assert!(_eq!(header, array);
}

#[test]
#[ignore] // TODO: requires complete type system
fn testFunctionArgumentCast() {
    is!("float addi(int x,int y){x+y};'hello'+5", "hello5");
    is!("float addi(int x,int y){x+y};'hello'+5.9", "hello5.9");
    is!(
        "float addi(int x,int y){x+y};'hello'+addi(2.2,2.2)",
        "hello4."
    );
    //     is!("float addi(int x,int y){x+y};'hello'+addi(2,3)", "hello5.") // OK some float cast going on!

    is!("fun addier(a,b){b+a};addier(42.0,1.0)", 43);
    is!("fun addier(int a,int b){b+a};addier(42,1)+1", 44);
    is!("fun addi(int x,int y){x+y};addi(2.2,2.2)", 4);
    is!("fun addi(float x,float y){x+y};addi(2.2,2.2)", 4.4);
    is!("float addi(int x,int y){x+y};addi(2.2,2.2)", 4.4);
    is!("fun addier(float a,float b){b+a};addier(42,1)+1", 44);
}
