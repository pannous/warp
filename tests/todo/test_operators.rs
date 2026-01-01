
// Operator tests
// Migrated from tests_*.rs files

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
fn testVectorShim() {
    //    unknown function matrix_multiply (matrix_multiply);
    is!("v=[1 2 3];w=[2 3 4];v*w", 2 + 6 + 12);
}

#[test]
fn testWitExport() {
//     const char
    *code = "struct point{x:int y:float}";
    Node & node = parse(code);
    bindgen(node);
}

#[test]
fn testWitImport() {}

#[test]
fn testEqualsBinding() {
    // colon closes with space, not semicolon !
    parse("a = float32, b: float32");
    assert!(result.length == 1);
    assert!(result["a"] == "float32");
    val;
    val.add(Node("float32"));
    val.add(Node("b").add(Node("float32")));
    eq!(result[0], val);
}

#[test]
fn testHypenVersusMinus() {
    // Needs variable register in parser.
//     const char
    *code = "a=-1 b=2 b-a";
    is!(code, 3);
    // kebab case
//     const char
    *data = "a-b:2 c-d:4 a-b";
    is!(data, 2);
    //    testHyphenUnits();

    //    Node &node = parse(data);
}

#[test]
fn test_sinus_wasp_import() {
    // using sin.wasp, not sin.wasm
    // todo: compile and reuse sin.wasm if unmodified
    is!("use sin;sin π/2", 1);
    is!("use sin;sin π", 0);
    is!("use sin;sin 3*π/2", -1);
    is!("use sin;sin 2π", 0);
    is!("use sin;sin -π/2", -1);
}

#[test]
fn testImport42() {
    assert_is("import fourty_two", 42);
    assert_is("include fourty_two", 42);
    assert_is("require fourty_two", 42);
    assert_is("import fourty_two;ft*2", 42 * 2);
    assert_is("include fourty_two;ft*2", 42 * 2);
    assert_is("require fourty_two;ft*2", 42 * 2);
}

#[test]
fn testDivDeep() {
    div = parse("div{ span{ class:'bold' 'text'} br}");
    Node & node = div["span"];
    node.print();
    assert(div["span"].length == 2);
    assert(div["span"]["class"] == "bold");
}

#[test]
fn testDivMark() {
    use_polish_notation = true;
    div = parse("{div {span class:'bold' 'text'} {br}}");
    Node & span = div["span"];
    span.print();
    assert(span.length == 2);
    assert(span["class"] == "bold");
    use_polish_notation = false;
}

#[test]
fn testDiv() {
    result = parse("div{ class:'bold' 'text'}");
    result.print();
    assert(result.length == 2);
    assert(result["class"] == "bold");
    testDivDeep();
    skip!(

        testDivMark();
    );
}

