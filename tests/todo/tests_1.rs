//==============================================================================
// Main test file for Wasp/Angle language
//==============================================================================
// This file contains ~200+ test functions not yet organized into logical categories.
//
// WIP: Gradually split this monolithic file into category-specific files:
//   - test_string.rs, test_list.rs, test_map.rs, test_type.rs
//   - test_math.rs, test_node.rs, test_parser.rs, test_web.rs
//   - test_feature.rs, test_implementation.rs, etc.
//

//==============================================================================
// TYPE SYSTEM TESTS (see type_tests.h for declarations);
//==============================================================================

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
fn testNotTruthyFalsy() {
    is!("not ''", 1);
    is!("not \"\"", 1);
}

#[test]
fn testNotNegation2() {
    // just aliases for 'not'
    is!("!0", 1);
    is!("!0.0", 1);
    is!("!1", 0);
    is!("!1.1", 0);
    is!("!2", 0);
    is!("!2.1", 0);
    is!("! 0", 1);
    is!("! 0.0", 1);
    is!("! 1", 0);
    is!("! 1.1", 0);
    is!("! 2", 0);
    is!("! 2.1", 0);
    is!("¬0", 1);
    is!("¬0.0", 1);
    is!("¬1", 0);
    is!("¬1.1", 0);
    is!("¬2", 0);
    is!("¬2.1", 0);
}

#[test]
fn testNotNegation() {
    // testNotNegation2(); // just aliases for 'not'
    is!("not 0", 1);
    is!("not 0.0", 1);
    is!("not true", 0);
    is!("not True", 0);
    is!("not false", 1);
    is!("not False", 1);
    is!("not 1", 0);
    is!("not 1.1", 0);
    is!("not 2", 0);
    is!("not 2.1", 0);
    is!("not (1==1)", 0);
    is!("not (1==2)", 1);
    is!("not (1==2)", 1);
    is!("not (2==2)", 0);
    is!("(not 1)==0", 1);
    is!("(not 0)==1", 1);
    is!("(not 1)==1", 0);
    is!("(not 0)==0", 0);
    is!("i=2;not (i==3)", 1);
    is!("i=2;not (i==2)", 0);
    is!("i=3;not (i==3)", 0);
    is!("i=3;not (i==2)", 1);
}

#[test]
fn testWhileNot() {
    is!("i=2;while !1:i++;i", 2);
    is!("i=2;while i!=3:i++;i", 3);
    is!("i=2;while i!=3:i++;i", 3);
    is!("i=2;while i!=3:i++;i", 3);
    is!("i=2;while i!=3:i++;i", 3);
    is!("i=2;while i!=3:i++;i", 3);
    is!("i=2;while i<=3:i++;i", 4);
    is!("i=2;while not 1:i++;i", 2);
    is!("i=2;while i<=3:i++;i", 4);
}

