use wasp::{eq, is, skip};
use wasp::node::Node;
use wasp::node::Node::{Empty, False, True};
use wasp::wasp_parser::parse;

fn Node() -> Node {
    todo!()
}


#[test]
fn test_did_you_mean_alias() {
    skip!(

        Node ok1 = assert_parses("printf!('hi')");
        eq!(ok1[".warnings"], "DYM print"); // THIS CAN NEVER HAVED WORKED! BUG IN TEST PIPELINE!
    );
}

#[test]
fn test_empty() {
    result = assert_parsesx("{  }");
//     eq!(_x(result.length, 0);
}

#[test]
fn test_eval() {
    skip!(

        is!("√4", 2);
    );
}


#[test]
fn test_node_name() {
    a = Node("xor"); // NOT type string by default!
//     bool
    ok1 = a == "xor";
    assert!(a == "xor");
    assert!(a.name == "xor");
    assert!(ok1);
}

#[test]
fn test_indent_as_block() {
    todo_emit(

        //==============================================================================
        // NODE/DATA STRUCTURE TESTS (see node_tests.h);
        //==============================================================================

//         is!((char *) "a\n\tb", "a{b}");
    );
    // 0x0E 	SO 	␎ 	^N 		Shift Out
    // 0x0F 	SI 	␏ 	^O 		Shift In
    //	indent/dedent  0xF03B looks like pause!?   0xF032…  it does, what's going on CLion? Using STSong!
    //	https://fontawesome.com/v4.7/icon/outdent looks more like it, also matching context  OK in font PingFang HK?
} // 􀖯􀉶𠿜🕻🗠🂿	𝄉



#[test]
fn test_string_concatenation() {
    //	eq!(Node("✔️"), True);
    //	eq!(Node("✔"), True);
    //	eq!(Node("✖️"), False);
    //	eq!(Node("✖"), False);
    let huh = "a" + 2;
//     assert!(_eq!(huh.length, 2);
//     assert!(_eq!(huh[0], 'a');
//     assert!(_eq!(huh[1], '2');
//     assert!(_eq!(huh[2], (int64) 0);
    is!("a2", "a2");

    eq!(huh, "a2");
    eq!("a" + 2, "a2");
    eq!("a" + 2.2, "a2.2");
    eq!("a" + "2.2", "a2.2");
    eq!("a" + 'b', "ab");
    eq!("a" + "bc", "abc");
    eq!("a" + true, "a✔️");
    eq!("a%sb" % "hi", "ahib");

    eq!("a%db" % 123, "a123b");
    eq!("a%s%db" % "hi" % 123, "ahi123b");
}




#[test]
fn test_group_cascade1() {
    let result0 = parse("a b; c d");
    assert!(result0.length() == 2);
    assert!(result0[1].length() == 2);
    let result = parse("{ a b c, d e f }");
    let result1 = parse("a b c, d e f ");
    eq!(result1, result);
    let result2 = parse("a b c; d e f ");
    eq!(result2, result1);
    eq!(result2, result);
    let result3 = parse("a,b,c;d,e,f");
    eq!(result3, result2);
    eq!(result3, result1);
    eq!(result3, result);
    let result4 = parse("a, b ,c; d,e , f ");
    eq!(result4, result3);
    eq!(result4, result2);
    eq!(result4, result1);
    eq!(result4, result);
}

#[test]
fn test_group_cascade2() {
    let result = parse("{ a b , c d ; e f , g h }");
    let result1 = parse("{ a b , c d \n e f , g h }");
    // print(result1.serialize());
    eq!(result1, result);
    let result2 = parse("a b ; c d \n e f , g h ");
    eq!(result1, result2);
    eq!(result2, result);
}

#[test]
fn test_superfluous_indentation() {
    let result = parse("a{\n  b,c}");
    let result1 = parse("a{b,c}");
    assert!(result1 == result);
}

#[test]
fn test_group_cascade() {
    //	test_group_cascade2();
    //	testGroupCascade0();
    //	test_group_cascade1();

    let result = parse(r#"{ a b c, d e f; g h i , j k l
              a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2}
              {a3 b3 c3, d3 e3 f3; g3 h3 i3 , j3 k3 l3
              a4 b4 c4 ,d4 e4 f4; g4 h4 i4 ,j4 k4 l4}"#);
    result.print();
    // eq!(result.kind(), groups); // ( {} {} ) because 2 {}!
    let first = result.first();
    // eq!(first.kind(), objects); // { a b c … }
    // eq!(first.first().kind(), groups); // or expression if x is op
//     eq!(result.length, 2) // {…} and {and}
//     eq!(result[0].length, 2) // a…  and a2…  with significant newline
//     eq!(result[0][0].length, 2) // a b c, d e f  and  g h i , j k l
//     eq!(result[0][0][0].length, 2) // a b c  and  d e f
    eq!(result[0][0], parse("a b c, d e f; g h i , j k l")); // significant newline!
    eq!(result[0][1], parse("a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2")); // significant newline!
//     eq!(result[0][0][0][0].length, 3) // a b c
    skip!(
        eq!(result[0][0][0][0], parse("a b c"));
    );
    eq!(result[0][0][0][0][0], "a");
    eq!(result[0][0][0][0][1], "b");
    eq!(result[0][0][0][0][2], "c");
    eq!(result[0][0][0][1][0], "d");
    eq!(result[0][0][0][1][1], "e");
    eq!(result[0][0][0][1][2], "f");
    eq!(result[1][1][0][1][2], "f4");
    let reparse = parse(result.serialize());
    // print(reparse.serialize());
    assert!(result == reparse);
}










