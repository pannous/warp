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
// TYPE SYSTEM TESTS (see type_tests.h for declarations)
//==============================================================================

#[test] fn testGoTypes() {
    is!("func add1(x int) int { return x + 1 };add1(41)", 42);
}

#[test] fn testAutoType() {
    is!("0/0", Nan);
    is!("0÷0", Nan);
    is!("-1/6.", -1/6.);
    is!("-1/6", -1/6.); // Auto-promote int/int division to float
    is!("-1÷6", -1/6.); // Auto-promote int/int division to float
}
#[test] fn testNotTruthyFalsy() {
    is!("not ''", 1);
    is!("not \"\"", 1);
}

#[test] fn testNotNegation2() {
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

#[test] fn testNotNegation() {
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

#[test] fn testWhileNot() {
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

#[test] fn testWhileNotCall() {
    // Tests with function calls in while conditions
    // Note: These require proper handling of function calls as while conditions
    skip( // todo!
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
    )

    is!("goon=0;while(goon){goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while goon{goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while goon:{goon+=1};goon+2", 2); // Variable instead of call
    is!("goon=0;while(goon):{goon+=1};goon+2", 2); // Variable instead of call
    is!("stop=1;while not stop:{stop++};stop", 1); // Variable with not
}
#[test] fn test_while_true_forever() {
    todow("test_while_true_forever");

    skip(
        is!("def stop():{0};while !stop() : {}", 0); // should hang forever ;)
        is!("def goo():{1};while goo() : {}", 0); // should hang forever ;)

        let node = parse("1:2");
        print(node.serialize());
        assert!(node.serialize() == "1:2");
        assert!(node.values().value.longy == 2);
        assert!(node.kind == pair or node.kind == key);
        assert!(node.value.longy == 1);
        is!("while True : 2", 0); // should hang forever ;)
        // is!("while 1 : 2", 0); // should hang forever ;)
    )
}
#[test] fn testTypeSynonyms() {
    // eq!(Type("i32"s),Type("int32"s));
    // eq!(Type("i32"s),Type("int"s));
    // eq!(Type("f32"s),Type("float32"s));
    // eq!(Type("f32"s),Type("float"s));
}
#[test] fn testWaspRuntimeModule() {
    print("sizeof(Module)");
    print(sizeof(Module));
    print("sizeof(Function)");
    print(sizeof(Function));
    Module &wasp = loadRuntime();
    // print(wasp);
    // eq!(wasp.name, "wasp");
    assert!(wasp.name.contains("wasp")); // wasp-runtime.wasm in system 'wasp' in js!
    // addLibrary(wasp);
#[cfg(feature = "WASM")]{
    // assert!(libraries.size()>0);
    // if it breaks then in WASM too!?
}
    assert!(wasp.code_count>400);
    assert!(wasp.data_segments_count>5);
    assert!(wasp.export_count>wasp.code_count-10);
    assert!(wasp.import_count<40);
    // assert!(wasp.export_names.has("memory")); // type memory
    // assert!(wasp.export_names.has("strlen")); // type func export "C"
    // assert!(wasp.export_names.has("_Z4powiij")); // type func mangled for overload
    // assert!(wasp.import_names.has("proc_exit")); // not important but todo broken in wasm!
    // wasp.signatures
    assert!(wasp.functions.size() > 100);
    // assert!(wasp.functions.has("_Z4powiij"));// extern "C"'ed
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
    eq!(wasp.functions["square"].variants[0]->name, "_Z6squarei");
    eq!(wasp.functions["square"].variants[0]->signature.parameters.size(), 1);
    eq!(wasp.functions["square"].variants[0]->signature.parameters[0].type, ints);
    eq!(wasp.functions["square"].variants[1]->name, "_Z6squared");
    eq!(wasp.functions["square"].variants[1]->signature.parameters.size(), 1);
    eq!(wasp.functions["square"].variants[1]->signature.parameters[0].type, reals);
}

#[test] fn test_list_lambdas() {
    List<int> nums = {1, -2, 3, -4, 5};
    int negCount2 = nums.count(+[](int &x) { return x < 0; });
    assert!(negCount2 == 2);

    // Remove negatives in-place
    nums.remove(+[](int &x) { return x < 0; }); // [1, 3, 5]
    assert!(nums.length() == 3);

    // Filter to new list
    let positives = nums.filter(+[](int &x) { return x > 0; });
    assert!(positives.length() == 3);

    // assert! conditions
    bool hasNeg = nums.any(+[](int &x) { return x < 0; });
    bool allPos = nums.all(+[](int &x) { return x > 0; });
    int negCount = nums.count(+[](int &x) { return x < 0; });
    assert!(negCount == 0);
    assert!(!hasNeg);
    assert!(allPos);
}

#[test] fn testMetaField() {
    Node tee = parse("tee{a:1}");
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

#[test] fn testMeta() {
    Node tee = parse("tee{a:1}");
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

#[test] fn testMetaAt() {
    eq!(parse("tee{a:1}").name, "tee");
    eq!(parse("tee{a:1}").serialize(), "tee{a:1}");
    let code = "@attrib tee{a:1}";
    let node = parse(code);
    assert!(node.name == "tee");
    assert!(node.length == 1);
    assert!(node["a"] == 1);
    assert!(node["@attrib"]);
}
#[test] fn testMetaAt2() {
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

#[test] fn testWGSL() {
    testMeta();
    testMetaAt();
    testMetaAt2();
    testMetaField();
    let code = r#" wgsl{
@group(0) @binding(0)
var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64)
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
    // TODO: a lot ;)
    // assert!(node[0].kind == wgsl_function);
    // assert!(wsgl[1]["name"] == "main");
    // assert!(wsgl[1]["workgroup_size"] == "64");
    // assert!(wsgl[1]["body"].length == 1);
}

#[test] fn testPing() {
    is!("def ping(): 'pong'; ping()", "pong");
}

#[test] fn test2Def() {
    // parse("def test1(x){x+1};def test2(x){x+1};test2(3)");
    is!("def test1(x){x+1};def test2(x){x+1};test2(3)", 4);
    is!("def test1(x){x+3};def test2(x){x+1};test2(3)", 6);
}

#[test] fn testReturnTypes() {
    is!("fun addier(a,b){b+a};addier(42,1)", 43);
    is!("fun addier(a,b){b+a};addier(42,1)+1", 44);
    is!("fun addi(x,y){x+y};addi(2.2,2.2)", 4.4)
    is!("float addi(x,y){x+y};addi(2.2,2.2)", 4.4)
    is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
    is!("add1 x:=x+1;add1 3", (int64) 4);
    is!("int x = $bla", 123);
    is!("`${1+1}`", "2")
    is!("real x = $bla", 123.);
    skip(
        is!("k=(1,2,3);i=1;k#i=4;k#1", 4) // fails linking _ZdlPvm operator delete(void*, unsigned long)
        is!("i=1;k='hi';k#i", 'h'); // BUT IT WORKS BEFORE!?! be careful with i64 smarty return!
    )

    //==============================================================================
    // STRING TESTS (see string_tests.h)
    //==============================================================================
}

#[test] fn testRandomParse() {
    const Node &node = parse("x:40;x+1");
    assert!(node.length == 2)
    assert!(node[0]["x"] == 40) // breaks!?
    assert!(operator_list.has("+"));
    assert!(not(bool) Node("x"));
    assert!_silent(false == (bool) Node("x"));
    assert!(Node("x") == false);
}

#[test] fn testEmitStringConcatenation() {
    is!("'say ' + 0.", "say 0.");
    is!("'say ' + (100 + 23)", "say 123");
    is!("'say ' + 123", "say 123");
    is!("'say ' + 'hello'", "say hello");
    // is!("'say ' + 100 + 23", "say 10023");// todo: warn "mixing math op with string concatenation
    is!("'say ' + 1.1", "say 1.1");
    is!("'say ' + 1.1 * 2", "say 2.2");
}

#[test] fn testStringInterpolation() {
    is!("`hello $test`", "hello hello"); // via externref or params!
    is!("'say ' + $test", "say hello");
    // exit(0);
    skip( // BUT:
        is!("'say ' + $bla", "say 123");
        is!("$test + 'world'", "hello world");
    )
    is!("'say ' 'hello'", "say hello");
    is!("'say ' + 'hello'", "say hello");
    is!("`$test world`", "hello world");

    // exit(0);
    is!("`hello ${42}`", "hello 42");
    is!("`hello ${1+1}`", "hello 2");
    is!("`${42} world`", "42 world");
    is!("`${1+1} world`", "2 world");
    is!("`unaffected`", "unaffected")
    is!("`${'hi'}`", "hi")
    is!("`${1+1}`", "2")
    is!("`1+1=${1+1}`", "1+1=2")
    skip(
        is!("$test", "hello"); // via externref or params! but calling toLong()!

        is!("x=123;'${x} world'", "123 world") // todo should work
        is!("x='hello';'${x} world'", "hello world") // todo should work
        is!("x='hello';'`$x world`", "hello world") // todo referencex vs reference
    )
}

#[test] fn testExternString() {
    is!("toString($test)", "hello");
    is!("string x=$test", "hello");
    // exit(1);
    skip(
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
    )
}

#[test] fn testExternReferenceXvalue() {
    is!("real x = $bla", 123.);
    is!("real x = $bla; x*2", 123*2.);
    is!("int x = $bla", 123);
    is!("int x = $bla; x*2", 123*2);
    is!("number x = $bla; x*2", 123*2.);
    skip(
        is!("2*$bla", 123*2);
    )
}

#[test] fn testMinusMinus() {
#[cfg(not(feature = "WASM"))]{ // todo square
    is!("1 - 3 - square 3+4", (int64) -51); // OK!
}

    //    is!("1 -3 - square 3+4", (int64) -51);// warn "mixing math op with list items (1, -3 … ) !"
    //    is!("1--3", 4);// todo parse error
    is!("1- -3", 4); // -1 uh ok?  warn "what are you doning?"
    is!("1 - -3", 4); // -1 uh ok?  warn "what are you doning?"
    //    is!("1 - - 3", 4);// error ok todo parse error
}

extern "C"
Node cast_smart(smarty value, Type to_type) {
    return cast(Node(value), to_type);
}

#[test] fn testCast() {
    assert!_eq("2"s, cast(Node(2), strings).value.string);
    assert!_eq(cast(Node(2), longs).value.longy, 2); // trivial
    assert!_eq(cast(Node(2.1), longs).value.longy, 2);
    assert!_eq(cast(Node(2), reals).value.real, 2.0);
    assert!_eq(cast(Node(2.1), reals).value.real, 2.1);
    assert!_eq("2.1"s, cast(Node(2.1), strings).value.string);
    assert!_eq("a"s, cast(Node('a'), strings).value.string);
    assert!_eq(false, cast(Node('0'), bools).value.longy);
    assert!_eq(false, cast(Node(u'ø'), bools).value.longy);
    assert!_eq(false, cast(Node("False"s, false), bools).value.longy);
    assert!_eq(false, cast(Node("ø"s, false), bools).value.longy);
    assert!_eq(true, cast(Node("True"s, false), bools).value.longy);
    assert!_eq(true, cast(Node("1"s, false), bools).value.longy);
    assert!_eq(true, cast(Node(1), bools).value.longy);
    assert!_eq(true, cast(Node("abcd"s, false), bools).value.longy);
}
#[test] fn testEmitCast() {
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

#[test] fn testExp() {
    // todo parsed same:
    assert_is("ℯ^0", 1);
    assert_is("ℯ^1", e);
    assert_is("π^0", 1);
    assert_is("π^1", pi);
    assert_is("π*√163", 40.1091); // ok
    skip(
        assert_is("π√163", 40.1091);
        assert_is("(π*√163)==(π√163)", 1);
        assert_is("π*√163==(π√163)", 1);
        assert_is("π*√163==π√163", 1);
        assert_is("exp(0)", 1); // "TODO rewrite as ℯ^x" OK
    )
    assert_is("ℯ^(π*√163)", 262537412640768743.99999999999925);
}

#[test] fn testConstructorCast() {
    is!("int('123')", 123);
    is!("str(123)", "123");
    is!("'a'", 'a');
    is!("char(0x41)", 'a');
    is!("string(123)", "123");
    is!("String(123)", "123");
}

#[cfg(feature = "WASMEDGE")]{
int test_wasmedge_gc() {
    // Initialize WasmEdge runtime
    WasmEdge_ConfigureContext *Conf = WasmEdge_ConfigureCreate();
    WasmEdge_ConfigureAddProposal(Conf, WasmEdge_Proposal_ReferenceTypes);
    WasmEdge_ConfigureAddProposal(Conf, WasmEdge_Proposal_GC);

    WasmEdge_VMContext *VM = WasmEdge_VMCreate(Conf, NULL);

    // Load the WASM module
    //    WasmEdge_String ModulePath = WasmEdge_StringCreateByCString("gc_example.wasm");
    WasmEdge_String ModuleName = WasmEdge_StringCreateByCString("gc_example");
    const char *path = "/Users/me/dev/script/wasm/gc_structs/gc_example.wasm";
    WasmEdge_Result Result = WasmEdge_VMRegisterModuleFromFile(VM, ModuleName, path);
    if (!WasmEdge_ResultOK(Result)) {
        printf("Failed to load module: %s\n", WasmEdge_ResultGetMessage(Result));
        return 1;
    }

    // Run the `new_object` function
    WasmEdge_String FuncName = WasmEdge_StringCreateByCString("new_object");
    WasmEdge_Value Params[0] = {}; //WasmEdge_ValueGenI32(32)};
    WasmEdge_Value Returns[1];
    int wasm_file_size = 0;
    const uint8_t *wasm_buffer = reinterpret_cast<const uint8_t *>(readFile(path, &wasm_file_size));
    Result = WasmEdge_VMRunWasmFromBuffer(VM, wasm_buffer, wasm_file_size, FuncName, Params, 0, Returns, 1);
    if (!WasmEdge_ResultOK(Result)) {
        printf("Failed to execute function: %s\n", WasmEdge_ResultGetMessage(Result));
        return 1;
    }
    let mem = WasmEdge_StringCreateByCString("memory");
    //    WasmEdge_ModuleInstanceContext *module_ctx2 = WasmEdge_VMGetStoreContext(VM);
    //    WasmEdge_StoreContext *storeContext = WasmEdge_VMGetStoreContext(VM);
    const WasmEdge_ModuleInstanceContext *module_ctx = WasmEdge_VMGetActiveModule(VM);
    WasmEdge_MemoryInstanceContext *memory_ctx = WasmEdge_ModuleInstanceFindMemory(module_ctx, mem);
    uint8_t *memo = WasmEdge_MemoryInstanceGetPointer(memory_ctx, 0, 0);
    if (memo)
        wasm_memory = memo;
    else
        warn("⚠️Can't connect wasmedge memory");
    // Print the result (object reference)
    WasmEdge_Value Return = Returns[0];
    #[test] fn *p#[test] fn = WasmEdge_ValueGetExternRef(Return);
    if (WasmEdge_ValTypeIsRef(Return.Type)) {
        printf("Result REF: %p\n", pVoid);
    } else {
        printf("Result: %d\n", WasmEdge_ValueGetI32(Return));
    }
    printf("Result: %p\n", pVoid);
    printf("Result: %d\n", *(int *) pVoid);
    printf("Result: %d\n", WasmEdge_ValueGetI32(Return));
    //    exit(0);

    // Cleanup
    WasmEdge_VMDelete(VM);
    WasmEdge_ConfigureDelete(Conf);

    return 0;
}
}

#[test] fn testMatrixOrder() {
    is!("m=([[1, 2], [3, 4]]);m[0][1]", 2);

    //==============================================================================
    // LIST/ARRAY TESTS (see list_tests.h)
    //==============================================================================

    is!("([[1, 2], [3, 4]])[0][1]", 2);
    is!("([[1, 2], [3, 4]])[1][0]", 3);
    is!("([1, 2], [3, 4])[1][0]", 3);
    is!("(1, 2; 3, 4)[1][0]", 3);
    is!("(1, 2; 3, 4)[1,0]", 3);
    is!("(1 2, 3 4)[1,0]", 3);
}

template<class S>
#[test] fn testListGrowth() {
    List<S> list; // List<S*> even better!
    for (int i = 0; i < 1000; i++) {
        list.add(*new S());
    }
    assert!_eq(list.size(), 1000);
    for (int i = 0; i < 15; i++) {
        // 10 VERY SLOW in new implementation! DONT DO 20 => growth by 2^20!!!
        list.grow();
    }
    assert!(list.capacity > 1000);
}

#[test] fn testListGrowthWithStrings() {
    List<String> list;
    for (int i = 0; i < 1000; i++) {
        list.add(String(i));
    }
    assert!_eq(list.size(), 1000);
    assert!_eq(list[999], new String(999));
    for (int i = 0; i < 10; i++) {
        list.grow(); // lots of deep copy!!
    }
    assert!(list.capacity > 100000);
    assert!_eq(list[999], new String(999));
}

// test once
#[test] fn test_list_growth() {
    testListGrowth<int>();
    testListGrowth<float>();
    testListGrowth<String>();
    testListGrowth<Signature>();
    testListGrowth<wabt::Index>(); // just int
    testListGrowth<wabt::Reloc>();
    testListGrowth<wabt::Type>();
    testListGrowth<wabt::Location>();
    testListGrowth<wabt::Result>();
    testListGrowth<wabt::TypeVector>();
    testListGrowth<Function>(); // pretty slow with new List shared_ptr implementation
    //    testListGrowth<Map>();
    testListGrowthWithStrings();
}

#[test] fn testBadType() {
    skip(
        // TODO strict mode a:b=c => b is type vs data mode a:b => b is data HOW?
        assert_throws("x:yz=1"); // "yz" is not a type
    )
}

#[test] fn testDeepType() {
    parse("a=$canvas.tagName");
    //    eq!(result.kind, smarti64);
    //    eq!(result.kind, Kind::strings);
}