#[test]
fn testWhileNotCall() {
    // Tests with function calls in while conditions
    // Note: These require proper handling of function calls as while conditions
    skip!(
 // todo!
    is!("def goon():0;;while goon() : {0};42", 42);
    is!("def goon():0;;while goon():{0};42", 42);
        is!("def goon():0;;while goon():1;42", 42);
        is!("def goon():0;;while goon():0;42", 42); todo
        is!("def goon():0;;while goon():{};42", 42);
        is!("def goon():0;;while goon():{1};42", 42);
        is!("def goon():0;;while goon(){};42", 42);
        is!("def goon():0;;while(goon()):0;42", 42);
        is!("def goon():0;;while(goon()):{};42", 42);
        is!("def goon():0;;while(goon()){};42", 42);
        is!("def goon(){0};;while(goon()): {0};42", 42);
    is!("def goon():0;;while(goon()): {0};42", 42);
    is!("def goon():0;;while(goon()):{0};42", 42);
    is!("def goon():{0};while goon() : {0};42", 42);
    is!("def stop():1;;while !stop() : {0};42", 42);
    is!("def stop():{1};while !stop() : {0};42", 42);

    is!("def goon(){0};;while(goon()){0};42", 42);
    );

    is!("goon=0;while(goon){goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while goon{goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while goon:{goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while(goon):{goon+=1};goon+2", 2); // Variable instead of call
    is!("stop=1;while not stop:{stop++};stop", 1); // Variable with not
}
#[test]
fn test_while_true_forever() {
    todow("test_while_true_forever");

    skip!(

        is!("def stop():{0};while !stop() : {}", 0); // should hang forever ;);
        is!("def goo():{1};while goo() : {}", 0); // should hang forever ;);

        let node = parse("1:2");
        print(node.serialize());
        assert!(node.serialize() == "1:2");
        assert!(node.values().value.longy == 2);
        assert!(node.kind == pair or node.kind == key);
        assert!(node.value.longy == 1);
        is!("while True : 2", 0); // should hang forever ;);
        // is!("while 1 : 2", 0); // should hang forever ;);
    );
}
#[test]
fn testTypeSynonyms() {
    // eq!(Type("i32"s),Type("int32"s));
    // eq!(Type("i32"s),Type("int"s));
    // eq!(Type("f32"s),Type("float32"s));
    // eq!(Type("f32"s),Type("float"s));
}
#[test]
fn testWaspRuntimeModule() {
    print("sizeof(Module)");
    print(sizeof(Module));
    print("sizeof(Function)");
    print(sizeof(Function));
    Module & wasp = loadRuntime();
    // print(wasp);
    // eq!(wasp.name, "wasp");
    assert!(wasp.name.contains("wasp")); // wasp-runtime.wasm in system 'wasp' in js!
    // addLibrary(wasp);
    #[cfg(feature = "WASM")]{
        // assert!(libraries.size()>0);
        // if it breaks then in WASM too!?
    }
    assert!(wasp.code_count > 400);
    assert!(wasp.data_segments_count > 5);
    assert!(wasp.export_count > wasp.code_count - 10);
    assert!(wasp.import_count < 40);
    // assert!(wasp.export_names.has("memory")); // type memory
    // assert!(wasp.export_names.has("strlen")); // type func export "C"
    // assert!(wasp.export_names.has("Z4powiij")); // type func mangled for overload
    // assert!(wasp.import_names.has("proc_exit")); // not important but todo broken in wasm!
    // wasp.signatures
    assert!(wasp.functions.size() > 100);
    // assert!(wasp.functions.has("Z4powiij"));// extern "C"'ed
    assert!(wasp.functions.has("powi")); // ok if not WASM
    assert!(wasp.functions.has("powd")); // ok if not WASM
    // todo load wasp-runtime-debug.wasm for:
    // assert!(wasp.functions.has("test42"));
    assert!(wasp.functions.has("modulo_float"));
    assert!(wasp.functions.has("modulo_double"));
    assert!(wasp.functions.has("square"));
    assert!(wasp.functions["square"].is_polymorphic);
    eq!(wasp.functions["square"].variants.size(), 2);
    eq!(wasp.functions["square"].name, "square"); // or mangled
    eq!(wasp.functions["square"].variants[0]->name, "Z6squarei");
    eq!(wasp.functions["square"].variants[0]->signature.parameters.size(), 1);
    eq!(wasp.functions["square"].variants[0]->signature.parameters[0].type, ints);
    eq!(wasp.functions["square"].variants[1]->name, "Z6squared");
    eq!(wasp.functions["square"].variants[1]->signature.parameters.size(), 1);
    eq!(wasp.functions["square"].variants[1]->signature.parameters[0].type, reals);
}