// todo: merge with testAllWasm, move ALL of these to test_wasm.rs
#[test]
fn test_all_emit() {
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
    test_roots();
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

    skip!(
 // todo!
        testBadInWasm();
    );
    //    part of
    //    testAllWasm() :
    //    testRoundFloorCeiling();

    #[cfg(feature = "APPLE")]{
        testAllSamples();
    }
    assert!(Empty == 0); // should never be modified
    // print("ALL TESTS PASSED");
}

#[test]
fn todo_done() { // moved from todo();
    // GOOD! move to tests() once they work again but a#[test] fn redundant test executions
    is!("2+1/2", 2.5);
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

    // test_const_string_comparison_bug(); // fixed in 8268c182
}
// todo: ^^ move back into tests() once they work again
#[test]
fn todos() {
    skip!(
 // unskip to test!!
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
    );

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
    skip!(

        is!("i=0;while(i++ <10001);i", 10000) // parsed wrongly! while(  <( ++ i 10001) i);
    );

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
    eq!(Node("1", 0) + Node("2"),
                  Node("1", "2", 0)); // 1+2 => 1:2  stupid border case because 1 not group (1);
//     is!((char *) "{a b c}#2", "b"); // ok, but not for patterns:
//     is!((char *) "[a b c]#2", "b"); // patterns
    is!("abs(0)", 0);
    is!("i=3;i--", 2); // todo bring variables to interpreter
    is!("i=3.7;.3+i", 4); // todo bring variables to interpreter
    is!("i=3;i*-1", -3); // todo bring variables to interpreter
    is!("one plus two times three", 7);
    //	print("OK %s %d" % ("WASM",1));// only 1 handed over
    //    print(" OK %d %d" % (2, 1));// error: expression result unused [-Werror,-Wunused-value] OK
    is!("use wasp;use lowerCaseUTF;a='ÂÊÎÔÛ';lowerCaseUTF(a);a", "âêîôû");
    test2Def();
    testConstructorCast();
    is!("html{bold{'Hello'}}", "Hello"); // in wasmtime
}

#[test]
fn test_todos() {
    todos();
    // move to test_done() once done!
}
#[test]
fn todo_done(){} // may be below

#[test]
fn tests() {
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
    test_string();
    testNodeBasics();
    testIterate();
    testLists();
    test_eval();
    testParent();
    testNoBlock(); // fixed
    test_sub_grouping_flatten();
    testNodeConversions();
    testUpperLowerCase();
    testListGrow();
    test_group_cascade();
    testNewlineLists();
    testStackedLambdas();

    testParamizedKeys();
    testForEach();
    test_empty();
    testDiv();
    testRoot();
    testSerialize();
    skip!(

        testPrimitiveTypes();
    );
    //	test_sin();
    test_indent_as_block();
    testDeepCopyDebugBugBug2(); // SUBTLE: BUGS OUT ONLY ON SECOND TRY!!!
    testDeepCopyDebugBugBug();
    testComments();
    testEmptyLineGrouping();
    testSwitch();
    test_asserts();
    testFloatReturnThroughMain();
    test_superfluous_indentation();
    test_string();
    testEmptyLineGrouping();
    testColonLists();
    testGraphParams();
    test_node_name();
    test_string_concatenation();
    testStringReferenceReuse();
    test_concatenation();
    testMarkSimple();
    testMarkMulti();
    testMarkMulti2();
    testDedent2();
    testDedent();
    testGroupCascade0();
    testGraphQlQuery();
    // print(testNodiscard());
    testCpp();
    test_nil_values();
    testMapsAsLists();
    testMaps();
    testLists();
    testDeepLists();
    testGraphParams();
    testAddField();
    testOverwrite();
    test_did_you_mean_alias();
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
    test_concatenation_border_cases();
    testNewlineLists();
    test_index();
    test_group_cascade();
    testParams();
    testSignificantWhitespace();
    test_bug();
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
//         # else
        test_all_emit();
    }
    // todo: split in test_wasp test_angle test_emit.rs
}

#[test]
fn test_new() {
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
// 2025-03-23 : <5 sec WITH runtime_emit, WASMTIME/WAMR/WASMEDGE on M1, 45 sec in Chrome (because print?);
// ⚠️ CANNOT USE is! in WASM! ONLY via #[test] fn testRun();
// 2025-12-23 : 10 sec WITH runtime_emit, wasmtime 4.0 on M2
#[test]
fn testCurrent() {
    // print("testCurrent DEACTIVATED");
    // return;
    // print("💡 starting current tests 💡");
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
    skip!(

        testDeepColon(); // wit definition vs wasp declaration clash!
        todos(); // WIP and BUGs
    );
    todo_done();
    // sleep(10);
    // exit(0);
    // test_dynlib_import_emit();
    #[cfg(feature = "WASMEDGE")]{
        testStruct(); // no wasmtime yet
    }

    skip!(
 // TODO!
        test_new();
        is!("x=3;y=4;c=1;r=5;(‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);

        testMergeGlobal(); //
        testRenameWasmFunction();
        testExp(); // todo!
        testKebabCase(); // needed later …
        testStruct();
        todos();
        is!("(1 4 3)#2", 4); //
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
    );
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
        testAssertRun(); // separate because they take longer (≈10 sec as of 2022.12);
        testAllWasm();
        //    todos();// those not passing yet (skip);
    }
    // print(tests_executed);
    // print("CURRENT TESTS PASSED");
}

// }
// // // // valgrind --track-origins=yes ./wasp
