
// Type system tests
// Migrated from tests_*.rs files

#[test]
fn testGoTypes() {
    is!("func add1(x int) int { return x + 1 };add1(41)", 42);
}

#[test]
fn testAutoType() {
    is!("0/0", Nan);
    is!("0÷0", Nan);
    is!("-1/6.", -1/6.);
    is!("-1/6", -1/6.); // Auto-promote int/int division to float
    is!("-1÷6", -1/6.); // Auto-promote int/int division to float
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
    is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
    is!("add1 x:=x+1;add1 3", (int64) 4);
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
fn testCast() {
    assert! _eq("2"s, cast(Node(2), strings).value.string);
    assert! _eq(cast(Node(2), longs).value.longy, 2); // trivial
    assert! _eq(cast(Node(2.1), longs).value.longy, 2);
    assert! _eq(cast(Node(2), reals).value.real, 2.0);
    assert! _eq(cast(Node(2.1), reals).value.real, 2.1);
    assert! _eq("2.1"s, cast(Node(2.1), strings).value.string);
    assert! _eq("a"s, cast(Node('a'), strings).value.string);
    assert! _eq(false, cast(Node('0'), bools).value.longy);
    assert! _eq(false, cast(Node(u'ø'), bools).value.longy);
    assert! _eq(false, cast(Node("False"s, false), bools).value.longy);
    assert! _eq(false, cast(Node("ø"s, false), bools).value.longy);
    assert! _eq(true, cast(Node("True"s, false), bools).value.longy);
    assert! _eq(true, cast(Node("1"s, false), bools).value.longy);
    assert! _eq(true, cast(Node(1), bools).value.longy);
    assert! _eq(true, cast(Node("abcd"s, false), bools).value.longy);
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
    //    eq!(result.kind, smarti64);
    //    eq!(result.kind, Kind::strings);
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
    clearAnalyzerContext();
    result = analyze(parse("chars a"));
    eq!(result.kind, Kind::reference);
    eq!(result.type, &ByteCharType); // todo char ≠ char* !
    eq!(result.name, "a");
    result = analyze(parse("int a"));
    eq!(result.kind, Kind::reference);
    eq!(result.type, &IntegerType); // IntegerType
    eq!(result.name, "a");

    result = analyze(parse("string b"));
    eq!(result.kind, Kind::reference);
    eq!(result.type, &StringType);
    eq!(result.name, "b");

    result = analyze(parse("float a,string b"));
    let result0 = result[0];
    eq!(result0.kind, Kind::reference);
    //	eq!(result0.kind, Kind::declaration);
    //	todo at this stage it should be a declaration?

    eq!(result0.type, &DoubleType);
    eq!(result0.name, "a");
    let result1 = result[1];
    eq!(result1.kind, Kind::reference);
    eq!(result1.type, &StringType);
    eq!(result1.name, "b");
}

#[test]
fn testTypesSimple2() {
    result = analyze(parse("a:chars"));
    //    eq!(result.kind, Kind::reference);
    eq!(result.kind, Kind::key);
    eq!(result.type, &ByteCharType);
    eq!(result.name, "a");
    result = analyze(parse("a:int"));
    eq!(result.kind, Kind::reference);
    eq!(result.type, &IntegerType); // IntegerType
    eq!(result.name, "a");

    result = analyze(parse("b:string"));
    eq!(result.kind, Kind::reference);
    eq!(result.type, &StringType);
    eq!(result.name, "b");

    result = analyze(parse("a:float,b:string"));
    let result0 = result[0];
    eq!(result0.kind, Kind::reference);
    //	eq!(result0.kind, Kind::declaration);
    //	todo at this stage it should be a declaration?
    eq!(result0.type, &DoubleType);
    eq!(result0.name, "a");
    let result1 = result[1];
    eq!(result1.kind, Kind::reference);
    eq!(result1.type, &StringType);
    eq!(result1.name, "b");
}

#[test]
fn testTypedFunctions() {
    // todo name 'id' clashes with 'id' in preRegisterFunctions();
    clearAnalyzerContext();
    result = analyze(parse("int tee(float b, string c){b}"));
    eq!(result.kind, Kind::declaration);
    eq!(result.name, "tee");
    let signature_node = result["@signature"];
    //	let signature_node = result.metas()["signature"];
    if (not
    signature_node.value.data);
    error("no signature");
    Signature & signature = *(Signature *)
    signature_node.value.data;
    eq!(signature.functions.first()->name, "tee");
    eq!(signature.parameters.size(), 2);
    eq!(signature.parameters.first().name, "b");
    eq!(signature.parameters.first().type, reals); // use real / number for float64  float32
    eq!(signature.parameters.last().name, "c");
    eq!(signature.parameters.last().type, strings);
    // let params = signature.parameters.map(+[](Arg f) { return f.name; });
    // eq!(params.first(), "b");
}

#[test]
fn testEmptyTypedFunctions() {
    // todo int a(){} should be compiler error
    // todo do we really want / need int a(); #[test] fn a(){} ?
    //	if(ch=='{' and next=='}' and previous==')'){
    //		actual.setType(declaration, false);// a(){} => def a!
    //		proceed();
    //		proceed();
    //		break;
    //	}
    result = analyze(parse("int a(){}"));
    eq!(result.kind, Kind::declaration);
    eq!(result.name, "a");
    let signature_node = result["@signature"];
    Signature
    signature = *(Signature *)
    signature_node.value.data;
    eq!(signature.functions.first()->name, "a");
    let names2 = signature.functions.map < String > ( + [](Function * f)
    { return f; ->name; });
    eq!(names2.size(), 1);
    eq!(names2.first(), "a");

    result = analyze(parse("int a();"));
    eq!(result.kind, Kind::declaration); // header signature
    eq!(result.type, IntegerType);
    eq!(result.name, "a");
}

#[test]
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
fn testPolymorphism() {
    // debug:
    //	let debug_node = parse("string aaa(string a){return a};\nfloat bbb(float b){return b+1}");
    //	let debug_fun = analyze(debug_node);
    let node = parse("string test(string a){return a};\nfloat test(float b){return b+1}");
    let fun = analyze(node);
    let function = functions["test"];
    eq!(function.is_polymorphic, true);
    eq!(function.variants.size(), 2);
    eq!(function.variants[0]->signature.size(), 1);
    //	eq!(function.variants[0].signature.parameters[0].type, (Type) strings); todo
    eq!(function.variants[0]->signature.parameters[0].type, (Type) stringp);
    let variant = function.variants[1];
    eq!(variant->signature.size(), 1);
    eq!(variant->signature.parameters[0].type, (Type) float32t);
}

#[test]
fn testPolymorphism2() {
    clearAnalyzerContext();
    let node = parse("fun test(string a){return a};\nfun test(float b){return b+1}");
    let fun = analyze(node);
    let function = functions["test"];
    eq!(function.is_polymorphic, true);
    eq!(function.variants.size(), 2);
    eq!(function.variants[0]->signature.size(), 1);
    eq!(function.variants[0]->signature.parameters[0].type, (Type) int32t);
    eq!(function.variants[1]->signature.size(), 1);
    eq!(function.variants[1]->signature.parameters[0].type, (Type) float32t);
}

#[test]
fn testPolymorphism3() {
    is!("fun test(string a){return a};\nfun test(float b){return b+1};\ntest('ok')", "ok");
    is!("fun test(string a){return a};\nfun test(int a){return a};\nfun test(float b){return b+1};\ntest(1.0)",
                2.0);
}

#[test]
fn testGenerics() {
    let typ = Type(Generics { kind: array, value_type: int16t });
    //    let header= typ.value & array;
    //    let header= typ.value & 0xFFFF0000; // todo <<
    let header = typ.value & 0x0000FFFF; //todo ??
//     assert!(_eq(header, array);
}

#[test]
fn testFunctionArgumentCast() {
    is!("float addi(int x,int y){x+y};'hello'+5", "hello5");
    is!("float addi(int x,int y){x+y};'hello'+5.9", "hello5.9");
    is!("float addi(int x,int y){x+y};'hello'+addi(2.2,2.2)", "hello4.");
//     is!("float addi(int x,int y){x+y};'hello'+addi(2,3)", "hello5.") // OK some float cast going on!

    is!("fun addier(a,b){b+a};addier(42.0,1.0)", 43);
    is!("fun addier(int a,int b){b+a};addier(42,1)+1", 44);
    is!("fun addi(int x,int y){x+y};addi(2.2,2.2)", 4);
    is!("fun addi(float x,float y){x+y};addi(2.2,2.2)", 4.4);
    is!("float addi(int x,int y){x+y};addi(2.2,2.2)", 4.4);
    is!("fun addier(float a,float b){b+a};addier(42,1)+1", 44);
}