#[test] fn testInclude() {
    //    is!("include test-include.wasp", 42);
    //    is!("use test-include.wasm", 42);
    is!("include test/lib.wasp", 42);
    //    is!("include test/lib.wast", 42);
    is!("use test/lib.wasm; test", 42);
    //    is!("use https://pannous.com/files/lib.wasm; test", 42);
    //    is!("use git://pannous/waps/test/lib.wasm; test", 42);
    //    is!("use system:test/lib.wasm; test", 42); // ^^
}

#[test] fn testExceptions() {
    //    is!("(unclosed bracket",123);
    assert_throws("x:int=1;x='ok'"); // worked before, cleanup fail!
    assert_throws("x:int=1;x=1.1");
    skip(
    )
    //    is!("x:int=1;x=1.0",1); // might be cast by compiler
    //    is!("x=1;x='ok';x=1", 1); // untyped x can be reassigned
    assert_throws("'unclosed quote");
    assert_throws("\"unclosed quote");
    assert_throws("unclosed quote'");
    assert_throws("unclosed quote\"");
    assert_throws("unclosed bracket)");
    assert_throws("(unclosed bracket");
}

#[test] fn testNoBlock() {
    // fixed
    assert_parses(r#"
#see math.wasp !
τ=π*2
#assert τ≈6.2831853
#τ≈6.2831853
#τ==6.2831853
    "#);
}

#[test] fn testTypeConfusion() {
    assert_throws("x=1;x='ok'");
    assert_throws("x=1;x=1.0");
    assert_throws("double:=it*2"); // double is type i64!
    // todo: get rid of stupid type name double, in C it's float64 OR int64 anyway
}

#[test] fn testVectorShim() {
    //    unknown function matrix_multiply (matrix_multiply)
    is!("v=[1 2 3];w=[2 3 4];v*w", 2 + 6 + 12);
}

#[test] fn testHtmlWasp() {
    eval("html{bold{Hello}}"); // => <html><body><bold>Hello</bold></body></html> via appendChild bold to body
    eval("html: h1: 'Hello, World!'"); // => <html><h1>Hello, World!</h1></html>
    //	eval("html{bold($myid style=red){Hello}}"); // => <bold id=myid style=red>Hello</bold>
}

#[test] fn testJS() {
    // todo remove (local $getContext i32)  !
    eval("$canvas.getContext('2d')"); // => invokeReference(canvas, getContext, '2d')
    skip(
        eval("js{alert('Hello')}"); // => <script>alert('Hello')</script>
        eval("script{alert('Hello')}"); // => <script>alert('Hello')</script>
    )
}

#[test] fn testInnerHtml() {
#[cfg(not(feature = "WEBAPP"))]{ and not MY_WASM
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
    // skip(
    eval("<html><bold id=b ok=123>test</bold></html>");
    assert_is("$b.ok", 123); // TODO emitAttributeSetter
    eval("<script>console.log('ok!')</script>");
    eval("<script>alert('alert ok!')</script>"); // // pop up window NOT supported by WebView, so we use print instead
    // )
}

    //	eval("$b.innerHTML='<i>ok</i>'");
    //	eval("<html><bold id='anchor'>…</bold></html>");
    //	eval("$anchor.innerHTML='<i>ok</i>'");
    //
    ////	eval("x=<html><bold>test</bold></html>;$results.innerHTML=x");
    //	eval("$results.innerHTML='<bold>test</bold>'");
}

#[test] fn testHtml() {
    //	testHtmlWasp();
    //	testJS();
    testInnerHtml();
}
#[test] fn testReplaceAll() {
    String s = "abaabaa";
    let replaced = s.replaceAll("a", "ca");
    //	let replaced = s.replaceAll('a', "ca");
    eq!(replaced, "cabcacabcaca");
    let replaced2 = replaced.replaceAll("ca", "a");
    eq!(replaced2, "abaabaa");
    replaced2.replaceAllInPlace('b', 'p');
    eq!(replaced2, "apaapaa");
}

#[test] fn testFetch() {
    // todo: use host fetch if available
    let string1 = fetch("https://pannous.com/files/test");
    let res = String(string1).trim();
    if (res.contains("not available")) {
        print("fetch not available. set CURL=1 in CMakelists.txt or use host function");
        return;
    }
    assert!_eq(res, "test 2 5 3 7");
    assert!_emit("fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    assert!_emit("x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    skip(
        assert!_emit("string x=fetch https://pannous.com/files/test;y=7;x", "test 2 5 3 7\n");
        assert!_emit("string x=fetch https://pannous.com/files/test", "test 2 5 3 7\n");
    )
}

#[test] fn test_getElementById() {
    result = analyze(parse("$result"));
    eq!(result.kind, externref);
    let nod = eval("$result");
    print(nod);
}

#[test] fn testCanvas() {
    result = analyze(parse("$canvas"));
    eq!(result.kind, externref);
    let nod = eval("    ctx = $canvas.getContext('2d');\n"
        "    ctx.fillStyle = 'red';\n"
        "    ctx.fillRect(10, 10, 150, 100);");
    print(nod);
}

// run in APP (or browser?)
#[test] fn testDom() {
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

inline #[test] fn print(Primitive l) {
    print(typeName(l));
}

#[test] fn testDomProperty() {
#[cfg(not(feature = "WEBAPP"))]{
    return;
}
    result = eval("getExternRefPropertyValue($canvas,'width')"); // ok!!
    eq!(result.value.longy, 300); // only works because String "300" gets converted to BigInt 300
    //	result = eval("width='width';$canvas.width");
    result = eval("$canvas.width");
    assert!_eq(result.value.longy, 300);
    //	return;
    result = eval("$canvas.style");
    eq!(result.kind, strings);
    //	eq!(result.kind, stringp);
    if (result.value.string)
        assert!_eq(*result.value.string, "dfsa");
    //	getExternRefPropertyValue OK  [object HTMLCanvasElement] style [object CSSStyleDeclaration]
    // ⚠️ But can't forward result as smarti or stringref:  SyntaxError: Failed to parse String to BigInt
    // todo : how to communicate new string as RETURN type of arbitrary function from js to wasp?
    // call Webview.getString(); ?

    //	embedder.trace('canvas = document.getElementById("canvas");')
    //	print(nod);
}
#[test] fn testTypesSimple() {
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

#[test] fn testTypesSimple2() {
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
#[test] fn testTypedFunctions() {
    // todo name 'id' clashes with 'id' in preRegisterFunctions()
    clearAnalyzerContext();
    result = analyze(parse("int tee(float b, string c){b}"));
    eq!(result.kind, Kind::declaration);
    eq!(result.name, "tee");
    let signature_node = result["@signature"];
    //	let signature_node = result.metas()["signature"];
    if (not signature_node.value.data)
        error("no signature");
    Signature &signature = *(Signature *) signature_node.value.data;
    eq!(signature.functions.first()->name, "tee")
    eq!(signature.parameters.size(), 2)
    eq!(signature.parameters.first().name, "b")
    eq!(signature.parameters.first().type, reals); // use real / number for float64  float32
    eq!(signature.parameters.last().name, "c")
    eq!(signature.parameters.last().type, strings);
    // let params = signature.parameters.map(+[](Arg f) { return f.name; });
    // eq!(params.first(), "b");
}

#[test] fn testEmptyTypedFunctions() {
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
    Signature signature = *(Signature *) signature_node.value.data;
    eq!(signature.functions.first()->name, "a")
    let names2 = signature.functions.map<String>(+[](Function *f) { return f->name; });
    eq!(names2.size(), 1);
    eq!(names2.first(), "a");

    result = analyze(parse("int a();"));
    eq!(result.kind, Kind::declaration); // header signature
    eq!(result.type, IntegerType);
    eq!(result.name, "a");
}

#[test] fn testTypes() {
    testBadType();
    testDeepType();
    testTypedFunctions();
    testTypesSimple();
    testTypeConfusion();
    skip(
        testTypesSimple2();
        testEmptyTypedFunctions();
    )
}

#[test] fn testPolymorphism() {
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

#[test] fn testPolymorphism2() {
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
#[test] fn testPolymorphism3() {
    is!("fun test(string a){return a};\nfun test(float b){return b+1};\ntest('ok')", "ok");
    is!("fun test(string a){return a};\nfun test(int a){return a};\nfun test(float b){return b+1};\ntest(1.0)",
                2.0);
}

#[test] fn testModifiers() {
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

#[test] fn testGenerics() {
    let type = Type(Generics{.kind = array, .value_type = int16t});
    //    let header= type.value & array;
    //    let header= type.value & 0xFFFF0000; // todo <<
    let header = type.value & 0x0000FFFF; //todo ??
    assert!_eq(header, array);
}

#[test] fn testNumbers() {
    Number n = 1; // as comfortable BigInt Object used inside wasp
    assert!(n == 1.0);
    assert!(n / 2 == 0.5);
    assert!(((n * 2) ^ 10) == 1024);
}
#[test] fn testFunctionArgumentCast() {
    is!("float addi(int x,int y){x+y};'hello'+5", "hello5")
    is!("float addi(int x,int y){x+y};'hello'+5.9", "hello5.9")
    is!("float addi(int x,int y){x+y};'hello'+addi(2.2,2.2)", "hello4.")
    is!("float addi(int x,int y){x+y};'hello'+addi(2,3)", "hello5.") // OK some float cast going on!

    is!("fun addier(a,b){b+a};addier(42.0,1.0)", 43);
    is!("fun addier(int a,int b){b+a};addier(42,1)+1", 44);
    is!("fun addi(int x,int y){x+y};addi(2.2,2.2)", 4)
    is!("fun addi(float x,float y){x+y};addi(2.2,2.2)", 4.4)
    is!("float addi(int x,int y){x+y};addi(2.2,2.2)", 4.4)
    is!("fun addier(float a,float b){b+a};addier(42,1)+1", 44);
}

#[test] fn testFunctionDeclaration() {
    // THESE NEVER WORKED! should they? YES! partly
    // 'fixing' one broke fib etc :(
    // 💡we already have a working syntax so this has low priority
    // ⚠️ DO we really have a working syntax??
    skip( // TODO!
        testFunctionParams(); // TODO!
        testFibonacci(); // much TODO!
        is!("fun x{42} x+1", 43);
        is!("def x{42};x+1", 43);
        is!("def x(){42};x+1", 43);
        is!("def x(){42};x()+1", 43);
        is!("define x={42};x()+1", 43);
        is!("function x(){42};x()+1", 43);
        is!("def x(a){42+a};x(1)+1", 44);
        is!("define x={42+it};x(1)+1", 44);
        is!("function x(a){42+a};x(1)+1", 44);
        is!("function x(){42+it};x(1)+1", 44);
        is!("def x(a=3){42+a};x+1", 46); // default value
        is!("def x(a){42+a};x+1", 43);
    )
}

#[test] fn testFunctionDeclarationParse() {
    //    let node1 = analyze(parse("fn main(){}"));
    //    assert!(node1.kind==declaration);
    //    assert!(node1.name=="main");
    clearAnalyzerContext();
    // let node2 = analyze(parse("fun test(float a):int{return a*2}")); // todo: cast return to int and parseDeclaration!
    let node2 = analyze(parse("fun test(float a){return a*2}"));
    assert!(node2.kind == declaration);
    assert!(node2.name == "test");
    eq!(functions["test"].signature.size(), 1);
    eq!(functions["test"].signature.parameters[0].name, "a");
    eq!(functions["test"].signature.parameters[0].type, (Type) floats);
    // eq!(functions["test"].signature.parameters[0].type, (Type) reals); // upgrade float to real TODO not if explicit!
    assert!(functions["test"].body);
    assert!(not(*functions["test"].body != analyze(parse("return a*2"))));
    skip(
        assert!(*functions["test"].body == analyze(parse("return a*2"))); // why != ok but == not?
        eq!(*functions["test"].body, analyze(parse("return a*2")));
    )
}

#[test] fn testRenameWasmFunction() {
    Module &module1 = loadModule("samples/test.wasm");
    module1.functions.at(0).name = "test";
    module1.save("samples/test2.wasm");
    // todo: assert! by loadModule("samples/test2.wasm");
}

#[test] fn testPower() {
    eq!(powi(10, 1), 10l);
    eq!(powi(10, 2), 100l);
    eq!(powi(10, 3), 1000l);
    eq!(powi(10, 4), 10000l);
    eq!(parseLong("8e6"), 8000000l);
    skip(
        eq!(parseLong("8e-6"), 1.0 / 8000000l);
    )
    eq!(parseDouble("8.333e-3"), 0.008333l);
    eq!(parseDouble("8.333e3"), 8333.0l);
    eq!(parseDouble("8.333e-3"), 0.008333l);
    //    eq!(ftoa(8.33333333332248946124e-03), "0.0083");
    eq!(powi(10, 1), 10l);
    eq!(powi(10, 2), 100l);
    eq!(powi(10, 4), 10000l);
    eq!(powi(2, 2), 4l);
    eq!(powi(2, 8), 256l);
    skip(
        eq!(powd(2, -2), 1 / 4.);
        eq!(powd(2, -8), 1 / 256.);
        eq!(powd(10, -2), 1 / 100.);
        eq!(powd(10, -4), 1 / 10000.);
        eq!(powd(3,0), 1.);
        eq!(powd(3,1), 3.);
        eq!(powd(3,2), 9.);
        eq!(powd(3,2.1), 10.04510856630514);

        //==============================================================================
        // MAP TESTS (see map_tests.h)
        //==============================================================================

        eq!(powd(3.1,2.1), 10.761171606099687);
    )
    // is!("√3^0", 0.9710078239440918); // very rough power approximation from where?
}

#[test] fn testMaps0() {
    Map<int, long> map;
    assert!(map.values[0] == map[0]);
    assert!(map.values == &(map[0]));
    map[0] = 2;
    assert!(map.values[0] == 2);
    assert!(map.size() == 1);
    map[2] = 4;
    assert!(map.size() == 2);
    assert!(map.values[1] == 4);
    assert!(map.keys[1] == 2);
    print((int) map[0]);
    print((int) map[2]);
    print(map[(size_t) 0]);
    print(map[(size_t) 1]);
    assert!(map[0] == 2);
    assert!(map[2] == 4);
}
#[test] fn testMapOfStrings() {
    Map<String, chars> map;
    map["a"] = "1";
    assert!(map.size() == 1);
    map["a"] = "1";
    assert!(map.size() == 1);
    assert!(map.keys[0] == "a");
    assert!(map.values[0] == "1"s);
    assert!(map["a"] == "1"s);
    //    assert!(!map.has("b"));
    assert!(map.position("b") == -1);
    map["b"] = "2";
    assert!(map.size() == 2);
    assert!(map.keys[1] == "b");
    assert!(map.values[1] == "2"s);
    assert!(map["b"] == "2"s);
}

#[test] fn testMapOfStringValues() {
    Map<chars, String> map;
    map["a"] = "1";
    assert!(map.size() == 1);
    assert!(map.keys[0] == "a"s);
    assert!(map.values[0] == "1");
    assert!(map["a"] == "1");
    map["b"] = "2";
    assert!(map.size() == 2);
    assert!(map.keys[1] == "b"s);
    assert!(map.values[1] == "2");
    assert!(map["b"] == "2");
}

#[test] fn testMaps1() {
    functions.clear();
    functions.insert_or_assign("abcd", {.name = "abcd"});
    functions.insert_or_assign("efg", {.name = "efg"});
    eq!(functions.size(), 2);
    assert!(functions["abcd"].name == "abcd");
    assert!(functions["efg"].name == "efg");
}

#[test] fn testMaps2() {
    functions.clear();
    Function abcd;
    abcd.name = "abcd";
    functions["abcd"] = abcd;
    functions["efg"] = {.name = "efg"};
    functions["abcd"] = {.name = "abcd"};
    functions["efg"] = {.name = "efg"};
    eq!(functions.size(), 2);
    print(functions["abcd"]);
    print(functions["abcd"].name);
    assert!(functions["abcd"].name == "abcd");
    assert!(functions["efg"].name == "efg");
}

#[test] fn testMaps() {
    testMaps0(); // ok
    testMapOfStrings();
    testMapOfStringValues();
    testMaps1();
    testMaps2(); // now ok
}
#[test] fn testHex() {
    eq!(hex(18966001896603L), "0x113fddce4c9b");
    assert_is("42", 42);
    assert_is("0xFF", 255);
    assert_is("0x100", 256);
    assert_is("0xdce4c9b", 0xdce4c9b);
    //    assert_is("0x113fddce4c9b", 0x113fddce4c9bl); todo
    //	assert_is("0x113fddce4c9b", 0x113fddce4c9bL);
}

#[test] fn test_fd_write() {
    // built-in wasi function
    //    is!("x='hello';fd_write(1,20,1,8)", (int64) 0);// 20 = &x+4 {char*,len}
    //    is!("puts 'ok';proc_exit(1)\nputs 'no'", (int64) 0);
    //    is!("quit",0);
    is!("x='hello';fd_write(1,x,1,8)", (int64) 0); // &x+4 {char*,len}
    //    is!("len('123')", 3); // Map::len
    //    quit()
    is!("puts 'ok'", (int64) 0); // connect to wasi fd_write
    loadModule("wasp");
    is!("puts 'ok'", (int64) 0);
    is!("puti 56", 56);
    is!("putl 56", 56);
    //    is!("putx 56", 56);
    is!("putf 3.1", 0);

    assert!(module_cache.has("wasp-runtime.wasm"s.hash()))
}

#[test] fn testEnumConversion() {
#[cfg(not(feature = "TRACE"))]{
    Valtype yy = (Valtype) Primitive::charp;
    int i = (int) Primitive::charp;
    int i1 = (int) yy; // CRASHES in Trace mode WHY?
    eq!(stackItemSize(Primitive::wasm_float64), 8);
    eq!(i, i1);
    assert!((Type) Primitive::charp == yy);
    assert!((Type) yy == Primitive::charp);
    assert!(Primitive::charp == (Type) yy);
    assert!(yy == (Type) Primitive::charp);
    assert!((int) yy == (int) Primitive::charp);
}
}