#[test]
fn testMarkMultiDeep() {
    // fragile:( problem :  c:{d:'hi'}} becomes c:'hi' because … bug

#[test]
fn testMarkMulti() {
//     chars
    source = "{a:'HIO' b:3}";
    assert_parses(source);
    Node & node = result['b'];
    print(result['a']);
    print(result['b']);
    assert(result["b"] == 3);
    assert(result['b'] == node);
}

#[test]
fn testMarkMulti2() {
    assert_parses("a:'HIO' b:3  d:{}");
    assert(result["b"] == 3);
}

#[test]
fn testErrors() {
    // use assert_throws
    throwing = true;
    // 0/0 now returns NaN (float division), not an error
    assert_throws("x"); // UNKNOWN local symbol 'x' in context main  OK
    #[cfg(feature = "WASI")]{
//         or
//         WASM
        skip!(
"can't catch ERROR in wasm");
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

#[test]
fn testForEach() {
//     int
    sum = 0;
//     for (Node &item: parse(
//     "1 2 3"));
    sum += item.value.longy;
    assert(sum == 6);
}

#[test]
fn testLogic() {
    assert_is("true or false", true);
    assert_is("false or true", true);

    assert_is("not true", false);
    assert_is("not false", true); // fourth test fails regardles of complexity?

    assert_is("false or false", false);
    assert_is("true or false", true);
    assert_is("true or true", true);
    //==============================================================================
    // LOGIC/BOOLEAN TESTS (see angle_tests.h + feature_tests.h);
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

#[test]
fn testLogicEmptySet() {
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

#[test]
fn testLogicOperators() {
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

#[test]
fn testLogic01() {
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

#[test]
fn testLengthOperator() {
    is!("#'0123'", 4); // todo at compile?
    is!("#[0 1 2 3]", 4);
    is!("#[a b c d]", 4);
    is!("len('0123')", 4); // todo at compile?
    is!("len([0 1 2 3])", 4);
    is!("size([a b c d])", 4);
    assert_is("#{a b c}", 3);
    assert_is("#(a b c)", 3); // todo: groups
}

#[test]
fn testConcatenationBorderCases() {
    eq!(Node(1, 0) + Node(3, 0), Node(1, 3, 0)); // ok
    eq!(Node("1", 0, 0) + Node("2", 0, 0), Node("1", "2", 0));
    // Border cases: {1}==1;
    eq!(parse("{1}"), parse("1"));
    // Todo Edge case a=[] a+=1
    eq!(Node() + Node("1", 0, 0), Node("1", 0, 0));
    //  singleton {1}+2==1+2 = 12/3 should be {1,2}
    eq!(Node("1", 0, 0) + Node("x"s), Node("1", "x", 0));
}

#[test]
fn testSort() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
        assert!(list.sort() == listb);
//         let by_precedence = [](int & a, int & b)
        { return a * a > b * b; };
        assert!(list.sort(by_precedence) == listb);
//         let by_square = [](int & a)
        {
            return (float);
            a * a;
        };
        assert!(list.sort(by_square) == listb);
    }
}

#[test]
fn testSort1() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
//         let by_precedence = [](int & a, int & b)
        { return a * a > b * b; };
        assert!(list.sort(by_precedence) == listb);
    }
}

#[test]
fn testSort2() {
    #[cfg(not(feature = "WASM"))]{
        // List<int> list = { 3, 1, 2, 5, 4 };
        // List<int> listb = { 1, 2, 3, 4, 5 };
//         let by_square = [](int & a)
        {
            return (float);
            a * a;
        };
        assert!(list.sort(by_square) == listb);
    }
}

#[test]
// fn print(Module &m) {
//     print("Module");
//     print("name:");
    // print(m.name);
    // print("code:");
    // print(m.code);
    // print("import_data:");
    // print(m.import_data);
    // print("export_data:");
    // print(m.export_data);
    // print("functype_data:");
    // print(m.functype_data);
    // print("code_data:");
    // print(m.code_data);
    // print("globals_data:");
    // print(m.globals_data);
    // print("memory_data:");
    // print(m.memory_data);
    // print("table_data:");
    // print(m.table_data);
    // print("name_data:");
    // print(m.name_data);
    // print("data_segments:");
    // print(m.data_segments);
    // print("linking_section:");
    // print(m.linking_section);
    // print("relocate_section:");
    // print(m.relocate_section);
    // print("funcToTypeMap:");
    // print(m.funcToTypeMap);
    // print("custom_sections:");
    // print(m.custom_sections);
    // print("type_count:");
    // print(m.type_count);
    // print("import_count:");
    // print(m.import_count);
    // print("total_func_count:");
    // print(m.total_func_count);
    // print("table_count:");
    // print(m.table_count);
    // print("memory_count:");
    // print(m.memory_count);
    // print("export_count:");
    // print(m.export_count);
    // print("global_count:");
    // print(m.global_count);
    // print("code_count:");
    // print(m.code_count);
    // print("data_segments_count:");
    // print(m.data_segments_count);
    // print("start_index:");
    // print(m.start_index);
    // print("globals); List<Global> WHY NOT??");
    // print("m.functions.size()");
    // print(m.functions.size());
    // print("m.funcTypes.size()");
    // print(m.funcTypes.size());
    assert!(m.funcTypes.size() == m.type_count);

    // print("m.signatures.size()");
    // print(m.signatures.size());
    // print("m.export_names");
    // print(m.export_names); // none!?
    // print("import_names:");
    // print(m.import_names);
// }

#[test]
fn test_const_String_comparison_bug() {
    // fixed in 8268c182 String == chars ≠> chars == chars  no more implicit cast
//     const String
    &library_name = "raylib";
    assert!(library_name == "raylib");
}