#[test]
    skip!(
fn test_list_lambdas() {
    // List<int> nums = { 1, -2, 3, -4, 5 };
    negCount2 = nums.count( + [](int & x)
    { return x < 0; });
    assert!(negCount2 == 2);

    // Remove negatives in-place
    nums.remove( + [](int & x)
    { return x < 0; }); // [1, 3, 5]
    assert!(nums.length() == 3);

    // Filter to new list
    let positives = nums.filter( + [](int & x)
    { return x > 0; });
    assert!(positives.length() == 3);

    // assert! conditions
    bool
    hasNeg = nums.any( + [](int & x)
    { return x < 0; });
    bool
    allPos = nums.all( + [](int & x)
    { return x > 0; });
    int
    negCount = nums.count( + [](int & x)
    { return x < 0; });
    assert!(negCount == 0);
    assert!(!hasNeg);
    assert!(allPos);
}

#[test]
fn testMetaField() {
    tee = parse("tee{a:1}");
    tee["a"]["@attrib"] = 42;
    tee["a"]["@attrib2"] = 43;
    // tee["a"].setMeta("attrib2",(Node) 43);
    // tee["a"].metas()["attrib2"]=(Node) 43;
    eq!(tee.name, "tee");
    assert!(tee["a"]["@attrib"]);
    assert!(tee["a"]["@attrib2"]);
    assert!(tee["a"] == 1);
    assert!(tee.length == 1);
    assert!(tee["a"]["@attrib"].value.longy == 42);
    assert!(tee["a"]["@attrib2"].value.longy == 43);
    eq!(tee.serialize(), "tee{@attrib(42) @attrib2(43) a:1}");
}

#[test]
fn testMeta() {
    tee = parse("tee{a:1}");
    tee["@attrib"] = 42;
    tee["@attrib2"] = 43;
    eq!(tee.name, "tee");
    eq!(tee.serialize(), "@attrib(42) @attrib2(43) tee{a:1}");
    assert!(tee["@attrib"]);
    assert!(tee["@attrib2"]);
    assert!(tee["a"] == 1);
    assert!(tee.length == 1);
    assert!(tee["@attrib"].value.longy == 42);
    assert!(tee["@attrib2"].value.longy == 43);
}

#[test]
fn testMetaAt() {
    eq!(parse("tee{a:1}").name, "tee");
    eq!(parse("tee{a:1}").serialize(), "tee{a:1}");
    let code = "@attrib tee{a:1}";
    let node = parse(code);
    assert!(node.name == "tee");
    assert!(node.length == 1);
    assert!(node["a"] == 1);
    assert!(node["@attrib"]);
}
#[test]
fn testMetaAt2() {
    let code = "@attrib(1) @attrib2(42) tee{a:1}";
    let node = parse(code);
    assert!(node.name == "tee");
    assert!(node.length == 1);
    assert!(node["a"] == 1);
    // eq!(node.serialize(),code); // todo ok except order!
    assert!(node["@attrib"]);
    assert!(node["@attrib2"]);
    eq!(node["@attrib"], 1);
    eq!(node["@attrib2"], 42);
}

#[test]
fn testWGSL() {
    testMeta();
    testMetaAt();
    testMetaAt2();
    testMetaField();
    let code = r#" wgsl{
@group(0) @binding(0);
var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64);
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    data[i] = data[i] * 2;
}
   } "#;
    let wsgl = parse(code);
    assert!(wsgl.name == "wgsl");
    // OUR WGSL parser creates nodes, original is in a string
    // assert!(node.kind == strings);
    // assert!(node.kind == datas);
    // assert!(node.value.string->contains("@compute"));
    print(wsgl);
    assert!(wsgl.length == 2);
    // TODO: a lot ;);
    // assert!(node[0].kind == wgsl_function);
    // assert!(wsgl[1]["name"] == "main");
    // assert!(wsgl[1]["workgroup_size"] == "64");
    // assert!(wsgl[1]["body"].length == 1);
}

#[test]
fn testPing() {
    is!("def ping(): 'pong'; ping()", "pong");
}