#[test] fn bindgen(Node &n) {
    //    todo parserOptions => formatOptions => Format ! inheritable
    //    todo interface host-funcs {current-user: func() -> string}
    print(n.serialize());
}

// https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-use
#[test] fn testUse() {
    parse("use * from other-file");
    parse("use { a, list, of, names } from another-file");
    parse("use { name as other-name } from yet-another-file");
    // MY SYNTAX :
    parse("use other-file"); // MY SYNTAX
    parse("use all from other-file"); // MY SYNTAX redundant
    parse("use name from yet-another-file"); // MY SYNTAX
    parse("use name from yet-another-file as other-name"); // MY SYNTAX
    //    parse("use name as other-name from yet-another-file");// MY SYNTAX
}

#[test] fn testClass() {
    analyze(parse("public data class Person(string FirstName, string LastName);"));
    analyze(parse("public data class Student : Person { int ID; }"));
    analyze(parse("var person = new Person('Scott', 'Hunter'); // positional construction"));
    analyze(parse("otherPerson = person with { LastName = \"Hanselman\" };"));
    //    "var (f, l) = person;                        // positional deconstruction"
}
#[test] fn test_c_numbers() {
    //    assert!(0x1000000000000000l==powi(2,60))
    unsigned int x = -1;
    unsigned int y = 0xFFFFFFFF;
    //    signed int biggest = 0x7FFFFFFF;
    //    signed int smallest = 0x80000000;// "implementation defined" so might not always pass
    signed int z = -1;
    assert!(x == y)
    assert!(x == z)
    assert!(z == y)
    assert!((int) -1 == (unsigned int) 0xFFFFFFFF)
}

#[test] fn testArraySize() {
    // todo!
    // There should be one-- and preferably only one --obvious way to do it.
    // requires struct lookup and aliases
    is!("pixel=[1 2 4];#pixel", 3);
    //  is!("pixel=[1 2 4];pixel#", 3);
    is!("pixel=[1 2 4];pixel size", 3);
    is!("pixel=[1 2 4];pixel length", 3);
    is!("pixel=[1 2 4];pixel count", 3);
    is!("pixel=[1 2 4];pixel number", 3); // ambivalence with type number!
    is!("pixel=[1 2 4];pixel.size", 3);
    is!("pixel=[1 2 4];pixel.length", 3);
    is!("pixel=[1 2 4];pixel.count", 3);
    is!("pixel=[1 2 4];pixel.number", 3); // ambivalence cast
    is!("pixels=[1 2 4];number of pixels ", 3);
    is!("pixels=[1 2 4];size of pixels ", 3);
    is!("pixels=[1 2 4];length of pixels ", 3);
    is!("pixels=[1 2 4];count of pixels ", 3);
    is!("pixel=[1 2 3];pixel.add(5);#pixel", 4);
}
#[test] fn testArrayOperations() {
    // todo!
    testArraySize();
    // todo 'do' notation to modify versus return different list!
    is!("pixel=[1 2 3];do add 4 to pixel; pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];y=pixel + 4; pixel", Node(1, 2, 3, 0));

    //        assert_throws("pixel=[1 2 3];pixel + 4;pixel");// unused non-mutating operation
    is!("pixels=[1 2 4];pixel#3", 4); // plural!
    is!("pixel=[1 2 3];pixel + [4]", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel + 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel<<4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];4>>pixel", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];add(pixel, 4)", Node(1, 2, 3, 4, 0)); // julia style
    is!("pixel=[1 2 3];add 4 to pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.add 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel add 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.add(4)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel insert 4", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4,-1)", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at end", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at -1", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];insert 4 at end of pixel", Node(1, 2, 3, 4, 0));
    is!("pixel=[1 2 3];pixel.insert(4,0)", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at 0", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at start", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at head", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel.insert 4 at beginning", Node(4, 1, 2, 3, 0));
    is!("pixels=[1 2 3];insert 4 at start of pixels", Node(4, 1, 2, 3, 0));
    is!("pixel=[1 2 3];pixel - [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel - 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];remove [3] from pixel", Node(1, 2, 0));
    is!("pixel=[1 2 3];remove 3 from pixel", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove(3)", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove(3)", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove 3", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove([3])", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel.remove [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove([3])", Node(1, 2, 0));
    is!("pixel=[1 2 3];pixel remove [3]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel.remove([3 4])", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel.remove [3 4]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove([3 4])", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove [3 4]", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove 3 4", Node(1, 2, 0));
    is!("pixel=[1 2 3 4];pixel remove (3 4)", Node(1, 2, 0));
    is!("pixels=[1 2 3 4];pixels without (3 4)", Node(1, 2, 0));
}

#[test] fn testArrayCreation() {
    //    skip(
    // todo create empty array
    is!("pixel=[];pixel[1]=15;pixel[1]", 15);
    is!("pixel=();pixel#1=15;pixel#1", 15); // diadic ternary operator
    is!("pixel array;pixel#1=15;pixel#1", 15);
    is!("pixel:int[100];pixel[1]=15;pixel[1]", 15);
    is!("pixel=int[100];pixel[1]=15;pixel[1]", 15); // todo wasp can't distinguish type ':' from value '=' OK?
    is!("pixel: 100 int;pixel[1]=15;pixel[1]", 15); // number times type = typed array
}

#[test] fn testIndexOffset() {
    is!("(2 4 3)[1]", 4);
    is!("(2 4 3)#2", 4);
    is!("y=(1 4 3)#2", 4);
    is!("y=(1 4 3)[1]", 4);
    assert_is("x=(1 4 3);x#2=5;x#2", 5);
    assert_is("x=(1 4 3);z=(9 8 7);x#2", 4);
    is!("x=(5 6 7);y=(1 4 3);y#2", 4);
    is!("x=(5 6 7);(1 4 3)#2", 4);
    skip(
        is!("y=(1 4 3);y[1]", 4); // CAN NOT WORK in data_mode because y[1] ≈ y:1 setter
        is!("x=(5 6 7);y=(1 4 3);y[1]", 4);
    )
    is!("(5 6 7);(2 4 3)[0]", 2);
    is!("x=(5 6 7);y=(1 4 3);y#2", 4);
    is!("(5 6 7);(1 4 3)#2", 4);
    is!("x=(5 6 7);(1 4 3)#2", 4);
    skip(
        is!("puts('ok');(1 4 3)#2", 4);
    )
    is!("x=0;while x++<11: nop;", 0);
    is!("i=10007;x=i%10000", 7);
    is!("k=(1,2,3);i=1;k#i=4;k#1", 4)
    is!("k=(1,2,3);i=1;k#i=4;k#1", 4)
    is!("maxi=3840*2160", 3840 * 2160);
    is!("i=10007;x=i%10000", 7);
    assert_is("x=(1 4 3);x#2=5;x#2", 5);
    assert_is("x=(1 4 3);x#2", 4);
}

#[test] fn testFlagSafety() {
    let code = "flags empty_flags{}; empty_flags mine = data_mode | space_brace;";
    assert_throws(code) // "data_mode not a member of empty_flags"s
    assert_throws("enum cant_combine{a;b}; a+b;");
    assert_throws("enum context_x{a;b};enum context_y{b;c};b;");
}
#[test] fn testFlags2() {
    // todo allow just parser-flags{…} in wasp > wit
    let code = r#"flags parser-flags{
        data_mode
        arrow
        space_brace
       }
       parser-flags my_flags = data_mode + space_brace
    "#;
    is!(code, 5) // 1+4
    clearAnalyzerContext();
    Node &parsed = parse(code, {.kebab_case = true});
    Node &node = analyze(parsed);
    assert!(types.has("parser-flags"))
    assert!(globals.has("data_mode"))
    assert!(globals.has("parser-flags.data_mode")) //
    Node &parserFlags = node.first();
    // todo AddressSanitizer:DEADLYSIGNAL why? lldb does'nt fail here
    assert!(parserFlags.name == "parser-flags")
    assert!(parserFlags.kind == flags)
    assert!(parserFlags.length == 3)
    assert!(parserFlags[1].name == "arrow")
    assert!(parserFlags[2].value.longy == 4)
    Node &instance = node.last();
    print(instance);
    assert!(instance.name == "my_flags")
    assert!(instance.type)
    assert!(instance.type->name == "parser-flags") // deduced!
    assert!(instance.kind == flags) // kind? not really type! todo?
    Node my_flags = instance.interpret();
    print(my_flags);
    assert!(my_flags.value.longy == 5) // 1+4 bit internal detail!
    skip(
        assert!(my_flags.values().serialize() == "data_mode + space_brace")
    )

    //    assert!(node.last().serialize() == "ParserOptions my_flags = data_mode | space_brace") // todo canonical type serialization!?
}
#[test] fn testFlags() {
    clearAnalyzerContext();
    Node &parsed = parse("flags abc{a b c}");
    backtrace_line();
    Node &node = analyze(parsed);
    assert!(node.name == "abc")
    assert!(node.kind == flags)
    assert!(node.length == 3);
    assert!(node[0].name == "a");
    eq!(typeName(node[0].kind), typeName(flag_entry));
    eq!(node[0].kind, flag_entry);
    assert!(node[0].kind == flag_entry);
    assert!(node[0].value.longy == 1);
    assert!(node[0].type);
    assert!(node[0].type == node);
    assert!(node[1].value.longy == 2);
    assert!(node[2].value.longy == 4);
}

#[test] fn testPattern() {
    result = parse("y[1]", ParserOptions{.data_mode = false});
    assert!(result[0].kind == patterns);
    assert!(result[0][0].kind == longs);
    assert!(result[0][0].value.longy == 1);
    //    is!("(2 4 3)[0]", 2);

    //==============================================================================
    // WIT/COMPONENT MODEL TESTS (see feature_tests.h)
    //==============================================================================
}

#[test] fn testWitInterface() {
    Node &mod = Node("host-funcs").setKind(modul).add(Node("current-user").setKind(functor).add(StringType));
    is!("interface host-funcs {current-user: func() -> string}", mod)
}

#[test] fn testWitExport() {
    const char *code = "struct point{x:int y:float}";
    Node &node = parse(code);
    bindgen(node);
}
#[test] fn testWitFunction() {
    //    funcDeclaration
    // a:b,c vs a:b, c:d

    is!("add: func(a: float32, b: float32) -> float32", 0);
    Module &mod = read_wasm("test.wasm");
    print(mod.import_count);
    eq!(mod.import_count, 1)
    eq!(Node().setKind(longs).serialize(), "0")
    eq!(mod.import_names, List<String>{"add"}); // or export names?
}

#[test] fn testWitImport() {
}

#[test] fn testEqualsBinding() {
    // colon closes with space, not semicolon !
    parse("a = float32, b: float32");
    assert!(result.length == 1);
    assert!(result["a"] == "float32");
    Node val;
    val.add(Node("float32"));
    val.add(Node("b").add(Node("float32")));
    eq!(result[0], val);
}

#[test] fn testColonImmediateBinding() {
    // colon closes with space, not semicolon !
    result = parse("a: float32, b: float32");
    assert!(result.length == 2);
    assert!(result["a"] == "float32");
    assert!(result[0] == Node("a").add(Node("float32")));
    assert!(result[1] == Node("b").add(Node("float32")));
}

#[test] fn testWit() {
    //    testWitFunction();
    //    testWitInterface();
    Node wit;
    wit = (new WitReader())->read("test/merge/world.wit");
    wit = (new WitReader())->read("samples/bug.wit");
    wit = (new WitReader())->read("test/merge/example_dep/index.wit");
    wit = (new WitReader())->read("test/merge/index.wit");
    wit = (new WitReader())->read("samples/wit/typenames.wit");
    wit = (new WitReader())->read("samples/wit/wasi_unstable.wit");
    //    assert!(wit.length > 0);
}

#[test] fn testHyphenUnits() {
    //     const char *code = "1900 - 2000 AD";// (easy with units)
    //     assert_analyze(code,"{kind=range type=AD value=(1900,2000)}");
    // todo how does Julia represent 10 ± 2 m/s ?
    assert_is("1900 - 2000 AD == 1950 AD ± 50", true);
    assert_is("1900 - 2000 cm == 1950 cm ± 50", true);
    assert_is("1900 - 2000 cm == 1950 ± 50 cm ", true);
}

#[test] fn testHypenVersusMinus() {
    // Needs variable register in parser.
    const char *code = "a=-1 b=2 b-a";
    is!(code, 3);
    // kebab case
    const char *data = "a-b:2 c-d:4 a-b";
    is!(data, 2);
    //    testHyphenUnits();

    //    Node &node = parse(data);
}

#[test] fn testKebabCase() {
    testHypenVersusMinus();
}
/*
0 0 1
64 40 2
8192 2000 3
1048576 100000 4
134217728 8000000 5
17179869184 400000000 6
2199023255552 20000000000 7
281474976710656 1000000000000 8
36028797018963968 80000000000000 9
0 0 1
-65 ffffffffffffffbf 2
-8193 ffffffffffffdfff 3
-1048577 ffffffffffefffff 4
-134217729 fffffffff7ffffff 5
-17179869185 fffffffbffffffff 6
-2199023255553 fffffdffffffffff 7
-281474976710657 fffeffffffffffff 8
-36028797018963969 ff7fffffffffffff 9
 */

// only test once, see lebByteSize for result
#[test] fn testLebByteSize() {
    assert!_eq(lebByteSize((int64) -17179869185 + 1), 5)
    assert!_eq(lebByteSize((int64) -17179869185), 6)
    assert!_eq(lebByteSize((int64) -17179869185 - 1), 6)
    short last = 1;
    for (int64 i = -63; i > -0x100000000000000l; --i) {
        //    for (int64 i = 0; i < 0x10000000000000l; ++i) {
        //    for (uint64 i = 0; i < 0x100000000000000; ++i) {
        short size = lebByteSize(i);
        if (size > last) {
            //            printf("%ld %lx %d\n", i, i, size);
            last = size;
            i = i * 128 + 129;
        }
    }
}

#[test] fn testListGrow() {
    // tested once, ok
    return;
    List<int> oh = {0, 1, 2, 3};
    for (int i = 4; i < 1000000000; ++i) {
        oh.add(i);
        unsigned int ix = random() % i;
        assert!_silent(oh[ix] == ix)
    }
    String aok = "ok";
    List<String> ja; // = {ok};
    ja.add(aok);
    String &o1 = ja[0];
    ja.grow();
    String &o3 = ja[0];
    assert!(o1.data == o3.data);
    o3.data = (char *) "hu";
    assert!(o1.data == o3.data);
}

#[test] fn testWasmRunner() {
    //	int result = run_wasm("test/test42.wasm");
    //	eq!(result, 42);
}

#[test] fn testLeaks() {
    int reruns = 0;
    //	int reruns = 100000;
    for (int i = 0; i < reruns; ++i) {
        //
        printf("\n\n    ===========================================\n%d\n\n\n", i);
        //		is!("i=-9;√-i", 3);// SIGKILL after about 3000 emits OK'ish ;)
        is!("i=-9;√-i", 3); // SIGKILL after about 120 runs … can be optimized ;)
    }
}

#[test] fn testWrong0Termination() {
#[cfg(not(feature = "WASM"))]{
    List<String> builtin_constants = {"pi", "π"};
    eq!(builtin_constants.size(), 2); // todo
}
}

#[test] fn testDeepColon() {
    result = parse("current-user: func() -> string");
    eq!(result.kind, key);
    eq!(result.values().name, "func");
    eq!(result.values().values().name, "string");
};

#[test] fn testDeepColon2() {
    result = parse("a:b:c:d");
    eq!(result.kind, key);
    eq!(result.values().name, "b");
    eq!(result.values().values().values().name, "d");
};
#[test] fn testStupidLongLong() {
    //	int a;
    //	long b;// 4 byte in wasm/windows grr
    //	long long c;// 8 bytes everywhere (still not guaranteed grr)
    //	int64 c;// 8 bytes everywhere (still not guaranteed grr)
    double b;
    float a;
    long double c; // float128 16 byte in wasm wow, don't use anyway;)
    print((int) sizeof(a));
    print((int) sizeof(b));
    print((int) sizeof(c)); // what? 16 bytes!?
}

#[test] fn testFloatReturnThroughMain() {
    double x = 0.0000001; // 3e...
    //	double x=1000000000.1;// 4...
    //	double x=-1000000000.1;// c1…
    //	double x=9999999999999999.99999999;// 43…
    //	double x=-9999999999999999.99999999;// c3…
    //	double x=1.1;// 3ff199999999999a
    //	double x=-1.1;// bff199999999999a
    int64 y = *(int64 *) &x;
#[cfg(not(feature = "WASM"))]{
    printf("%llx\n", y);
}
    y = 0x00FF000000000000; // -> 0.000000 OK
    x = *(double *) &y;
    printf("%lf\n", x);
}

#[test] fn testArrayS() {
    let node = analyze(parse("int"));
    //	eq!( node.type->kind, classe);
    eq!(node.kind, clazz);

    let node2 = analyze(parse("ints"));
    eq!(node2.kind, arrays); // type: array<int>

    node = parse("ints x");
    //	eq!( node.kind, reference);
    //	eq!( node.kind, arrays);
    eq!(node.kind, groups);
    eq!(node.type, &DoubleType);
}

#[test] fn testArrayInitialization() {
    // via Units
    is!("x : int[100]; x.length", 100)
    is!("x : u8 * 100; x.length", 100) // type times size operation!!
    is!("x : 100 * int; x.length", 100)
    is!("x : 100 * ints; x.length", 100)
    is!("x : 100 ints; x.length", 100) // implicit multiplication, no special case!
    is!("x : 100 int; x.length", 100)
    is!("x : 100 integers; x.length", 100)
    is!("x : 100 numbers; x.length", 100)
    is!("x is 100 times [0]; x.length", 100)
    is!("x is array of size 100; x.length", 100)
    is!("x is an 100 integer array; x.length", 100)
    is!("x is a 100 integer array; x.length", 100)
    is!("x is a 100 element array; x.length", 100)
}