#[test]
fn test2Def() {
    // parse("def test1(x){x+1};def test2(x){x+1};test2(3)");
    is!("def test1(x){x+1};def test2(x){x+1};test2(3)", 4);
    is!("def test1(x){x+3};def test2(x){x+1};test2(3)", 6);
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
fn testRandomParse() {
    let node = parse("x:40;x+1");
    assert!(node.length == 2);
    assert!(node[0]["x"] == 40); // breaks!?
    assert!(operator_list.has("+"));
    assert!(not(bool) Node("x"));
    assert! _silent(false == (bool) Node("x"));
    assert!(Node("x") == false);
}

#[test]
fn testEmitStringConcatenation() {
    is!("'say ' + 0.", "say 0.");
    is!("'say ' + (100 + 23)", "say 123");
    is!("'say ' + 123", "say 123");
    is!("'say ' + 'hello'", "say hello");
    // is!("'say ' + 100 + 23", "say 10023");// todo: warn "mixing math op with string concatenation
    is!("'say ' + 1.1", "say 1.1");
    is!("'say ' + 1.1 * 2", "say 2.2");
}

#[test]
fn testStringInterpolation() {
    is!("`hello $test`", "hello hello"); // via externref or params!
    is!("'say ' + $test", "say hello");
    // exit(0);
    skip!(
 // BUT:
        is!("'say ' + $bla", "say 123");
        is!("$test + 'world'", "hello world");
    );
    is!("'say ' 'hello'", "say hello");
    is!("'say ' + 'hello'", "say hello");
    is!("`$test world`", "hello world");

    // exit(0);
    is!("`hello ${42}`", "hello 42");
    is!("`hello ${1+1}`", "hello 2");
    is!("`${42} world`", "42 world");
    is!("`${1+1} world`", "2 world");
    is!("`unaffected`", "unaffected");
    is!("`${'hi'}`", "hi");
    is!("`${1+1}`", "2");
    is!("`1+1=${1+1}`", "1+1=2");
    skip!(

        is!("$test", "hello"); // via externref or params! but calling toLong()!

        is!("x=123;'${x} world'", "123 world") // todo should work
        is!("x='hello';'${x} world'", "hello world") // todo should work
        is!("x='hello';'`$x world`", "hello world") // todo referencex vs reference
    );
}

#[test]
fn testExternString() {
    is!("toString($test)", "hello");
    is!("string x=$test", "hello");
    // exit(1);
    skip!(

        // TODO fix again, $test conflicts with runtime.test function, so …
        is!("$test as string", "hello");
        is!("puts($test)", 21); // "hello"
        is!("puts(toString($hello))", 21);
        is!("$hello as string + 'world'", "helloworld");
        is!("`$test world`", "hello world");
        is!("var x=$hello as string", "hello"); // todo should work with analyze / guess type
        is!("var x=$hello as string;x", "hello");
        is!("print($hello)", "hello"); // (i64) -> nil
        is!("printRef($hello)", "hello"); // (externref) -> nil
        is!("print(toString($hello))", "hello"); // (i64) -> nil via smarti?
    );
}

#[test]
fn testExternReferenceXvalue() {
    is!("real x = $bla", 123.);
    is!("real x = $bla; x*2", 123*2.);
    is!("int x = $bla", 123);
    is!("int x = $bla; x*2", 123*2);
    is!("number x = $bla; x*2", 123*2.);
    skip!(

        is!("2*$bla", 123*2);
    );
}

#[test]
fn testMinusMinus() {
    #[cfg(not(feature = "WASM"))]{ // todo square
        is!("1 - 3 - square 3+4", (int64) -51); // OK!
    }

    //    is!("1 -3 - square 3+4", (int64) -51);// warn "mixing math op with list items (1, -3 … ) !"
    //    is!("1--3", 4);// todo parse error
    is!("1- -3", 4); // -1 uh ok?  warn "what are you doning?"
    is!("1 - -3", 4); // -1 uh ok?  warn "what are you doning?"
    //    is!("1 - - 3", 4);// error ok todo parse error
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
fn testExp() {
    // todo parsed same:
    assert_is("ℯ^0", 1);
    assert_is("ℯ^1", e);
    assert_is("π^0", 1);
    assert_is("π^1", pi);
    assert_is("π*√163", 40.1091); // ok
    skip!(

        assert_is("π√163", 40.1091);
        assert_is("(π*√163)==(π√163)", 1);
        assert_is("π*√163==(π√163)", 1);
        assert_is("π*√163==π√163", 1);
        assert_is("exp(0)", 1); // "TODO rewrite as ℯ^x" OK
    );
    assert_is("ℯ^(π*√163)", 262537412640768743.99999999999925);
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
fn testMatrixOrder() {
    is!("m=([[1, 2], [3, 4]]);m[0][1]", 2);

    //==============================================================================
    // LIST/ARRAY TESTS (see list_tests.h);
    //==============================================================================

    is!("([[1, 2], [3, 4]])[0][1]", 2);
    is!("([[1, 2], [3, 4]])[1][0]", 3);
    is!("([1, 2], [3, 4])[1][0]", 3);
    is!("(1, 2; 3, 4)[1][0]", 3);
    is!("(1, 2; 3, 4)[1,0]", 3);
    is!("(1 2, 3 4)[1,0]", 3);
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
fn testInclude() {
    //    is!("include test-include.wasp", 42);
    //    is!("use test-include.wasm", 42);
    is!("include test/lib.wasp", 42);
    //    is!("include test/lib.wast", 42);
    is!("use test/lib.wasm; test", 42);
    //    is!("use https://pannous.com/files/lib.wasm; test", 42);
    //    is!("use git://pannous/waps/test/lib.wasm; test", 42);
    //    is!("use system:test/lib.wasm; test", 42); // ^^
}

#[test]
fn testExceptions() {
    //    is!("(unclosed bracket",123);
    assert_throws("x:int=1;x='ok'"); // worked before, cleanup fail!
    assert_throws("x:int=1;x=1.1");
    skip!(

    );
    //    is!("x:int=1;x=1.0",1); // might be cast by compiler
    //    is!("x=1;x='ok';x=1", 1); // untyped x can be reassigned
    assert_throws("'unclosed quote");
    assert_throws("\"unclosed quote");
    assert_throws("unclosed quote'");
    assert_throws("unclosed quote\"");
    assert_throws("unclosed bracket)");
    assert_throws("(unclosed bracket");
}

#[test]
fn testNoBlock() {
    // fixed
    assert_parses(r#"
#see math.wasp !
τ=π*2
#assert τ≈6.2831853
#τ≈6.2831853
#τ==6.2831853
    "#);
}

#[test]
fn testTypeConfusion() {
    assert_throws("x=1;x='ok'");
    assert_throws("x=1;x=1.0");
    assert_throws("double:=it*2"); // double is type i64!
    // todo: get rid of stupid type name double, in C it's float64 OR int64 anyway
}

#[test]
fn testVectorShim() {
    //    unknown function matrix_multiply (matrix_multiply);
    is!("v=[1 2 3];w=[2 3 4];v*w", 2 + 6 + 12);
}

#[test]
fn testHtmlWasp() {
    eval("html{bold{Hello}}"); // => <html><body><bold>Hello</bold></body></html> via appendChild bold to body
    eval("html: h1: 'Hello, World!'"); // => <html><h1>Hello, World!</h1></html>
    //	eval("html{bold($myid style=red){Hello}}"); // => <bold id=myid style=red>Hello</bold>
}

#[test]
fn testJS() {
    // todo remove (local $getContext i32)  !
    eval("$canvas.getContext('2d')"); // => invokeReference(canvas, getContext, '2d');
    skip!(

        eval("js{alert('Hello')}"); // => <script>alert('Hello')</script>
        eval("script{alert('Hello')}"); // => <script>alert('Hello')</script>
    );
}

#[test]
fn testInnerHtml() {
    #[cfg(not(any(feature = "WEBAPP", feature = "MY_WASM")))]{
        return;
    }
    let html = parse("<html><bold>test</bold></html>");
    eq!(html.kind, Kind::strings);
    assert!(html.value.string);
    eq!(*html.value.string, "<bold>test</bold>");
    let serialized = html.serialize();
    eq!(serialized, "<html><bold>test</bold></html>");
    //	eval("<html><script>alert('ok')");
    //	eval("<html><script>alert('ok')</script></html>");
    #[cfg(feature = "WEBAPP")]{ // todo browser "too"
        // skip!(

        eval("<html><bold id=b ok=123>test</bold></html>");
        assert_is("$b.ok", 123); // TODO emitAttributeSetter
        eval("<script>console.log('ok!')</script>");
        eval("<script>alert('alert ok!')</script>"); // // pop up window NOT supported by WebView, so we use print instead
        // );
    }

    //	eval("$b.innerHTML='<i>ok</i>'");
    //	eval("<html><bold id='anchor'>…</bold></html>");
    //	eval("$anchor.innerHTML='<i>ok</i>'");
    //
    ////	eval("x=<html><bold>test</bold></html>;$results.innerHTML=x");
    //	eval("$results.innerHTML='<bold>test</bold>'");
}

#[test]
fn testHtml() {
    //	testHtmlWasp();
    //	testJS();
    testInnerHtml();
}
#[test]
fn testReplaceAll() {
    s = "abaabaa";
    let replaced = s.replaceAll("a", "ca");
    //	let replaced = s.replaceAll('a', "ca");
    eq!(replaced, "cabcacabcaca");
    let replaced2 = replaced.replaceAll("ca", "a");
    eq!(replaced2, "abaabaa");
    replaced2.replaceAllInPlace('b', 'p');
    eq!(replaced2, "apaapaa");
}

#[test]
fn testFetch() {
    // todo: use host fetch if available
    let string1 = fetch("https://pannous.com/files/test");
    let res = String(string1).trim();
    if (res.contains("not available")) {
        print("fetch not available. set CURL=1 in CMakelists.txt or use host function");
        return;
    }
    assert! _eq(res, "test 2 5 3 7");
    assert! _emit("fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    assert! _emit("x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    skip!(

        assert!_emit("string x=fetch https://pannous.com/files/test;y=7;x", "test 2 5 3 7\n");
        assert!_emit("string x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    );
}

#[test]
fn test_getElementById() {
    result = analyze(parse("$result"));
    eq!(result.kind, externref);
    let nod = eval("$result");
    print(nod);
}

#[test]
fn testCanvas() {
    result = analyze(parse("$canvas"));
    eq!(result.kind, externref);
    let nod = eval("    ctx = $canvas.getContext('2d');\n"
                   "    ctx.fillStyle = 'red';\n"
                   "    ctx.fillRect(10, 10, 150, 100);");
    print(nod);
}

// run in APP (or browser?);
#[test]
fn testDom() {
    print("testDom");
    preRegisterFunctions();
    result = analyze(parse("getElementById('canvas')"));
    eq!(result.kind, call);
    result = eval("getElementById('canvas');");
    //	print(typeName(result.kind));
    //	eq!(result.kind, strings); // why?
    //	eq!(result.kind, longs); // todo: can't use smart pointers for elusive externref
    //	eq!(result.kind, bools); // todo: can't use smart pointers for elusive externref
    print(typeName(30));
    print(typeName(9));
    //	eq!(result.kind, 30);//
    //	eq!(result.kind,9);//
    //	eq!(result.kind, (int64) externref); // todo: can't use smart pointers for elusive externref
    //	result = eval("document.getElementById('canvas');");
    //	result = analyze(parse("$canvas"));
    //	eq!(result.kind, (int64) externref);
}

inline #[test]
fn print(Primitive l) {
    print(typeName(l));
}

#[test]
fn testDomProperty() {
    #[cfg(not(feature = "WEBAPP"))]{
        return;
    }
    result = eval("getExternRefPropertyValue($canvas,'width')"); // ok!!
    eq!(result.value.longy, 300); // only works because String "300" gets converted to BigInt 300
    //	result = eval("width='width';$canvas.width");
    result = eval("$canvas.width");
    assert! _eq(result.value.longy, 300);
    //	return;
    result = eval("$canvas.style");
    eq!(result.kind, strings);
    //	eq!(result.kind, stringp);
    if (result.value.string);
    assert! _eq(*result.value.string, "dfsa");
    //	getExternRefPropertyValue OK  [object HTMLCanvasElement] style [object CSSStyleDeclaration]
    // ⚠️ But can't forward result as smarti or stringref:  SyntaxError: Failed to parse String to BigInt
    // todo : how to communicate new string as RETURN type of arbitrary function from js to wasp?
    // call Webview.getString(); ?

    //	embedder.trace('canvas = document.getElementById("canvas");');
    //	print(nod);
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
fn testModifiers() {
    is!("public fun ignore(){3}", 3);
    is!(
        "public static export import extern external C global inline virtual override final abstract private protected internal const constexpr volatile mutable thread_local synchronized transient native fun ignore(){3}",
        3);
}

//#import "pow.h"
//#[test] fn testOwnPowerExponentialLogarithm() {
// 
//	eq!(exp(1), 2.718281828459045);
//	eq!(exp(5.5), 244.69193226422033);
//	let x = powerd(1.5, 5.5);
//	printf("1.5^5.5=%f", x);
//	assert!_eq(x, 9.30040636712988);
//	let x1 = powerd(2.5, 1.5);
//	printf("2.5^1.5=%f", x1);
//	assert!_eq(x1, 3.952847075210474);
//	let x2 = powerd(2.5, 3.5);
//	assert!_eq(x2, 24.705294220065465);
//}
    );
// // // // // // // // 