#[test] fn testArrayInitializationBasics() {
    // via Units
    let node = analyze(parse("x : 100 numbers"));
    eq!(node.kind, arrays);
    eq!(node.length, 100);
}
#[test] fn test_sinus_wasp_import() {
    // using sin.wasp, not sin.wasm
    // todo: compile and reuse sin.wasm if unmodified
    is!("use sin;sin π/2", 1);
    is!("use sin;sin π", 0);
    is!("use sin;sin 3*π/2", -1);
    is!("use sin;sin 2π", 0);
    is!("use sin;sin -π/2", -1);
}

#[test] fn testIteration() {
    List<String> args;
    for (let x: args)
        error("NO ITEM, should'nt be reached "s + x);

    //#[cfg(not(feature = "WASM"))]{
    List<String> list = {"1", "2", "3"}; // wow! initializer_list now terminate!
    //	List<String> list = {"1", "2", "3", 0};
    int i = 0;
    for (let x: list) {
        i++;
        trace(x);
    }
    eq!(i, 3);

    //    Node items = {"1", "2", "3"};
    Node items = Node{"1", "2", "3"};
    i = 0;
    for (let x: list) {
        i++;
        trace(x);
    }
    eq!(i, 3);
    //}
}

//#[test] fn testLogarithmInRuntime(){
// 
//	float ℯ = 2.7182818284590;
//	//	eq!(ln(0),-∞);
//	eq!(log(100000),5.);
//	eq!(log(10),1.);
//	eq!(log(1),0.);
//	eq!(ln(ℯ*ℯ),2.);
//	eq!(ln(1),0.);
//	eq!(ln(ℯ),1.);
//}
//==============================================================================
// PARSER/SYNTAX TESTS (see parser_tests.h)
//==============================================================================

#[test] fn testUpperLowerCase() {
    //    is!("lowerCaseUTF('ÂÊÎÔÛ')", "âêîôû")

    char string[] = "ABC";
    lowerCase(string, 0);
    eq!(string, "abc");
    skip(
        char string[] = "ÄÖÜ";
        lowerCase(string, 0);
        eq!(string, "äöü");
        char string[] = "ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU"; // String literals are read only!
        lowerCase(string, 0);
        eq!(string, "âêîôû áéíóú àèìòù aöu");
        char *string2 = (char *) u8"ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU";
        lowerCase(string2, 0);
        eq!(string2, "âêîôû áéíóú àèìòù aöu");
        chars string3 = "ÂÊÎÔÛ ÁÉÍÓÚ ÀÈÌÒÙ AÖU";
    )
    //	g_utf8_strup(string);
}

#[test] fn testPrimitiveTypes() {
    is!("double 2", 2);
    is!("float 2", 2);
    is!("int 2", 2);
    is!("long 2", 2);
    is!("8.33333333332248946124e-03", 0);
    is!("8.33333333332248946124e+01", 83);
    is!("S1  = -1.6666", -1);
    is!("double S1  = -1.6666", -1);
    //	is!("double\n"
    //	            "\tS1  = -1.6666", -1);

    is!("grow(double z):=z*2;grow 5", 10);
    is!("grow(z):=z*2;grow 5", 10);
    is!("int grow(double z):=z*2;grow 5", 10);
    is!("double grow(z):=z*2;grow 5", 10);
    is!("int grow(int z):=z*2;grow 5", 10);
    is!("double grow(int z):=z*2;grow 5", 10);
    is!("double\n"
                "\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n"
                "\tS2  =  8.33333333332248946124e03, /* 0x3F811111, 0x1110F8A6 */\n\nS1", -16);
    is!("double\n"
                "\tS1  = -1.66666666666666324348e01, /* 0xBFC55555, 0x55555549 */\n"
                "\tS2  =  8.33333333332248946124e01, /* 0x3F811111, 0x1110F8A6 */\n\nS2", 83);
    eq!(ftoa(8.33333333332248946124e-03), "0.0083");
    //	eq!(ftoa2(8.33333333332248946124e-03), "8.333E-3");
    is!("S1 = -1.66666666666666324348e-01;S1*100", -16);
    is!("S1 = 8.33333333332248946124e-03;S1*1000", 8);
    skip(
        is!("(2,4) == (2,4)", 1); // todo: array creation/ comparison
        is!("(float 2, int 4.3)  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
        is!("float 2, int 4.3  == 2,4", 1); //  PRECEDENCE needs to be in valueNode :(
        //	float  2, ( int ==( 4.3 2)), 4
    )
}

// One of the few tests which can be removed because who will ever change the sin routine?
#[test] fn test_sin() {
#[cfg(feature = "LINUX")]{
    return; // only for internal sinus implementation testing
#else
    eq!(sin(0), 0.);
    eq!(sin(pi / 2), 1.);
    eq!(sin(-pi / 2), -1.);
    eq!(sin(pi), 0.);
    eq!(sin(2 * pi), 0.);
    eq!(sin(3 * pi / 2), -1.);

    eq!(cos(-pi / 2 + 0), 0.);
    eq!(cos(0), 1.);
    eq!(cos(-pi / 2 + pi), 0.);
    eq!(cos(-pi / 2 + 2 * pi), 0.);
    eq!(cos(pi), -1.);
    eq!(cos(-pi), -1.);
}
}
#[test] fn testModulo() {
    //	eq!(mod_d(10007.0, 10000.0), 7)
    is!("10007%10000", 7); // breaks here!?!
    is!("10007.0%10000", 7);
    is!("10007.0%10000.0", 7);

    is!("10007%10000.0", 7); // breaks here!?! load_lib mod_d suspect!!
    is!("i=10007;x=i%10000", 7);
    is!("i=10007.0;x=i%10000.0", 7); // breaks here!?!
    is!("i=10007.1;x=i%10000.1", 7);
}

#[test] fn testRepresentations() {
    result = parse("a{x:1}");
    let result2 = parse("a:{x:1}");
    eq!(result.kind, reference);
    eq!(result2.kind, key);
    //	a{x:1} ==
}

#[test] fn testDataMode() {
    result = parse("a b=c", ParserOptions{.data_mode = true});
    print(result);
    assert!(result.length == 2); // a, b:c

    result = parse("a b = c", ParserOptions{.data_mode = true});
    //    assert!(result.length == 1);// (a b):c
    print(result);

    result = parse("a b=c", ParserOptions{.data_mode = false});
    print(result);
    assert!(result.length == 4); // a b = c

    skip(
        result = analyze(result);
        print(result);
        assert!(result.length == 1); // todo  todo => (a b)=c => =( (a b) c)

        result = parse("<a href=link.html/>", ParserOptions{.data_mode = true, .use_tags = true});
        assert!(result.length == 1); // a(b=c)
    )
}

#[test] fn testSignificantWhitespace() {
    skip(testDataMode())
    result = parse("a b (c)");
    assert!(result.length == 3);
    result = parse("a b(c)");
    assert!(result.length == 2 or result.length == 1);
    result = parse("a b:c");
    assert!(result.length == 2); // a , b:c
    assert!(result.last().kind == key); // a , b:c
    result = parse("a: b c d", {.colon_immediate = false});
    assert!(result.length == 3);
    assert!(result.name == "a"); // "a"(b c d), NOT ((a:b) c d)
    assert!(result.kind == groups); // not key!
    result = parse("a b : c", {.colon_immediate = false});
    assert!(result.length == 1 or result.length == 2); // (a b):c
    eq!(result.kind, key);
    skip(
        assert(eval("1 + 1 == 2"));
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);

    )
    //1 + 1 ≠ 1 +1 == [1 1]
    //	assert_is("1 +1", parse("[1 1]"));
    skip(
        assert(eval("1 +1 == [1 1]"));
        assert_is("1 +1", Node(1, 1, 0));
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        assert(eval("1 +1 ≠ 1 + 1"));
    )
}
#[test] fn testComments() {
    let c = "blah a b c # to silence python warnings;)\n y/* yeah! */=0 // really";
    result = parse(c);
    assert!(result.length == 2);
    assert!(result[0].length == 4);
    assert!(result[1].length == 3);
}

#[test] fn testEmptyLineGrouping() {
    let indented = r#"
a:
  b
  c

d
e
	"#;
    let &groups = parse(indented);
    //	let &groups = parse("a:\n b\n c\n\nd\ne\n");
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.length == 2);
    assert!(parsed[1] == "c");
    assert!(parsed.name == "a");
}
//[[maybe_used]]
[[nodiscard("replace generates a new string to be consumed!")]]
//__attribute__((__warn_unused_result__))
int testNodiscard() {
    return 54;
}
#[test] fn testSerialize() {
    const char *inprint = "green=256*255";
    //	const char *inprint = "blue=255;green=256*255;maxi=3840*2160/2;init_graphics();surface=(1,2,3);i=10000;while(i<maxi){i++;surface#i=blue;};10";
    assertSerialize(inprint);
}
#[test] fn testDedent2() {
    let indented = r#"
a:
  b
  c
d
e
	"#;
    let &groups = parse(indented);
    //	let groups = parse("a:\n b\n c\nd\ne\n");
    print(groups.serialize());
    print(groups.length);
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.name == "a");
    assert!(parsed.length == 2);
    print(parsed[1]);
    assert!(parsed[1].name == "c");
}

#[test] fn testDedent() {
    let indented = r#"
a
  b
  c
d
e
    "#;
    let &groups = parse(indented);
    //	let groups = parse("a:\n b\n c\nd\ne\n");
    print(groups.serialize());
    print(groups.length);
    assert!(groups.length == 3); // a(),d,e
    let &parsed = groups.first();
    assert!(parsed.name == "a");
    assert!(parsed.length == 2);
    print(parsed[1]);
    assert!(parsed[1].name == "c");
}

/*
#[test] fn testWasmSpeed() {
 
	struct timeval stop, start;
	gettimeofday(&start, NULL);
	time_t s, e;
	time(&s);
	// todo: let compiler comprinte constant expressions like 1024*65536/4
	//out of bounds memory access if only one Memory page!
	//	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
	is!("i=0;k='hi';while(i<16777216){i++;k#i=65};k[1]", 65)// still slow, but < 1s
	//	is!("i=0;k='hi';while(i<16){i++;k#i=65};k[1]", 65)// still slow, but < 1s
	//	70 ms PURE C -O3   123 ms  PURE C -O1  475 ms in PURE C without optimization
	//  141 ms wasmtime very fast (similar to wasmer)
	//  150 ms wasmer very fast!
	//  546 ms in WebKit (todo: test V8/WebView2!)
	//	465 - 3511 ms in WASM3  VERY inconsistent, but ok, it's an interpreter!
	//	1687 ms wasmx (node.js)
	//  1000-3000 ms in wasm-micro-runtime :( MESSES with system clock! // wow, SLOWER HOW!?
	//	so we can never draw 4k by hand wow. but why? only GPU can do more than 20 frames per second
	//	sleep(1);
	gettimeofday(&stop, NULL);
	time(&e);

	printf("took %ld sec\n", e - s);
	printf("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);

	exit(0);
}*/

#[test] fn testImport42() {
    assert_is("import fourty_two", 42);
    assert_is("include fourty_two", 42);
    assert_is("require fourty_two", 42);
    assert_is("import fourty_two;ft*2", 42 * 2);
    assert_is("include fourty_two;ft*2", 42 * 2);
    assert_is("require fourty_two;ft*2", 42 * 2);
}

//
//#[test] fn testWaspInitializationIntegrity() {
// 
//	assert!(not contains(operator_list0, "‖"))// it's a grouper!
//}

#[test] fn testColonLists() {
    let parsed = parse("a: b c d", {.colon_immediate = false});
    assert!(parsed.length == 3);
    assert!(parsed[1] == "c");
    assert!(parsed.name == "a");
}
#[test] fn testModernCpp() {
    let aa = 1. * 2;
    printf("%f", aa); // lol
}

#[test] fn testDeepCopyBug() {
    chars source = "{c:{d:123}}";
    assert_parses(source);
    assert!(result["d"] == 123);
}
#[test] fn testDeepCopyDebugBugBug() {
    testDeepCopyBug();
    chars source = "{deep{a:3,b:4,c:{d:true}}}";
    assert_parses(source);
    assert!(result.name == "deep");
    result.print();
    Node &c = result["deep"]['c'];
    Node &node = c['d'];
    eq!(node.value.longy, (int64) 1);
    eq!(node, (int64) 1);
}

#[test] fn testDeepCopyDebugBugBug2() {
    //	chars source = "{deep{a:3,b:4,c:{d:123}}}";
    chars source = "{deep{c:{d:123}}}";
    assert_parses(source);
    Node &c = result["deep"]['c'];
    Node &node = c['d'];
    eq!(node.value.longy, (int64) 123);
    eq!(node, (int64) 123);
}
#[test] fn testNetBase() {
    warn("NETBASE OFFLINE");
    if (1 > 0)return;
    chars url = "http://de.netbase.pannous.com:8080/json/verbose/2";

    //==============================================================================
    // NETWORK/WEB TESTS (see web_tests.h)
    //==============================================================================

    //	print(url);
    chars json = fetch(url);
    //	print(json);
    result = parse(json);
    Node results = result["results"];
    //	Node Erde = results[0];// todo : EEEEK, let flatten can BACKFIRE! results=[{a b c}] results[0]={a b c}[0]=a !----
    Node Erde = results;
    assert(Erde.name == "Erde" or Erde["name"] == "Erde");
    Node &statements = Erde["statements"];
    assert(statements.length >= 1); // or statements.value.node->length >=
    assert(result["query"] == "2");
    assert(result["count"] == "1");
    assert(result["count"] == 1);

    //	skip(
    //			 );
    assert(Erde["name"] == "Erde");
    //	assert(Erde.name == "Erde");
    assert(Erde["id"] == 2); // todo : let numbers when?
    assert(Erde["kind"] == -104);
    //	assert(Erde.id==2);
}

#[test] fn testDivDeep() {
    Node div = parse("div{ span{ class:'bold' 'text'} br}");
    Node &node = div["span"];
    node.print();
    assert(div["span"].length == 2);
    assert(div["span"]["class"] == "bold");
}

#[test] fn testDivMark() {
    use_polish_notation = true;
    Node div = parse("{div {span class:'bold' 'text'} {br}}");
    Node &span = div["span"];
    span.print();
    assert(span.length == 2);
    assert(span["class"] == "bold");
    use_polish_notation = false;
}

#[test] fn testDiv() {
    result = parse("div{ class:'bold' 'text'}");
    result.print();
    assert(result.length == 2);
    assert(result["class"] == "bold");
    testDivDeep();
    skip(
        testDivMark();
    )
}

#[test] fn assert!Nil() {
    assert!(NIL.isNil());
    eq!(NIL.name.data, nil_name);
    assert!(nil_name == "nil"s); // WASM
    if (NIL.name.data == nil_name)
        eq!(NIL.name, nil_name);
#[cfg(not(feature = "LINUX"))]{ // WHY???
    assert!(NIL.name.data == nil_name);
}
    assert!(NIL.length == 0);
    assert!(NIL.children == 0);
    assert!(NIL.parent == 0);
    assert!(NIL.next == 0);
}

#[test] fn testMarkAsMap() {
    Node compare = Node();
    //	compare["d"] = Node();
    compare["b"] = 3;
    compare["a"] = "HIO";
    Node &dangling = compare["c"];
    assert!(dangling.isNil());
    assert!Nil();
    assert!(dangling == NIL);
    assert!(&dangling != &NIL); // not same pointer!
    dangling = Node(3);
    //	dangling = 3;
    assert!(dangling == 3);
    assert!(compare["c"] == 3);
    eq!(compare["c"], Node(3));
    Node &node = compare["a"];
    assert(node == "HIO");
    chars source = "{b:3 a:'HIO' c:3}"; // d:{}
    Node marked = parse(source);
    Node &node1 = marked["a"];
    assert(node1 == "HIO");
    assert!(compare["a"] == "HIO");
    assert!(marked["a"] == "HIO");
    assert(node1 == compare["a"]);
    assert(marked["a"] == compare["a"]);
    assert(marked["b"] == compare["b"]);
    assert(compare == marked);
}
#[test] fn testMarkSimple() {
    print("testMarkSimple");
    char xx[] = "1";
    Node x = assert_parses(xx);
    Node a = assert_parses("{aa:3}");
    eq!(a.value.longy, (int64) 3);
    eq!(a, int64(3));
    assert(a == 3);
    assert(a.kind == longs or a.kind == key and a.value.node->kind == longs);
    assert(a.name == "aa");
    //	assert(a3.name == "a"_s);// todo? cant
    Node &b = a["b"];
    a["b"] = a;
    assert(a["b"] == a);
    assert(a["b"] == b);
    assert(a["b"] == 3);

    assert(parse("3.") == 3.);
    assert(parse("3.") == 3.f);
    //	assert(Mark::parse("3.1") == 3.1); // todo epsilon 1/3≠0.33…
    //	assert(Mark::parse("3.1") == 3.1f);// todo epsilon
    result = parse("'hi'");
    assert!(result.kind == strings);
    assert!(*result.value.string == "hi");
    assert!(result == "hi");
    assert(parse("'hi'") == "hi");
    assert(parse("3") == 3);
}
// test only once to understand
#[test] fn testUTFinCPP() {
    char32_t wc[] = U"zß水🍌"; // or
    printf("%s", (char *) wc);

    //	char32_t wc2[] = "z\u00df\u6c34\U0001f34c";/* */ Initializing wide char array with non-wide string literal
    let wc2 = "z\u00df\u6c34\U0001f34c";
    printf("%s", wc2);

    //	let wc3 = "z\udf\u6c34\U1f34c";// not ok in cpp

    // char = byte % 128   char<0 => utf or something;)
    //	using namespace std;
#[cfg(not(feature = "WASM"))]{
    const char8_t str[9] = u8"عربى"; // wow, 9 bytes!
    printf("%s", (char *) str);
}
    const char str1[9] = "عربى";
    printf("%s", (char *) str1);
    assert!(eq((char *) str1, str1));
#[cfg(not(feature = "WASM"))]{
#[cfg(feature = "std")]{
    std::string x = "0☺2√";
    // 2009 :  std::string is a complete joke if you're looking for Unicode support
    let smile0 = x[1];
    char16_t smile1 = x[1];
    char32_t smile = x[1];
    //	assert!(smile == smile1);
}
}
    //	wstring_convert<codecvt_utf8<char32_t>, char32_t> wasm_condition;
    //	let str32 = wasm_condition.from_bytes(str);
    char16_t character = u'牛';
    char32_t hanzi = U'牛';
    wchar_t word = L'牛';
    printf("%c", character);
    printf("%c", hanzi);
    printf("%c", word);

    //	for(let c : str32)
    //		cout << uint_least32_t(c) << '\n';
    //		char a = '☹';// char (by definition) is one byte (WTF)
    //		char[10] a='☹';// NOPE
    chars a = "☹"; // OK
    byte *b = (byte *) a;
    assert!_eq(a[0], (char) -30); // '\xe2'
    assert!_eq(a[1], (char) -104); // '\x98'
    assert!_eq(a[2], (char) -71); // '\xb9'
    assert!_eq(b[0], (byte) 226); // '\xe2'
    assert!_eq(b[1], (byte) 152); // '\x98'
    assert!_eq(b[2], (byte) 185); // '\xb9'
    assert!_eq(b[3], (byte) 0); // '\0'
}

#[test] fn testUnicode_UTF16_UTF32() {
    // constructors/ conversion maybe later
    //	char letter = '牛';// Character too large for enclosing character literal type char ≈ byte
    char16_t character = u'牛';
    char32_t hanzi = U'牛';
    wchar_t word = L'牛';
    // assert!(hanzi == character);
    assert!(hanzi == word);
    //	use_interpreter=true
    // todo: let wasm return strings!
    assert(interpret("ç='a'") == String(u8'a'));
    assert(interpret("ç='☺'") == String(u'☺'));
    assert(interpret("ç='☺'") == String(L'☺'));
    assert(interpret("ç='☺'") == String(U'☺'));
    //	skip(
    assert(interpret("ç='☺'") == String(u"☺"));
    assert(interpret("ç='☺'") == String(u8"☺"));
    assert(interpret("ç='☺'") == String(U"☺"));
    // assert(interpret("ç='☺'") == String(L"☺"));
    //	)
    assert!(String(u'牛') == "牛");
    assert!(String(L'牛') == "牛");
    assert!(String(U'牛') == "牛");

    assert!(String(L'牛') == u'牛');
    assert!(String(L'牛') == U'牛');
    assert!(String(L'牛') == L'牛');
    assert!(String(U'牛') == u'牛');
    assert!(String(U'牛') == U'牛');
    assert!(String(U'牛') == "牛");
    assert!(String(U'牛') == L'牛');
    assert!(String(u'牛') == u'牛');
    assert!(String(u'牛') == U'牛');
    assert!(String(u'牛') == L'牛');
    assert!(String(u'牛') == "牛");
    assert!(String("牛") == u'牛');
    assert!(String("牛") == U'牛');
    assert!(String("牛") == L'牛');
    assert!(String("牛") == "牛");
    //	print(character);
    //	print(hanzi);
    //	print(word);
    print(sizeof(char32_t)); // 32 lol
    print(sizeof(wchar_t));

    assert_parses("ç='☺'");
    assert(interpret("ç='☺'") == "☺");

    assert_parses("ç=☺");
    assert(result == "☺" or result.kind == expression);
}

#[test] fn testStringReferenceReuse() {
    String x = "ab牛c";
    String x2 = String(x.data, false);
    assert!(x.data == x2.data);
    String x3 = x.substring(0, 2, true);
    assert!(x.data == x3.data);
    assert!(x.length >
        x3.length)
    // shared data but different length! assert! shared_reference when modifying it!! &text[1] doesn't work anyway;)
    assert!(x3 == "ab");
    print(x3);
    // todo("make sure all algorithms respect shared_reference and crucial length! especially print!");
}

//testUTFø  error: stray ‘\303’ in program
#[test] fn testUTF() {
    //    	testUTFinCPP();
    skip(testUnicode_UTF16_UTF32());
    assert!(utf8_byte_count(U'ç') == 2);
    assert!(utf8_byte_count(U'√') == 3);
    assert!(utf8_byte_count(U'🥲') == 4);
    assert!(is_operator(u'√')) // can't work because ☺==0xe2... too
    assert!(!is_operator(U'☺'))
    assert!(!is_operator(U'🥲'))
    assert!(not is_operator(U'ç'));
    assert!(is_operator(U'='));
    //	assert!(x[1]=="牛");
    assert!("a牛c"s.codepointAt(1) == U'牛');
    String x = "a牛c";
    codepoint i = x.codepointAt(1);
    assert!("牛"s == i);
#[cfg(not(feature = "WASM"))]{  // why??
    assert!("a牛c"s.codepointAt(1) == "牛"s);
    assert!(i == "牛"s); // owh wow it works reversed
}
    wchar_t word = L'牛';
    assert!(x.codepointAt(1) == word);

    assert_parses("{ç:☺}");
    assert(result["ç"] == "☺");

    assert_parses("ç:'☺'");
    skip(
        assert(result == "☺");
    )

    assert_parses("{ç:111}");
    assert(result["ç"] == 111);

    skip(
        assert_parses("ç='☺'");
        assert(eval("ç='☺'") == "☺");

        assert_parses("ç=☺");
        assert(result == "☺" or result.kind == expression);
    )
    //	assert(node == "ø"); //=> OK
}
#[test] fn testMarkMultiDeep() {
    // fragile:( problem :  c:{d:'hi'}} becomes c:'hi' because … bug
    chars source = "{deep{a:3,b:4,c:{d:'hi'}}}";
    assert_parses(source);
    Node &c = result["deep"]['c'];
    Node &node = result["deep"]['c']['d'];
    eq!(node, "hi");
    assert(node == "hi"_s);

    //==============================================================================
    // MARK DATA NOTATION TESTS (see parser_tests.h)
    //==============================================================================

    assert(node == "hi");
    assert(node == c['d']);
}

#[test] fn testMarkMulti() {
    chars source = "{a:'HIO' b:3}";
    assert_parses(source);
    Node &node = result['b'];
    print(result['a']);
    print(result['b']);
    assert(result["b"] == 3);
    assert(result['b'] == node);
}

#[test] fn testMarkMulti2() {
    assert_parses("a:'HIO' b:3  d:{}");
    assert(result["b"] == 3);
}

#[test] fn testOverwrite() {
    chars source = "{a:'HIO' b:3}";
    assert_parses(source);
    result["b"] = 4;
    assert(result["b"] == 4);
    assert(result['b'] == 4);
}

#[test] fn testAddField() {
    //	chars source = "{}";
    result["e"] = 42;
    assert(result["e"] == 42);
    assert(result['e'] == 42);
}

#[test] fn testErrors() {
    // use assert_throws
    throwing = true;
    // 0/0 now returns NaN (float division), not an error
    assert_throws("x"); // UNKNOWN local symbol 'x' in context main  OK
#[cfg(feature = "WASI")]{ or WASM
    skip("can't catch ERROR in wasm")
    return;
}
    assert_throws("]"); // set throwing to true!!
    // throwing = false; // error always throws
    // result = parse("]");
    // assert(result == ERROR);
    /*
        ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/out
        ln -s /Users/me/dev/apps/wasp/samples /Users/me/dev/apps/wasp/cmake-build-default/ #out/
      */
    // breakpoint_helper todo
    // result = /*Wasp::*/parseFile("samples/errors.wasp");
    // throwing = true;
}
#[test] fn testForEach() {
    int sum = 0;
    for (Node &item: parse("1 2 3"))
        sum += item.value.longy;
    assert(sum == 6);
}

#[cfg(not(feature = "WASM"))]{
#[cfg(not(feature = "WASI"))]{
#[cfg(feature = "APPLE")]{

using files = std::filesystem::recursive_directory_iterator;

#[test] fn testAllSamples() {
    // FILE NOT FOUND :
    //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-debug/
    // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/
    //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/
    // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/out wtf
    for (const let &file: files("samples/")) {
        if (!String(file.path().string().data()).contains("error"))
            Mark::/*Wasp::*/parseFile(file.path().string().data());
    }
}
}
}
}

#[test] fn testSample() {
    result = /*Wasp::*/parseFile("samples/comments.wasp");
}

#[test] fn testNewlineLists() {
    result = parse("  c: \"commas optional\"\n d: \"semicolons optional\"\n e: \"trailing comments\"");
    assert(result['d'] == "semicolons optional");
}

#[test] fn testKitchensink() {
    result = /*Wasp::*/parseFile("samples/kitchensink.wasp");
    result.print();
    assert(result['a'] == "classical json");
    assert(result['b'] == "quotes optional");
    assert(result['c'] == "commas optional");
    assert(result['d'] == "semicolons optional");
    assert(result['e'] == "trailing comments"); // trailing comments
    assert(result["f"] == /*inline comments*/ "inline comments");
}

#[test] fn testEval3() {
    let math = "one plus two";
    result = eval(math);
    assert(result == 3);
}
#[test] fn testDeepLists() {
    assert_parses("{a:1 name:'ok' x:[1,2,3]}");
    assert(result.length == 3);
    assert(result["x"].length == 3);
    assert(result["x"][2] == 3);
}

#[test] fn testIterate() {
    //	parse("(1 2 3)");
    Node empty;
    bool nothing = true;
    for (Node &child: empty) {
        nothing = false;
        child = ERROR;
    }
    assert!(nothing);
    Node liste = parse("{1 2 3}");
    liste.print();
    for (Node &child: liste) {
        // SHOULD effect result
        child.value.longy = child.value.longy + 10;
    }
    assert!(liste[0].value.longy == 11)
    for (Node child: liste) {
        // should NOT affect result
        child.value.longy = child.value.longy + 1;
    }
    assert!(liste[0].value.longy == 11)
}

#[test] fn testListInitializerList() {
    List<int> oks = {1, 2, 3}; // easy!
    assert!(oks.size_ == 3)
    assert!(oks[2] == 3)
}

#[test] fn testListVarargs() {
    testListInitializerList();
    // ^^ OK just use List<int> oks = {1, 2, 3};
    skip(
        const List<int> &list1 = List<int>(1, 2, 3, 0);
        if (list1.size_ != 3)
        breakpoint_helper
        assert!(list1.size_ == 3);
        assert!(list1[2] == 3);
    )
}
#[test] fn testLists() {
    testListVarargs(); //
    assert_parses("[1,2,3]");
    result.print();
    eq!(result.length, 3);
    eq!(result.kind, patterns);
    assert(result[2] == 3);
    assert(result[0] == 1);
    skip(
        assert(result[0] == "1"); // autocast
    )
    List<int> a = {1, 2, 3};
    List<int> b{1, 2, 3};
    List<short> c{1, 2, 3};
    List<short> d = {1, 2, 3};
    assert!_eq(a.size_, 3);
    assert!_eq(b.size_, 3);
    assert!_eq(a.size_, b.size_);
    assert!_eq(a[0], b[0]);
    assert!_eq(a[2], b[2]);
    assert!_eq(a, b);
    //    assert!_eq(a, c); // not comparable
    assert!_eq(c, d);
    //List<double> c{1, 2, 3};
    //List<float> d={1, 2, 3};

    //	assert_is("[1,2,3]",1);
}

#[test] fn testMapsAsLists() {
    assert_parses("{1,2,3}");
    assert_parses("{'a'\n'b'\n'c'}");
    assert_parses("{add x y}"); // expression?
    assert_parses("{'a' 'b' 'c'}"); // expression?
    assert_parses("{'a','b','c'}"); // list
    assert_parses("{'a';'b';'c'}"); // list
    assert(result.length == 3);
    assert(result[1] == "b");
    //	assert_is("[1,2,3]",1); what?
}
#[test] fn testLogic() {
    assert_is("true or false", true);
    assert_is("false or true", true);

    assert_is("not true", false);
    assert_is("not false", true); // fourth test fails regardles of complexity?

    assert_is("false or false", false);
    assert_is("true or false", true);
    assert_is("true or true", true);
    //==============================================================================
    // LOGIC/BOOLEAN TESTS (see angle_tests.h + feature_tests.h)
    //==============================================================================

    assert_is("true and true", true);
    assert_is("true and false", false);
    assert_is("false and true", false);
    assert_is("false and false", false);

    assert_is("false xor true", true);
    assert_is("true xor false", true);
    assert_is("false xor false", false);
    assert_is("true xor true", false);

    assert_is("¬ 1", 0);
    assert_is("¬ 0", 1);

    assert_is("0 ⋁ 0", 0);
    assert_is("0 ⋁ 1", 1);
    assert_is("1 ⋁ 0", 1);
    assert_is("1 ⋁ 1", 1);

    assert_is("0 ⊻ 0", 0);
    assert_is("0 ⊻ 1", 1);
    assert_is("1 ⊻ 0", 1);
    assert_is("1 ⊻ 1", 0);

    assert_is("1 ∧ 1", 1);
    assert_is("1 ∧ 0", 0);
    assert_is("0 ∧ 1", 0);
    assert_is("0 ∧ 0", 0);
}

// use the bool() function to determine if a value is truthy or falsy.
#[test] fn testTruthiness() {
    result = parse("true");
    //	print("TRUE:");
    nl();
    print(result.name);
    nl();
    print(result.value.longy);
    assert!(True.kind == bools);
    assert!(True.name == "True");
    assert!(True.value.longy == 1);
    assert_is("false", false);
    assert_is("true", true);
    //	assert!(True.value.longy == true);
    //	assert!(True.name == "true");
    //	assert!(True == true);
    assert_is("False", false);
    assert_is("True", true);
    assert_is("False", False);
    assert_is("True", True);
    assert_is("false", False);
    assert_is("true", True);
    assert_is("0", False);
    assert_is("1", True);
    skip(
        assert_is("ø", NIL);
    )
    assert_is("nil", NIL);
    assert_is("nil", False);
    assert_is("nil", false);
    assert_is("ø", false);
    skip(
        assert_is("2", true); // Truthiness != equality with 'true' !
        assert_is("2", True); // Truthiness != equality with 'True' !
        assert_is("{x:0}", true); // wow! falsey so deep?
        assert_is("[0]", true); // wow! falsey so deep?
    )
    assert_is("1", true);
    assert_is("{1}", true);
    skip(
        assert_is("{x:1}", true);
    )

    todo_emit( // UNKNOWN local symbol ‘x’ in context main OK
        assert_is("x", false);
        assert_is("{x}", false);
        assert_is("cat{}", false);
    )

    // empty referenceIndices are falsey! OK
}

#[test] fn testLogicEmptySet() {
    if (eval_via_emit) {
        print("todo eval_via_emit testLogicEmptySet …"); // todo
        return;
    }
    assert_is("not ()", true); // missing args for operator not
    assert_is("() xor 1", true);
    assert_is("1 xor ()", true);
    assert_is("() xor ()", false);
    assert_is("1 xor 1", false);
    assert_is("() or 1", true);
    assert_is("() or ()", false);
    assert_is("1 or ()", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and ()", false);
    assert_is("() and 1", false);
    assert_is("() and ()", false);

    assert_is("not 1", false);
    assert_is("{} xor 1", true);
    assert_is("1 xor {}", true);
    assert_is("{} xor {}", false);
    assert_is("1 xor 1", false);
    assert_is("{} or 1", true);
    assert_is("{} or {}", false);
    assert_is("1 or {}", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and {}", false);
    assert_is("{} and 1", false);
    assert_is("{} and {}", false);

    assert_is("not {}", true);
    assert_is("not 1", false);

    assert_is("[] or 1", true);
    assert_is("[] or []", false);
    assert_is("1 or []", true);
    assert_is("1 or 1", true);

    assert_is("1 and 0", false);
    assert_is("1 and []", false);
    assert_is("[] and 1", false);
    assert_is("[] and []", false);

    assert_is("not []", true);
    assert_is("not 1", false);
    assert_is("[] xor 1", true);
    assert_is("1 xor []", true);
    assert_is("[] xor []", false);
    assert_is("1 xor 1", false);
}
#[test] fn testLogicOperators() {
    assert_is("¬ 0", 1);
    assert_is("¬ 1", 0);

    assert_is("0 ⋁ 0", 0);
    assert_is("0 ⋁ 1", 1);
    assert_is("1 ⋁ 0", 1);
    assert_is("1 ⋁ 1", 1);

    assert_is("0 ⊻ 0", 0);
    assert_is("0 ⊻ 1", 1);
    assert_is("1 ⊻ 0", 1);
    assert_is("1 ⊻ 1", 0);

    assert_is("1 ∧ 1", 1);
    assert_is("1 ∧ 0", 0);
    assert_is("0 ∧ 1", 0);
    assert_is("0 ∧ 0", 0);
}
#[test] fn testLogic01() {
    assert_is("0 or 0", false);
    assert_is("0 or 1", true);
    assert_is("1 or 0", true);
    assert_is("1 or 1", true);

    assert_is("1 and 1", true);
    assert_is("1 and 0", false);
    assert_is("0 and 1", false);
    assert_is("0 and 0", false);

    // eor either or
    assert_is("0 xor 0", false);
    assert_is("0 xor 1", true);
    assert_is("1 xor 0", true);
    assert_is("1 xor 1", false);

    assert_is("not 0", true);
    assert_is("not 1", false);
}

#[test] fn testEqualities() {
    assert_is("1≠2", True);
    assert_is("1==2", False);
    //	assert_is("1=2", False);
    assert_is("1!=2", True);
    assert_is("1≠1", False);
    //	assert_is("2=2", True);
    assert_is("2==2", True);
    assert_is("2!=2", False);
}

// test once: not a test, just documentation
#[test] fn testBitField() {
    union mystruct {
        // bit fields
        struct {
            short Reserved1: 3;
            short WordErr: 1;
            short SyncErr: 1;
            short WordCntErr: 1;
            //            short Reserved2: 10;
        };

        short word_field;
    };
    assert!_eq(sizeof(mystruct), 2 /*bytes */);
    mystruct x;
    x.WordErr = true;
    assert!_eq(x.word_field, 8); // 2^^3
}

#[test] fn testCpp() {
    //    testBitField();
    //	esult of comparison of constant 3 with expression of type 'bool' is always true
    //	assert(1 < 2 < 3);// NOT WHAT YOU EXPECT!
    //	assert(3 > 2 > 1);// NOT WHAT YOU EXPECT!
    //	assert('a' < 'b' < 'c');// NOT WHAT YOU EXPECT!
    //	assert('a' < b and b < 'c');// ONLY WAY <<
}

#[test] fn testGraphSimple() {
    assert_parses("{  me {    name  } # Queries can have comments!\n}");
    assert(result.children[0].name == "name"); // result IS me !!
    assert(result["me"].children[0].name == "name"); // me.me = me good idea?
}
#[test] fn testGraphQlQueryBug() {
    let graphResult = "{friends: [ {name:x}, {name:y}]}";
    assert_parses(graphResult);
    Node &friends = result["friends"];
    assert(friends[0]["name"] == "x");
}

#[test] fn testGraphQlQuery() {
    let graphResult = "{\n  \"data\": {\n"
            "    \"hero\": {\n"
            "      \"id\": \"R2-D2\",\n"
            "      \"height\": 5.6430448,\n"
            "      \"friends\": [\n"
            "        {\n"
            "          \"name\": \"Luke Skywalker\"\n"
            "        },\n"
            "        {\n"
            "          \"name\": \"Han Solo\"\n"
            "        },\n"
            "      ]" /* todo \n nextNonWhite */
            "    }\n"
            "  }\n"
            "}";
    assert_parses(graphResult);
    result.print();
    Node &data = result["data"];
    data.print();
    Node &hero = data["hero"];
    hero.print();
    Node &height = data["hero"]["height"];
    height.print();
    Node &id = hero["id"];
    id.print();
    assert(id == "R2-D2");
    assert(height == 5.6430448);
    //	assert(height==5.643);
    Node &friends = result["data"]["hero"]["friends"];
    assert(friends[0]["name"] == "Luke Skywalker");
    //todo	assert(result["hero"] == result["data"]["hero"]);
    //	assert(result["hero"]["friends"][0]["name"] == "Luke Skywalker")// if 1-child, treat as root
}

#[test] fn testGraphQlQuery2() {
    assert_parses("{\n"
        "  human(id: \"1000\"){\n"
        "    name\n"
        "    height(unit: FOOT)\n"
        "  }\n"
        "}");
    assert(result["human"]["id"] == 1000);
    skip(assert(result["id"] == 1000, 0)); // if length==1 descend!
}

#[test] fn testGraphQlQuerySignificantWhitespace() {
    // human() {} != human(){}
    assert_parses("{\n"
        "  human(id: \"1000\") {\n"
        "    name\n"
        "    height(unit: FOOT)\n"
        "  }\n"
        "}");
    assert(result["human"]["id"] == 1000);
    skip(assert(result["id"] == 1000, 0)); // if length==1 descend!
}

#[test] fn testGraphParams() {
    assert_parses("{\n  empireHero: hero(episode: EMPIRE){\n    name\n  }\n"
        "  jediHero: hero(episode: JEDI){\n    name\n  }\n}");
    Node &hero = result["empireHero"];
    hero.print();
    assert(hero["episode"] == "EMPIRE");
    assert_parses("\nfragment comparisonFields on Character{\n"
        "  name\n  appearsIn\n  friends{\n    name\n  }\n }");
    assert_parses("\nfragment comparisonFields on Character{\n  name\n  appearsIn\n  friends{\n    name\n  }\n}")
    // VARIAblE: { "episode": "JEDI" }
    assert_parses("query HeroNameAndFriends($episode: Episode){\n"
        "  hero(episode: $episode){\n"
        "    name\n"
        "    friends{\n"
        "      name\n"
        "    }\n"
        "  }\n"
        "}")
}

#[test] fn testRootLists() {
    // vargs needs to be 0 terminated, otherwise pray!
    assert_is("1 2 3", Node(1, 2, 3, 0))
    assert_is("(1 2 3)", Node(1, 2, 3, 0))
    assert_is("(1,2,3)", Node(1, 2, 3, 0))
    assert_is("(1;2;3)", Node(1, 2, 3, 0))
    assert_is("1;2;3", Node(1, 2, 3, 0, 0)) //ok
    assert_is("1,2,3", Node(1, 2, 3, 0))
    assert_is("[1 2 3]", Node(1, 2, 3, 0).setKind(patterns))
    assert_is("[1 2 3]", Node(1, 2, 3, 0))
    assert_is("[1,2,3]", Node(1, 2, 3, 0))
    assert_is("[1,2,3]", Node(1, 2, 3, 0).setKind(patterns));
    assert_is("[1;2;3]", Node(1, 2, 3, 0))
    todo_emit( // todo ?
        assert_is("{1 2 3}", Node(1, 2, 3, 0))
        assert_is("{1,2,3}", Node(1, 2, 3, 0))
        assert_is("{1;2;3}", Node(1, 2, 3, 0))
    )
    todo_emit( // todo symbolic wasm
        assert_is("(a,b,c)", Node("a", "b", "c", 0))
        assert_is("(a;b;c)", Node("a", "b", "c", 0))
        assert_is("a;b;c", Node("a", "b", "c", 0))
        assert_is("a,b,c", Node("a", "b", "c", 0))
        assert_is("{a b c}", Node("a", "b", "c", 0))
        assert_is("{a,b,c}", Node("a", "b", "c", 0))
        assert_is("[a,b,c]", Node("a", "b", "c", 0))
        assert_is("(a b c)", Node("a", "b", "c", 0))
        assert_is("[a;b;c]", Node("a", "b", "c", 0))
        assert_is("a b c", Node("a", "b", "c", 0, 0))
        assert_is("{a;b;c}", Node("a", "b", "c", 0))
        assert_is("[a b c]", Node("a", "b", "c", 0))
    )
}
#[test] fn testRoots() {
    assert!(NIL.value.longy == 0);
    // assert_is((char *) "'hello'", "hello");
    skip(assert_is("hello", "hello", 0)); // todo reference==string really?
    assert_is("True", True)
    assert_is("False", False)
    assert_is("true", True)
    assert_is("false", False)
    assert_is("yes", True)
    assert_is("no", False)
    //	assert_is("right", True)
    //	assert_is("wrong", False)
    assert_is("null", NIL);
    assert_is("", NIL);
    assert!(NIL.value.longy == 0);
    assert_is("0", NIL);
    assert_is("1", 1)
    assert_is("123", 123)
    skip(
        assert_is("()", NIL);
        assert_is("{}", NIL); // NOP
    )
}
#[test] fn testParams() {
    //	eq!(parse("f(x)=x*x").param->first(),"x");
    //    data_mode = true; // todo ?
    Node body = assert_parses("body(style='blue'){a(link)}");
    assert(body["style"] == "blue");

    parse("a(x:1)");
    assert_parses("a(x:1)");
    assert_parses("a(x=1)");
    assert_parses("a{y=1}");
    assert_parses("a(x=1){y=1}");
    skip(assert_parses("a(1){1}", 0));
    skip(assert_parses("multi_body{1}{1}{1}", 0)); // why not generalize from the start?
    skip(assert_parses("chained_ops(1)(1)(1)", 0)); // why not generalize from the start?

    assert_parses("while(x<3){y:z}");
    skip(
        Node body2 = assert_parses(
            "body(style='blue'){style:green}"); // is that whole xml compatibility a good idea?
        skip(assert(body2["style"] ==
            "green", 0)); // body has prescedence over param, semantically param provide extra data to body
        assert(body2[".style"] == "blue");
    )
    //	assert_parses("a(href='#'){'a link'}");
    //	assert_parses("(markdown link)[www]");
}
#[test] fn testDidYouMeanAlias() {
    skip(
        Node ok1 = assert_parses("printf('hi')");
        eq!(ok1[".warnings"], "DYM print"); // THIS CAN NEVER HAVED WORKED! BUG IN TEST PIPELINE!
    )
}

#[test] fn testEmpty() {
    result = assert_parsesx("{  }");
    eq!_x(result.length, 0);
}

#[test] fn testEval() {
    skip(
        assert_is("√4", 2);
    )
}

#[test] fn testLengthOperator() {
    is!("#'0123'", 4); // todo at compile?
    is!("#[0 1 2 3]", 4);
    is!("#[a b c d]", 4);
    is!("len('0123')", 4); // todo at compile?
    is!("len([0 1 2 3])", 4);
    is!("size([a b c d])", 4);
    assert_is("#{a b c}", 3);
    assert_is("#(a b c)", 3); // todo: groups
}

#[test] fn testNodeName() {
    Node a = Node("xor"); // NOT type string by default!
    bool ok1 = a == "xor";
    assert!(a == "xor")
    assert!(a.name == "xor")
    assert!(ok1)
}

#[test] fn testIndentAsBlock() {
    todo_emit(

        //==============================================================================
        // NODE/DATA STRUCTURE TESTS (see node_tests.h)
        //==============================================================================

        assert_is((char *) "a\n\tb", "a{b}")
    )
    // 0x0E 	SO 	␎ 	^N 		Shift Out
    // 0x0F 	SI 	␏ 	^O 		Shift In
    //	indent/dedent  0xF03B looks like pause!?   0xF032…  it does, what's going on CLion? Using STSong!
    //	https://fontawesome.com/v4.7/icon/outdent looks more like it, also matching context  OK in font PingFang HK?
} // 􀖯􀉶𠿜🕻🗠🂿	𝄉

#[test] fn testParentContext() {
    chars source = "{a:'HIO' d:{} b:3 c:ø}";
    assert_parses(source);
    result.print();
    Node &a = result["a"];
    a.print();
    eq!(a.kind, strings);
    eq!(a.value.string, "HIO");
    eq!(a.string(), "HIO"); // keyNodes go to values!
    assert(a == "HIO");
    //	eq!(a.name,"a" or"HIO");// keyNodes go to values!
    skip(
        eq!(a.kind, key);
        assert(a.name == "HIO");
    )
}

#[test] fn testParent() {
    //	chars source = "{a:'HIO' d:{} b:3 c:ø}";
    chars source = "{a:'HIO'}";
    assert_parses(source);
    Node &a = result["a"];
    print(a);
    assert!(a.kind == key or a.kind == strings);
    assert!(a == "HIO");
    assert!(a.parent == 0); // key is the highest level
    Node *parent = a.value.node->parent;
    assert!(parent);
    print(parent); // BROKEN, WHY?? let's find out:
    assert!(*parent == result);
    skip(
        // pointer identity broken by flat() ?
        assert!(parent == &result);
    )
    testParentContext(); // make sure parsed correctly
}

#[test] fn testAsserts() {
    eq!(11, 11);
    eq!(11.1f, 11.1f);
    //	eq!(11.1l, 11.1);
    eq!((float) 11., (float) 11.);
    //	eq!((double)11., (double )11.);
    eq!("a", "a");
    eq!("a"_s, "a"_s);
    eq!("a"_s, "a");
    eq!(Node("a"), Node("a"));
    eq!(Node(1), Node(1));
}

#[test] fn testStringConcatenation() {
    //	eq!(Node("✔️"), True);
    //	eq!(Node("✔"), True);
    //	eq!(Node("✖️"), False);
    //	eq!(Node("✖"), False);
    String huh = "a"_s + 2;
    assert!_eq(huh.length, 2);
    assert!_eq(huh[0], 'a');
    assert!_eq(huh[1], '2');
    assert!_eq(huh[2], (int64) 0);
    assert!(eq("a2", "a2"));
    assert!(eq("a2", "a2", 3));

    eq!(huh, "a2");
    eq!("a"_s + 2, "a2");
    eq!("a"_s + 2.2, "a2.2");
    eq!("a"_s + "2.2", "a2.2");
    eq!("a"_s + 'b', "ab");
    eq!("a"_s + "bc", "abc");
    eq!("a"_s + true, "a✔️"_s);
    eq!("a%sb"_s % "hi", "ahib");

    eq!("a%db"_s % 123, "a123b");
    eq!("a%s%db"_s % "hi" % 123, "ahi123b");
}

#[test] fn testString() {
    String *a = new String("abc");
    String b = String("abc");
    String c = *a;
    print(a);
    print(b);
    print(c);
    printf("...");
    //    for (int i = 0; i < 1000; ++i) {
    //        puti(i);
    //        puts("… x y z");
    //        newline();
    //            assert!(c == "abc");
    //
    //        if (b == "abc");
    //        else assert!(b == "abc");
    //    }
    //    printf("DONE ...");
    //    exit(1);
    eq!(a, b);
    eq!(a, c);
    eq!(b, c);
    let d = "abc";
    print(a->data);
    print(d);
    assert!(eq(a->data, d));
    eq!(b, "abc");
    eq!(c, "abc");
    assert!_eq(b, "abc");
    assert!_eq(c, "abc");
    assert!(c == "abc");
    assert!(b == a);
    assert!(b == c);
    assert!("%d"s % 5 == "5");
    assert!("%s"s % "a" == "a");
    assert!("%s"s % "ja" == "ja");
    assert!("hi %s ok"s.replace("%s", "ja") == "hi ja ok");
    assert!("1234%d6789"s % 5 == "123456789");
    assert!("char %c"s % 'a' == "char a");
    assert!("%c %d"s % 'a' % 3 == "a 3");
    assert!("abc"s.replace("a", "d") == "dbc");
    assert!("hi %s ok"s % "ja" == "hi ja ok");
    assert!("%s %d"s % "hu" % 3 == "hu 3");
    assert!("%s %s %d"s % "ha" % "hu" % 3 == "ha hu 3");
    eq!("%c"s % u'γ', "γ");
    eq!("%C"s % U'γ', "γ");
    eq!(String("abcd").substring(1, 2, false), "b");
    eq!(String("abcd").substring(1, 3, false), "bc");
    eq!(String("abcd").substring(1, 2, true/*share*/), "b"); // excluding, like js
    eq!(String("abcd").substring(1, 3, true), "bc");
    eq!(String("abcd").substring(1, 3), "bc");
    eq!(String("abcd").substring(1, 2), "b");
    assert!("%s"s.replace("%s", "ja") == "ja");
    assert!("hi %s"s.replace("%s", "ja") == "hi ja");
    assert!("%s ok"s.replace("%s", "ja") == "ja ok");
    assert!("hi %s ok"s.replace("%s", "ja") == "hi ja ok");
    let x = "hi %s ok"s % "ja";
    assert!(x);
    printf("%s", x.data);
    assert!(x == "hi ja ok");
    assert!("hi %s ok"s % "ja" == "hi ja ok");
    eq!(atoi1('x'), -1);
    eq!(atoi1('3'), 3);
    eq!(parseLong("١٢٣"), 123l); // Arabic numerals are (LTR) too!
    assert!_eq(parseLong("123"), 123l); // can crash!?!
    //	eq!( atoi1(u'₃'),3);// op
    eq!(parseLong("0"), 0l);
    eq!(parseLong("x"), 0l); // todo side channel?
    eq!(parseLong("3"), 3l);
    assert!_eq(" a b c  \n"s.trim(), "a b c");
    eq!("     \n   malloc"s.trim(), "malloc");
    eq!("     \n   malloc     \n   "s.trim(), "malloc");
    eq!("malloc     \n   "s.trim(), "malloc");
    testStringConcatenation();
    testStringReferenceReuse();
    eq!_x(parse("١٢٣"), Node(123));
    //    assert_is("١٢٣", 123);
    assert!("abc"_ == "abc");

    assert!(String(u'☺').length == 3)
    assert!(String(L'☺').length == 3)
    assert!(String(U'☺').length == 3)

    let node1 = interpret("ç='☺'");
    assert!(node1.kind == strings);
    assert!(*node1.value.string == u'☺');
    assert!(*node1.value.string == u'☺');
    assert(node1 == String(u'☺'));
    assert(node1 == String(L'☺'));
    assert(node1 == String(U'☺'));
}
#[test] fn testNilValues() {
#[cfg(feature = "LINUX")]{
    return; // todo: not working on linux, why?
}
    assert(NIL.name == nil_name.data);
    assert(NIL.isNil());
    assert_parses("{ç:null}");
    Node &node1 = result["ç"];
    debugNode(node1);
    assert(node1 == NIL);

    assert_parses("{a:null}");
    assert!(result["a"].value.data == 0)
    assert!(result.value.data == 0)
    assert!(result["a"].value.longy == 0)
    assert!(result.value.longy == 0)
    debugNode(result["a"]);
    print(result["a"].serialize());
    assert(result["a"] == NIL);
    assert(result == NIL);
    eq!(result["a"], NIL);

    assert_parses("{ç:ø}");
    Node &node = result["ç"];
    assert(node == NIL);
}
#[test] fn testConcatenationBorderCases() {
    eq!(Node(1, 0) + Node(3, 0), Node(1, 3, 0)); // ok
    eq!(Node("1", 0, 0) + Node("2", 0, 0), Node("1", "2", 0));
    // Border cases: {1}==1;
    eq!(parse("{1}"), parse("1"));
    // Todo Edge case a=[] a+=1
    eq!(Node() + Node("1", 0, 0), Node("1", 0, 0));
    //  singleton {1}+2==1+2 = 12/3 should be {1,2}
    eq!(Node("1", 0, 0) + Node("x"_s), Node("1", "x", 0));
}

#[test] fn testConcatenation() {
    Node node1 = Node("1", "2", "3", 0);
    assert!(node1.length == 3);
    assert!(node1.last() == "3");
    assert!(node1.kind == objects);
    Node other = Node("4").setKind(strings); // necessary: Node("x") == reference|strings? => kind=unknown
    assert!(other.kind == strings);
    assert!(!other.isNil());
    assert!(!(&other == &NIL));
    //	address of 'NIL' will always evaluate to 'true' because NIL is const now!
    //	assert!(!(other == &NIL));
    //	assert!(not(&other == &NIL));
    //	assert!(not(other == &NIL));
    assert!(other != NIL);
#[cfg(not(feature = "WASM"))]{
    //	assert!(other != &NIL);
}
    assert!(&other != &NIL);
    assert!(not other.isNil());
    Node node1234 = node1.merge(other);
    //	Node node1234=node1.merge(Node("4"));
    //	Node node1234=node1.merge(new Node("4"));
    Node *four = new Node("4");
    node1.add(four);
    //	node1=node1 + Node("4");
    assert!_eq(node1.length, 4);
    assert!(node1.last() == "4");
    //	assert!(&node1234.last() == four); not true, copied!
    assert!(node1234.last() == four);
    assert!(*four == "4");
    node1234.print();

    assert!_eq(node1234.length, 4);

    node1234.children[node1234.length - 2].print();
    node1234.children[node1234.length - 1].print();
    node1234.last().print();
    assert!(node1234.last() == "4");

    eq!(node1, Node("1", "2", "3", "4", 0));
    Node first = Node(1, 2, 0);
    assert!_eq(first.length, 2);
    assert!_eq(first.kind, objects);
    result = first + Node(3);
    assert!_eq(result.length, 3);
    assert!(result.last() == 3);

    eq!(Node(1, 2, 0) + Node(3), Node(1, 2, 3, 0));
    eq!(Node(1, 2, 0) + Node(3, 4, 0), Node(1, 2, 3, 4, 0));
    eq!(Node("1", "2", 0) + Node("3", "4", 0), Node("1", "2", "3", "4", 0));
    eq!(Node(1) + Node(2), Node(3));
    eq!(Node(1) + Node(2.4), Node(3.4));
    eq!(Node(1.0) + Node(2), Node(3.0));

    skip(
        eq!(Node(1) + Node("a"_s), Node("1a"));
        Node bug = Node("1"_s) + Node(2);
        // AMBIGUOUS: "1" + 2 == ["1" 2] ?
        eq!(Node("1"_s) + Node(2), Node("12"));
        eq!(Node("a"_s) + Node(2.2), Node("a2.2"));
        // "3" is type unknown => it is treated as NIL and not added!
        eq!(Node("1", "2", 0) + Node("3"), Node("1", "2", "3", 0)); // can't work ^^
    )
}
#[test] fn testParamizedKeys() {
    //	<label for="pwd">Password</label>

    // 0. parameters accessible
    Node label0 = parse("label(for:password)");
    label0.print();
    Node &node = label0["for"];
    eq!(node, "password");
    eq!(label0["for"], "password");

    // 1. paramize keys: label{param=(for:password)}:"Text"
    Node label1 = parse("label(for:password):'Passwort'"); // declaration syntax :(
    // Node label1 = parse("label{for:password}:'Passwort'");
    // Node label1 = parse("label[for:password]:'Passwort'");
    label1.print();
    eq!(label1, "Passwort");
    eq!(label1["for"], "password");
    //	eq!(label1["for:password"],"Passwort");

    // 2. paramize values
    // TODO 1. move params of Passwort up to lable   OR 2. preserve Passwort as object in stead of making it string value of label!
    skip(
        Node label2 = parse("label:'Passwort'(for:password)");
        assert!(label2 == "Passwort");
        eq!(label2, "Passwort");
        eq!(label2["for"], "password");
        eq!(label2["for"], "password"); // descend value??
        eq!(label2["Passwort"]["for"], "password");
    )

    skip(
        //	3. relative equivalence? todo not really
        eq!(label1, label2);
        Node label3 = parse("label:{for:password 'Password'}");
    )
}

#[test] fn testStackedLambdas() {
    result = parse("a{x:1}{y:2}{3}");
    result.print();
    assert!(result.length == 3);
    assert!(result[0] == parse("{x:1}"));
    assert!(result[0] == parse("x:1")); // grouping irrelevant
    assert!(result[1] == parse("{y:2}"));
    assert!(result[2] == parse("{3}"));
    assert!(result[2] != parse("{4}"));

    assert!(parse("a{x}{y z}") != parse("a{x,{y z}}"));
}

#[test] fn testIndex() {
    assert_parses("[a b c]#2");
    result.print();
    assert!(result.length == 3);
    skip(
        assert_is("(a b c)#2", "b");
        assert_is("{a b c}#2", "b");
        assert_is("[a b c]#2", "b");
    )
    todo_emit(
        assert_is("{a:1 b:2}.a", 1)
        assert_is("a of {a:1 b:2}", 1)
        assert_is("a in {a:1 b:2}", 1)
        assert_is("{a:1 b:2}[a]", 1)
        assert_is("{a:1 b:2}.b", 2)
        assert_is("b of {a:1 b:2}", 2)
        assert_is("b in {a:1 b:2}", 2)
        assert_is("{a:1 b:2}[b]", 2)
    )

    //==============================================================================
    // ADVANCED TESTS (see various)
    //==============================================================================
}

// can be removed because noone touches List.sort algorithm!
#[test] fn testSort() {
#[cfg(not(feature = "WASM"))]{
    List<int> list = {3, 1, 2, 5, 4};
    List<int> listb = {1, 2, 3, 4, 5};
    assert!(list.sort() == listb)
    let by_precedence = [](int &a, int &b) { return a * a > b * b; };
    assert!(list.sort(by_precedence) == listb)
    let by_square = [](int &a) { return (float) a * a; };
    assert!(list.sort(by_square) == listb)
}
}

#[test] fn testSort1() {
#[cfg(not(feature = "WASM"))]{
    List<int> list = {3, 1, 2, 5, 4};
    List<int> listb = {1, 2, 3, 4, 5};
    let by_precedence = [](int &a, int &b) { return a * a > b * b; };
    assert!(list.sort(by_precedence) == listb)
}
}

#[test] fn testSort2() {
#[cfg(not(feature = "WASM"))]{
    List<int> list = {3, 1, 2, 5, 4};
    List<int> listb = {1, 2, 3, 4, 5};
    let by_square = [](int &a) { return (float) a * a; };
    assert!(list.sort(by_square) == listb)
}
}

#[test] fn testRemove() {
    result = parse("a b c d");
    result.remove(1, 2);
    Node replaced = parse("a d");
    assert!(result == replaced);
}

#[test] fn testRemove2() {
    result = parse("a b c d");
    result.remove(2, 10);
    Node replaced = parse("a b");
    assert!(result == replaced);
}

#[test] fn testReplace() {
    result = parse("a b c d");
    result.replace(1, 2, new Node("x"));
    Node replaced = parse("a x d");
    assert!(result == replaced);
}
#[test] fn testNodeConversions() {
    Node b = Node(true);
    print("b.kind");
    print(b.kind);
    print(typeName(b.kind));
    print("b.value.longy");
    print(b.value.longy);
    assert!(b.value.longy == 1);
    assert!(b.kind == bools);
    assert!(b == True);
    Node a = Node(1);
    assert!(a.kind == longs);
    assert!(a.value.longy == 1);
    Node a0 = Node((int64_t) 10ll);
    assert!(a0.kind == longs);
    assert!(a0.value.longy == 10);
    Node a1 = Node(1.1);
    assert!_eq(a1.kind, reals);
    assert!(a1.kind == reals);
    assert!(a1.value.real == 1.1);
    Node a2 = Node(1.2f);
    assert!(a2.kind == reals);
    assert!(a2.value.real == 1.2f);
    Node as = Node('a');
    assert!(as.kind == strings or as.kind == codepoint1);
    if (as.kind == strings) { assert!(*as.value.string == 'a'); }
    if (as.kind == codepoint1) assert!((codepoint) as.value.longy == 'a');
}

#[test] fn testGroupCascade0() {
    result = parse("x='abcde';x#4='y';x#4");
    assert!(result.length == 3);
}

#[test] fn testGroupCascade1() {
    Node result0 = parse("a b; c d");
    assert!(result0.length == 2);
    assert!(result0[1].length == 2);
    result = parse("{ a b c, d e f }");
    Node result1 = parse("a b c, d e f ");
    eq!(result1, result);
    Node result2 = parse("a b c; d e f ");
    eq!(result2, result1);
    eq!(result2, result);
    Node result3 = parse("a,b,c;d,e,f");
    eq!(result3, result2);
    eq!(result3, result1);
    eq!(result3, result);
    Node result4 = parse("a, b ,c; d,e , f ");
    eq!(result4, result3);
    eq!(result4, result2);
    eq!(result4, result1);
    eq!(result4, result);
}

#[test] fn testGroupCascade2() {
    result = parse("{ a b , c d ; e f , g h }");
    Node result1 = parse("{ a b , c d \n e f , g h }");
    print(result1.serialize());
    eq!(result1, result);
    Node result2 = parse("a b ; c d \n e f , g h ");
    eq!(result1, result2);
    eq!(result2, result);
}

#[test] fn testSuperfluousIndentation() {
    result = parse("a{\n  b,c}");
    Node result1 = parse("a{b,c}");
    assert!(result1 == result);
}

#[test] fn testGroupCascade() {
    //	testGroupCascade2();
    //	testGroupCascade0();
    //	testGroupCascade1();

    result = parse("{ a b c, d e f; g h i , j k l \n "
        "a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2}"
        "{a3 b3 c3, d3 e3 f3; g3 h3 i3 , j3 k3 l3 \n"
        "a4 b4 c4 ,d4 e4 f4; g4 h4 i4 ,j4 k4 l4}");
    result.print();
    eq!(result.kind, groups); // ( {} {} ) because 2 {}!
    let &first = result.first();
    eq!(first.kind, objects); // { a b c … }
    eq!(first.first().kind, groups); // or expression if x is op
    eq!(result.length, 2) // {…} and {and}
    eq!(result[0].length, 2) // a…  and a2…  with significant newline
    eq!(result[0][0].length, 2) // a b c, d e f  and  g h i , j k l
    eq!(result[0][0][0].length, 2) // a b c  and  d e f
    eq!(result[0][0], parse("a b c, d e f; g h i , j k l")); // significant newline!
    eq!(result[0][1], parse("a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2")); // significant newline!
    eq!(result[0][0][0][0].length, 3) // a b c
    skip(
        eq!(result[0][0][0][0], parse("a b c"));
    )
    eq!(result[0][0][0][0][0], "a");
    eq!(result[0][0][0][0][1], "b");
    eq!(result[0][0][0][0][2], "c");
    eq!(result[0][0][0][1][0], "d");
    eq!(result[0][0][0][1][1], "e");
    eq!(result[0][0][0][1][2], "f");
    eq!(result[1][1][0][1][2], "f4");
    Node reparse = parse(result.serialize());
    print(reparse.serialize());
    assert!(result == reparse);
}
#[test] fn testNodeBasics() {
    Node a1 = Node(1);
    //	assert!(a1.name == "1");// debug only!
    assert!(a1 == 1);
    Node a11 = Node(1.1);
    assert!_eq(a11.name, "1.1");
    assert!(a11 == 1.1);

    Node a = Node("a");
    print(a);
    print(a.serialize());
    print(a.name);
    assert!_eq(a.name, "a");
    assert!(a.name == "a");
    assert!(a == "a");
    Node b = Node("c");
    assert!_eq(b.name, "c");
    a.add(b.clone());
    assert!_eq(b.name, "c"); // wow, worked before, corrupted memory!!
    assert!_eq(a.length, 1);
    assert!(a.children);
    Node *b2 = b.clone();
    assert!_eq(b.name, "c"); // wow, worked before, corrupted memory!!
    assert!(b == b2);
    assert!_eq(b, a.children[0]);

    //	a["b"] = "c";
    assert!_eq(b, a.children[0]);
    assert!_eq(b.name, "c"); // wow, worked before, corrupted memory!!
    assert!_eq(a.children[0].name, "c");
    assert!(a.has("c"));
    assert!(b == a.has("c"));

    //	a["b"] = "c";
    a["d"] = "e";
    assert!_eq(a.length, 2);
    assert!(a.has("d"));
    assert!(a["d"] == "e");
    Node &d = a.children[a.length - 1];
    assert!(d.length == 0);
    assert!(d == "e");
    assert!(d.kind == key);
    a.addSmart(b); // why?
}

#[test] fn testBUG();

#[test] fn testEmitBasics();

#[test] fn testSourceMap();
#[test] fn testArrayIndices() {
    skip(
        // fails second time WHY?
        assert_is("[1 2 3]", Node(1, 2, 3, 0).setType(patterns))
        assert_is("[1 2 3]", Node(1, 2, 3, 0))
    )
#[cfg(not(feature = ""))]{(WASM and INCLUDE_MERGER)
    assert_is("(1 4 3)#2", 4); // todo needs_runtime = true => whole linker machinery
    assert_is("x=(1 4 3);x#2", 4);
    assert_is("x=(1 4 3);x#2=5;x#2", 5);
}
}
#[test] fn testNodeEmit() {
    is!("y:{x:2 z:3};y.x", 2);
    is!("y:{x:'z'};y.x", 'z'); // emitData( node! ) emitNode()
    is!("y{x:1}", true); // emitData( node! ) emitNode()
    is!("y{x}", true); // emitData( node! ) emitNode()
    is!("{x:1}", true); // emitData( node! ) emitNode()
    is!("y={x:{z:1}};y", true); // emitData( node! ) emitNode()
}

//int dump_nr = 1;
//#[test] fn dumpMemory(){
// 
//	String file_name="dumps/dump"s+dump_nr++;
//	FILE* file=fopen(file_name,"wb");
//	size_t length = 65536*10;
//	fwrite(memory, length, 1, file);
//	fclose(file);
//}

#[cfg(feature = "IMPLICIT_NODES")]{
#[test] fn testNodeImplicitConversions() {
    // looks nice, but only causes trouble and is not necessary for our runtime!
    Node b = true;
    print(typeName(b.kind));
    assert!(b.value.longy == 1);
    assert!(b.kind == bools);
    assert!(b == True);
    Node a = 1;
    assert!(a.kind == longs);
    assert!(a.value.longy = 1);
    Node a0 = 10l;
    assert!(a0.kind == longs);
    assert!(a0.value.longy = 1);
    Node a1 = 1.1;
    assert!_eq(a1.kind, reals);
    assert!(a1.kind == reals);
    assert!(a1.value.real = 1.1);
    Node a2 = 1.2f;
    assert!(a2.kind == reals);
    assert!(a2.value.real = 1.2f);
    Node as = 'a';
    assert!(as.kind == strings);
    assert!(as.value.string == 'a');
}
}

#[test] fn testUnits() {
    assert_is("1 m + 1km", Node(1001).setType(types["m"]));
}

#[test] fn testPaint() {
#[cfg(feature = "SDL")]{
    init_graphics();
    while (1)paint(-1);
}
}

#[test] fn testPaintWasm() {
#[cfg(feature = "GRAFIX")]{
    //	struct timeval stop, start;
    //	gettimeofday(&start, NULL);
    // todo: let compiler compute constant expressions like 1024*65536/4
    //    	is!("i=0;k='hi';while(i<1024*65536/4){i++;k#i=65};k[1]", 65)// wow SLOOW!!!
    //out of bounds memory access if only one Memory page!
    is!("i=0;k='hi';while(i<16777216){i++;k#i=65};paint()", 0) // still slow, but < 1s
    // wow, SLOWER in wasm-micro-runtime HOW!?
    //	exit(0);

    //(√((x-c)^2+(y-c)^2)<r?0:255)
    //(x-c)^2+(y-c)^2
    is!("h=100;r=10;i=100;c=99;r=99;x=i%w;y=i/h;k=‖(x-c)^2+(y-c)^2‖<r", 1);
    ////char *wasm_paint_routine = "surface=(1,2);i=0;while(i<1000000){i++;surface#i=i*(10-√i);};paint";
    char *wasm_paint_routine = "w=1920;c=500;r=100;surface=(1,2);i=0;"
            "while(i<1000000){"
            "i++;x=i%w;y=i/w;surface#i=(x-c)^2+(y-c)^2"
            "};paint";
    //((x-c)^2+(y-c)^2 < r^2)?0x44aa88:0xffeedd
    //char *wasm_paint_routine = "surface=(1,2);i=0;while(i<1000000){i++;surface#i=i;};paint";
    //is!(wasm_paint_routine, 0);
    //	char *wasm_paint_routine = "maxi=3840*2160/4/2;init_graphics();surface=(1,2,3);i=0;while(i<maxi){i++;surface#i=i*(10-√i);};";
    eval(wasm_paint_routine);
    //	paint(0);
    //	gettimeofday(&stop, NULL);
    //	printf("took %lu µs\n", (stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec);
    //	printf("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);
    //	exit(0);
    //char *wasm_paint_routine = "init_graphics(); while(1){paint()}";// SDL bugs a bit
    //        while (1)paint(0);// help a little
}
}

#[test] fn testNodesInWasm() {
    is!("{b:c}", parse("{b:c}"));
    is!("a{b:c}", parse("a{b:c}"));
}

#[test] fn testSubGroupingIndent() {
    result = parse("x{\ta\n\tb,c,\n\td;\n\te");
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test] fn testSubGrouping() {
    // todo dangling ',' should make '\n' not close
    //	result=parse("a\nb,c,\nd;e");
    result = parse("a\n"
        "b,c,\n"
        "d;\n"
        "e");
    eq!(result.length, 3); // b,c,d should be grouped as one because of dangling comma
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test] fn testSubGroupingFlatten() {
    // ok [a (b,c) d] should be flattened to a (b,c) d
    result = parse("[a\nb,c\nd]");
    //	result=parse("a\nb,c\nd");// still wrapped!
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "d");
}

#[test] fn testBUG() {
    // move to tests() once done!
    //        testRecentRandomBugs();
}

#[test] fn testBadInWasm() {
    // break immediately
    testStringConcatWasm();
    is!("square(3.0)", 9.); // todo groupFunctionCallPolymorphic
    is!("global x=1+π", 1 + pi); // int 4 ƒ
    testWasmMutableGlobal(); // todo!
    is!("i=0;w=800;h=800;pixel=(1 2 3);while(i++ < w*h){pixel[i]=i%2 };i ", 800 * 800);
    //local pixel in context wasp_main already known  with type long, ignoring new type group<byte>
    is!("grows:=it*2; grows 3*42 > grows 2*3", 1)
    // is there a situation where a COMPARISON is ambivalent?
    // sleep ( time > 8pm ) and shower ≠ sleep time > ( 8pm and true)
    testNodeDataBinaryReconstruction(); // todo!  y:{x:2 z:3}
    testSmartReturnHarder(); // y:{x:2 z:3} can't work yet(?)
    is!("add1 x:=$0+1;add1 3", (int64) 4); // $0 specially parsed now
    is!("print 3", 3); // todo dispatch!
    is!("if 4>1 then 2 else 3", 2)

    // bad only SOMETIMES / after a while!
    is!("puts('ok');(1 4 3)#2", 4); // EXPECT 4 GOT 1n
    is!("'αβγδε'#3", U'γ'); // TODO! sometimes works!?
    is!("3 + √9", (int64) 6); // why !?!
    is!("id 3*42> id 2*3", 1)
    testSquares(); // ⚠️

    // often breaks LATER! usually some map[key] where key missing!
    // WHY do thesAe tests break in particular, sometimes?
    testMergeOwn();
    testEmitter(); // huh!?!
}
#[test] fn assurances() {
#[cfg(feature = "WASM")]{
    //	assert!(sizeof(Type32) == 4) // todo:
#else
    //    assert!(sizeof(Type32) == 4) // otherwise all header structs fall apart
    assert!(sizeof(Type64) == 8) // otherwise all header structs fall apart
    //    assert!(sizeof(Type) == 8) // otherwise all header structs fall apart
}
    //    assert!(sizeof(void*)==4) // otherwise all header structs fall apart TODO adjust in 64bit wasm / NORMAL arm64 !!
    assert!(sizeof(int64) == 8)
}

// todo: merge with testAllWasm, move ALL of these to test_wasm.rs
#[test] fn testAllEmit() {
    // WASM emit tests under the hood:
    // is!("√3^2", 3); // basics
    //	is!("42", 42);// basics
    //    exit(42);
    //    is!("√ π ²", pi);
    //    is!("√π²", pi);
    testFunctionDeclaration();
    testForLoops();
    testHex();
    testEmitBasics();
    testMinusMinus();
    testSinus();

    // newly resolved:
    testModulo(); // fixed by adding modulo_float!
    testRootLists();
    testIndexOffset();
    testEnumConversion();
    testDeepColon2();
    testPattern();

    testSmartReturn();
    testWasmString(); // with length as header
    //    return;
    testArrayIndices();
    testMultiValue();
    testLogic();

    testLogic01();
    testLogicOperators();
    testRoots();
    testRootFloat();
    testMathExtra(); // "one plus two times three"==7 used to work?
    testTruthiness();
    testLogicPrecedence();
    testRootLists();
    testArrayIndices();
    testSmartReturn();
    testMultiValue();
    //    testSinus();

    testAllAngle();
    testRecentRandomBugs();
    testEqualities();

    skip( // todo!
        testBadInWasm();
    )
    //    part of
    //    testAllWasm() :
    //    testRoundFloorCeiling();

#[cfg(feature = "APPLE")]{
    testAllSamples();
}
    assert!(NIL.value.longy == 0); // should never be modified
    print("ALL TESTS PASSED");
}
#[test] fn testHostIntegration() {
#[cfg(feature = "WASMTIME")]{ or WASMEDGE
    return;
}
#[cfg(not(feature = "WASM"))]{
    testHostDownload(); // no is!
}
    test_getElementById();
    testDom();
    testDomProperty();
    testInnerHtml();
    testJS();
    testFetch();
    skip(
        testCanvas(); // attribute setter missing value breaks browser
    )
}
#[test] fn print(Module &m) {
    print("Module");
    print("name:");
    print(m.name);
    print("code:");
    print(m.code);
    print("import_data:");
    print(m.import_data);
    print("export_data:");
    print(m.export_data);
    print("functype_data:");
    print(m.functype_data);
    print("code_data:");
    print(m.code_data);
    print("globals_data:");
    print(m.globals_data);
    print("memory_data:");
    print(m.memory_data);
    print("table_data:");
    print(m.table_data);
    print("name_data:");
    print(m.name_data);
    print("data_segments:");
    print(m.data_segments);
    print("linking_section:");
    print(m.linking_section);
    print("relocate_section:");
    print(m.relocate_section);
    print("funcToTypeMap:");
    print(m.funcToTypeMap);
    // print("custom_sections:");
    // print(m.custom_sections);
    print("type_count:");
    print(m.type_count);
    print("import_count:");
    print(m.import_count);
    print("total_func_count:");
    print(m.total_func_count);
    print("table_count:");
    print(m.table_count);
    print("memory_count:");
    print(m.memory_count);
    print("export_count:");
    print(m.export_count);
    print("global_count:");
    print(m.global_count);
    print("code_count:");
    print(m.code_count);
    print("data_segments_count:");
    print(m.data_segments_count);
    print("start_index:");
    print(m.start_index);
    // print("globals); List<Global> WHY NOT??");
    print("m.functions.size()");
    print(m.functions.size());
    // print("m.funcTypes.size()");
    // print(m.funcTypes.size());
    assert!(m.funcTypes.size()==m.type_count);

    // print("m.signatures.size()");
    // print(m.signatures.size());
    // print("m.export_names");
    // print(m.export_names); // none!?
    // print("import_names:");
    // print(m.import_names);
}

#[test] fn test_const_String_comparison_bug() {
    // fixed in 8268c182 String == chars ≠> chars == chars  no more implicit cast
    const String &library_name = "raylib";
    assert!(library_name == "raylib");
}
#[test] fn todo_done() { // moved from todo()
    // GOOD! move to tests() once they work again but a#[test] fn redundant test executions
    assert_is("2+1/2", 2.5);
    is!("a = [1, 2, 3]; a[2]", 3);
    // #[cfg(not(feature = "WASMTIME"))]{ and not LINUX // todo why
    // is!("n=3;2ⁿ", 8);
#[cfg(feature = "NATIVE_FFI")]{
    test_ffi_sdl();
    // SDL tests temporarily disabled - debugging type mismatches
    // test_ffi_sdl_red_square_demo();
}
    test_list_lambdas();

    testMapOfStrings();
    testMapOfStringValues();
    testMaps();
    testNotNegation();
    testWhileNot();
    testAutoType();
    testTypeSynonyms();
    testFetch();
    testWGSL();
    testFunctionDeclaration();
    testReturnTypes();
    testRecentRandomBugs();
    // exit(0); // todo: remove this once all tests are passing
    testStringInterpolation();
    // we already have a working syntax so this has low priority?
    // testFunctionDeclaration();
    // testFibonacci(); // much TODO!
    // testSinus();

    testWaspRuntimeModule();

    testPing();
    test_while_true_forever();
    testStructWast();
    test_wasm_node_struct();
#[cfg(feature = "NATIVE_FFI")]{
    test_ffi_all();
}
    testMergeRuntime();
    testFunctionArgumentCast();
    testWrong0Termination();
    is!("fun addier(x, y){ x + y }; addier(3,4)", 7);
    testErrors(); // error: failed to call function   wasm trap: integer divide by zero

    read_wasm("lib/stdio.wasm");
    //    testStruct();

    testWit();
    testColonImmediateBinding();
    testWasmRuntimeExtension();
    testUpperLowerCase();
    //    exit(1);
    testDataMode();
    testParams();
    is!("\"Hello \" + \"🌍\" + (2000+25)","Hello 🌍2025");

    // test_const_String_comparison_bug(); // fixed in 8268c182
}
// todo: ^^ move back into tests() once they work again
#[test] fn todos() {

    skip( // unskip to test!!
        test_wasm_structs();

        testKitchensink();
        testNodeEmit();
        testLengthOperator();
        testEmitCast();
        is!("2,4 == 2,4", 1);
        is!("(2,4) == (2,4)", 1); // todo: array creation/ comparison
        is!("‖-2^2 - -2^3‖", 4); // Too many args for operator ‖,   a - b not grouped!
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        testWasmTypedGlobals();
    )

#[cfg(not(feature = "TRACE"))]{
    println("parseLong fails in trace mode WHY?");
    is!("parseLong('123000')+parseLong('456')", 123456);
}

    test_sinus_wasp_import();
    testSinus(); // todo FRAGILE fails before!
    //    testSinus2();
    //    run("circle.wasp");
    // while without body
    //    Missing condition for while statement
    skip(
        is!("i=0;while(i++ <10001);i", 10000) // parsed wrongly! while(  <( ++ i 10001) i)
    )

    is!("use math;⅓ ≈ .3333333 ", 1);
    is!("precision = 3 digits; ⅓ ≈ .333 ", 1);
    assert_throws("i*=3"); // well:
    is!("i*=3", (int64) 0);
    // todo: ERRORS when cogs don't match! e.g. remove ¬ from prefixOperators!
    assert_throws("ceiling 3.7");
    // default bug!
    //    	subtract(other complex) := re -= other.re; im -= other.im
    // := is terminated by \n, not by ;!
    assert_throws("xyz 3.7"); // todo SHOULD THROW unknown symbol!
    is!("if(0):{3}", false); // 0:3 messy node
    eq!(Node("1", 0) + Node("2"_s),
                  Node("1", "2", 0)); // 1+2 => 1:2  stupid border case because 1 not group (1)
    assert_is((char *) "{a b c}#2", "b"); // ok, but not for patterns:
    assert_is((char *) "[a b c]#2", "b"); // patterns
    is!("abs(0)", 0);
    assert_is("i=3;i--", 2); // todo bring variables to interpreter
    assert_is("i=3.7;.3+i", 4); // todo bring variables to interpreter
    assert_is("i=3;i*-1", -3); // todo bring variables to interpreter
    assert_is("one plus two times three", 7);
    //	print("OK %s %d"s % ("WASM",1));// only 1 handed over
    //    print(" OK %d %d"s % (2, 1));// error: expression result unused [-Werror,-Wunused-value] OK
    is!("use wasp;use lowerCaseUTF;a='ÂÊÎÔÛ';lowerCaseUTF(a);a", "âêîôû")
    test2Def();
    testConstructorCast();
    is!("html{bold{'Hello'}}", "Hello"); // in wasmtime
}

#[test] fn test_todos() {
    todos();
    // move to test_done() once done!
}
#[test] fn todo_done(); // may be below

#[test] fn tests() {
    todo_done();
    assurances();
#[cfg(not(feature = "WASM"))]{
    testNumbers();
    testPower();
    testEmitStringConcatenation();
    testExternReferenceXvalue();
    testExternString();
}
    testCast();
    testFunctionDeclarationParse();
    testPower();
    testRandomParse();
    eq!(String("a1b1c1d").lastIndexOf("1"), 5);
    testUnicode_UTF16_UTF32();
    testReplaceAll();
    testExceptions();
    testString();
    testNodeBasics();
    testIterate();
    testLists();
    testEval();
    testParent();
    testNoBlock(); // fixed
    testSubGroupingFlatten();
    testNodeConversions();
    testUpperLowerCase();
    testListGrow();
    testGroupCascade();
    testNewlineLists();
    testStackedLambdas();

    testParamizedKeys();
    testForEach();
    testEmpty();
    testDiv();
    testRoot();
    testSerialize();
    skip(
        testPrimitiveTypes();
    )
    //	test_sin();
    testIndentAsBlock();
    testDeepCopyDebugBugBug2(); // SUBTLE: BUGS OUT ONLY ON SECOND TRY!!!
    testDeepCopyDebugBugBug();
    testComments();
    testEmptyLineGrouping();
    testSwitch();
    testAsserts();
    testFloatReturnThroughMain();
    testSuperfluousIndentation();
    testString();
    testEmptyLineGrouping();
    testColonLists();
    testGraphParams();
    testNodeName();
    testStringConcatenation();
    testStringReferenceReuse();
    testConcatenation();
    testMarkSimple();
    testMarkMulti();
    testMarkMulti2();
    testDedent2();
    testDedent();
    testGroupCascade0();
    testGraphQlQuery();
    print(testNodiscard());
    testCpp();
    testNilValues();
    testMapsAsLists();
    testMaps();
    testLists();
    testDeepLists();
    testGraphParams();
    testAddField();
    testOverwrite();
    testDidYouMeanAlias();
    testNetBase();
    testForEach();
    // testLengthOperator();
    testLogicEmptySet();
    testDeepCopyDebugBugBug();
    testDeepCopyDebugBugBug2();
    //    testMarkSimpleAssign();
    testSort();
    testSort1();
    testSort2();
    testReplace();
    testRemove();
    testRemove2();
    testGraphQlQueryBug();
    testGraphQlQuery(); // fails sometimes => bad pointer!?
    testGraphQlQuery2();
    testUTF(); // fails sometimes => bad pointer!?
    testUnicode_UTF16_UTF32();
    testConcatenationBorderCases();
    testNewlineLists();
    testIndex();
    testGroupCascade();
    testParams();
    testSignificantWhitespace();
    testBUG();
    testFlags();
    //    testFlags2();
    //    testFlagSafety();
#[cfg(feature = "WASM")]{
    warn("Currently NOT PASSING via wasmtime -D debug-info=y --dir . wasp.wasm test");
}
    testMarkAsMap();
    testFunctionDeclarationParse();
    testMarkSimple();
#[cfg(not(feature = "WASM"))]{
    testMarkMultiDeep();
}
#[cfg(feature = "WASM")]{
    warn("Normal tests ALL PASSING in wasm!");
    warn("WASM emit tests CURRENTLY __ALL__ SKIPPED or asynchroneous!");
    return;
#else
    testAllEmit();
}
    // todo: split in test_wasp test_angle test_emit.rs
}

#[test] fn test_new() {
    //    testInclude();
    //    testMatrixOrder();
#[cfg(feature = "WASMEDGE")]{ 
    test_wasmedge_gc();
}
    // test_list_growth();
    testFlags();
    testTypes();
    testPolymorphism();
}

// 2021-10 : 40 sec for Wasm3
// 2022-05 : 8 sec in Webapp / wasmtime with wasp.wasm built via wasm-runtime
// 2022-12-03 : 2 sec WITHOUT runtime_emit, wasmtime 4.0 X86 on M1
// 2022-12-03 : 10 sec WITH runtime_emit, wasmtime 4.0 X86 on M1
// 2022-12-28 : 3 sec WITH runtime_emit, wasmedge on M1 WOW ALL TESTS PASSING
// 2025-03-23 : <5 sec WITH runtime_emit, WASMTIME/WAMR/WASMEDGE on M1, 45 sec in Chrome (because print?)
// ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
// 2025-12-23 : 10 sec WITH runtime_emit, wasmtime 4.0 on M2
#[test] fn testCurrent() {
    // print("testCurrent DEACTIVATED");
    // return;
    print("💡 starting current tests 💡");
    // testTruthiness();
#[cfg(feature = "WASM")]{
    print("⚠️ make sure to put all is! into testRun() ");
}
    // eval("./samples/raylib_mouse_circle.wasp");
    // testTruthyAnd();
    is!("fun addier(a,b){b+a};addier(42,1)", 43);
    testPing();
    testIfCallZero();
    testWhileNotCall();
    testReturnTypes();
    // test_ffi_all();
    // test_ffi_raylib_simple_use_import();

    // exit(   0);
    skip(
        testDeepColon(); // wit definition vs wasp declaration clash!
        todos(); // WIP and BUGs
    )
    todo_done();
    // sleep(10);
    // exit(0);
    // test_dynlib_import_emit();
#[cfg(feature = "WASMEDGE")]{
    testStruct(); // no wasmtime yet
}

    skip( // TODO!
        test_new();
        is!("x=3;y=4;c=1;r=5;(‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);

        testMergeGlobal(); //
        testRenameWasmFunction();
        testExp(); // todo!
        testKebabCase(); // needed later …
        testStruct();
        todos();
        assert_is("(1 4 3)#2", 4); //
        assert_throws("0/0"); // now NaN OK
        testPolymorphism2();
        testPolymorphism3();
        testVectorShim(); // use GPU even before wasm vector extension is available
        testModifiers();
        testLengthOperator();
        testNamedDataSections();
        testHostDownload();
        testHostIntegration();
        testJS();
        testHtmlWasp();
    )
    // testListGrowth<const int&>();// pointer to a reference error

    // todo print as general dispatch depending on smarttype
    //    is!("for i in 1 to 5 : {print i};i", 6);

    testSourceMap();
    //	testDwarf();

    // ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
    tests(); // make sure all still ok after messing with memory

#[cfg(not(feature = "WASM"))]{
    // ⚠️ in WASM these tests are called via async trick
    testAngle(); // fails in WASM why?
    testAssertRun(); // separate because they take longer (≈10 sec as of 2022.12)
    testAllWasm();
    //    todos();// those not passing yet (skip)
}
    print(tests_executed);
    print("CURRENT TESTS PASSED");
}

// valgrind --track-origins=yes ./wasp
