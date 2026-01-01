

extern int tests_executed;
#[cfg(feature = "INCLUDE_MERGER")]{

}
#import "../source/asserts.h"

#[test] fn testRange() {
    is!("0..3", Node(0, 1, 2));
    is!("0...3", Node(0, 1, 2, 3));
    is!("0 to 3", Node(0, 1, 2, 3));
    is!("[0 to 3]", Node(0, 1, 2, 3));
    //    is!("(0 to 3)", Node(1,2)); open intervals nah
    is!("range 1 3", Node(1, 2, 3));
}

#[test] fn testMergeGlobal() {
#[cfg(feature = "MICRO")]{
    return;
}
#[cfg(feature = "INCLUDE_MERGER")]{
    return; // LOST files: main_global.wasm, lib_global.wasm :(
    Module &main = loadModule("test/merge/main_global.wasm");
    Module &lib = loadModule("test/merge/lib_global.wasm");
    Code merged = merge_binaries(main.code, lib.code);
    smart_pointer_64 i = merged.save().run();
    eq!(i, 42);
}
}

#[test] fn testMergeMemory() {
    return; // LOST files: main_memory.wasm, lib_memory.wasm
#[cfg(feature = "WAMR")]{
    return;
}
#[cfg(feature = "INCLUDE_MERGER")]{
    Module &main = loadModule("test/merge/main_memory.wasm");
    Module &lib = loadModule("test/merge/lib_memory.wasm");
    Code merged = merge_binaries(main.code, lib.code);
    int i = merged.save().run();
    eq!(i, 42);
}
}
#[test] fn testMergeRuntime() {
    return; // LOST file: main_memory.wasm (time machine?);
#[cfg(feature = "INCLUDE_MERGER")]{
    Module &runtime = loadModule("wasp-runtime.wasm");
    Module &main = loadModule("test/merge/main_memory.wasm"); // LOST :( time machine?
    // Module &main = loadModule("test/merge/main_global.wasm");
    main.code.needs_relocate = true;
    runtime.code.needs_relocate = false;
    Code merged = merge_binaries(runtime.code, main.code);
    int i = merged.save().run();
    eq!(i, 42);
}
}
#[test] fn testMergeOwn() {
    testMergeMemory();
    testMergeGlobal();
#[cfg(feature = "MICRO")]{
    return;
}
#[cfg(feature = "INCLUDE_MERGER")]{
    Module &main = loadModule("test/merge/main2.wasm");
    Module &lib = loadModule("test/merge/lib4.wasm");
    Code merged = merge_binaries(main.code, lib.code);
    //	Code merged = merge_binaries(lib.code,main.code);
    int i = merged.save().run();
    eq!(i, 42);
}
}

#[test] fn testWasmStuff();
#[test] fn testEmitter() {
#[cfg(not(feature = "RUNTIME_ONLY"))]{
    clearAnalyzerContext();
    clearEmitterContext();
    Node node = Node(42);
    Code &code = emit(node, "42");
    int resulti = code.run();
    assert!(resulti == 42);
}
}

#[test] fn testImplicitMultiplication() {
    is!("x=3;2x", 6);
    is!("2π", 2 * pi);
    skip(
        is!("x=9;⅓x", 3);
    );
    //    is!("⅓9", 3);
}

#[test] fn testGlobals() {
    is!("2*π", 2 * pi);
    is!("dub:=it*2;dub(π)", 2 * pi);

    is!("global x=7", 7);
    is!("global x;x=7;x", 7);

    is!("global x=1;x=7;x+1", 8);

    // only the most primitive expressions are allowed in global initializers => move to main!
    // test_wasm_todos
    // is!("global x=1+π", 1 + pi);
    is!("global x=1+2", 3);

    is!("global x=7;x+=1", 8);
    is!("global x;x=7;x+=1", 8);
    is!("global x;x=7;x+=1;x+1", 9);
    skip(
        is!("global x=π;x=7;x", 7);
    );
    is!("global x;x=7;x", 7);
    is!("global x=1;x=7;x", 7);
}

#[test] fn test_get_local() {
    is!("add1 x:=it+1;add1 3", (int64) 4);
    skip(
        is!("add1 x:=$0+1;add1 3", (int64) 4); // $0 specially parsed now
    );
}

#[test] fn testWasmFunctionDefiniton() {
    //	assert_is("add1 x:=x+1;add1 3", (int64) 4);
    is!("fib:=if it<2 then it else fib(it-1)+fib(it-2);fib(7)", 13);
    is!("fac:= if it<=0 : 1 else it * fac it-1; fac(5)", 5 * 4 * 3 * 2 * 1);

    is!("add1 x:=x+1;add1 3", (int64) 4);
    is!("add2 x:=x+2;add2 3", (int64) 5);
    skip(
        is!("expression_as_return:=y=9;expression_as_return", (int64) 9);
        is!("addy x:= y=2 ; x+y ; addy 3", (int64) 5);
    );

    is!("grows x:=x*2;grows(4)", 8);
    is!("grows:=it*2; grows 3", 6);
    is!("grows:=it*2; grows 3*4", 24);
    is!("grows:=it*2; grows(3*42) > grows 2*3", 1);
    is!("factorial:=it<2?1:it*factorial(it-1);factorial 5", 120);

    //0 , 1 , 1 , 2 , 3 , 5 , 8 , 13 , 21 , 34 , 55 , 89 , 144
    is!("fib x:=if x<2 then x else fib(x-1)+fib(x-2);fib(7)", 13);
    is!("fib:=if it<2 then it else fib(it-1)+fib(it-2);fib(7)", 13);
    skip(
        is!("fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)", 13);
        is!("fib:=it<2 then it or fib(it-1)+fib(it-2);fib(7)", 13);
        is!("fib:=it<2 or fib(it-1)+fib(it-2);fib(4)", 5);
        is!("fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)", 5);
    );
}
#[test] fn testWasmTernary() {
    is!("2>1?3:4", 3);
    is!("1>0?3:4", 3);
    is!("2<1?3:4", 4);
    is!("1<0?3:4", 4);
    //	is!("(1<2)?10:255", 255);

    is!("fac:= it<=0 ? 1 : it * fac it-1; fac(5)", 5 * 4 * 3 * 2 * 1);
    skip(
        // What seems to be the problem?
    );
}
#[test] fn testLazyEvaluation() {
    //	if lazy_operators.has(op) and … not numeric? …
    //	if op==or emitIf(not lhs,then:rhs);
    //	if op==or emitIf(lhs,else:rhs);
    //	if op==and emitIf(lhs,then:rhs);
    is!("fac:= it<=0 or it * fac it-1; fac(5)", 5 * 4 * 3 * 2 * 1); // requires lazy evaluation
}

#[test] fn testWasmFunctionCalls() {
    // todo put square puti putf back here when it works!!
    skip(
        is!("puts 'ok'", (int64) 0);
    );
    is!("i=1;while i<9:i++;i+1", 10);
    is!("ceil 3.7", 4);

    is!("id(3*42) > id 2*3", 1);
    is!("id 123", (int64) 123);
    is!("id (3+3)", (int64) 6);
    assert_is("id 3+3", 6);
    is!("3 + id 3+3", (int64) 9);
}

#[test] fn testConstReturn() {
    is!(("42"), 42);
}

#[test] fn testPrint() {
    // does wasm print? (visual control!!);
    is!(("print 42"), 42);
    print("OK");
    //	printf("%llx\n", -2000000000000ll);
    //	printf("%llx", -4615739258092021350ll);
    print("a %d c"s % 3);
    print("a %f c"s % 3.1);
    print("a %x c"s % 15);
    printf("a %d c\n", 3);
    printf("a %f c\n", 3.1);
    printf("a %x c\n", 15);
}

#[test] fn testMathPrimitives() {
    skip(
        is!(("42.1"), 42.1) // todo: return &Node(42.1) or print value to stdout
        is!(("-42.1"), 42.1);
    );
    is!(("42"), 42);
    is!("-42", -42);
    skip(
        is!(("2000000000"), 2000000000) // todo stupid smart pointers
        is!(("-2000000000"), -2000000000);
    );
    is!(("2000000000000"), (int64) 2000000000000) // let int64
    is!(("-2000000000000"), (int64) -2000000000000L);

    is!("x=3;x*=3", 9);
    is!("'hello';(1 2 3 4);10", 10);
    //	data_mode = false;
    is!("i=ø; not i", true);
    is!("0.0", (int64) 0); // can't emit float yet
    is!(("x=15;x>=14"), 1);
    is!("i=1.0;i", 1.0); // works first time but not later in code :(
    is!("i=0.0;i", 0.0); //
    assert_is("3*-1", -3);
    is!("3*-1", -3);

    skip( // todo NOT SKIP!
        is!("maxi=3840*2160", 3840 * 2160);
        is!("maxi=3840*2160;maxi", 3840 * 2160);
        is!("blue=255;green=256*255;", 256 * 255);
    );
}

#[test] fn testFloatOperators() {
    assert_is(("42.0/2.0"), 21);
    is!(("3.0+3.0*3.0"), 12);
    is!(("42.0/2.0"), 21);
    is!(("42.0*2.0"), 84);
    is!(("42.0+2.0"), 44);
    is!(("42.0-2.0"), 40);
    is!(("3.0+3.0*3.0"), 12);
    is!(("3.1>3.0"), true);
    is!(("2.1<3.0"), true);
    is!("i=123.4;i", 123.4); // main returning int
    is!("i=1.0;i", 1.0);
    is!("i=3;i", 3);
    is!("i=1.0;i", 1.0);

    is!(("2.1<=3.0"), true);

    skip(
        is!("i=8;i=i/2;i", 4); // make sure i stays a-float
        is!("i=1.0;i=3;i=i/2;i=i*4", 6.0); // make sure i stays a-float
        "BUG IN WASM?? should work!?"
        is!(("3.1>=3.0"), true);
    );

    is!(("3.0+3.0*3.0>3.0+3.0+3.0"), true);
    is!(("3.0+3.0*3.0<3.0*3.0*3.0"), true);
    is!(("3.0+3.0*3.0<3.0+3.0+3.0"), false);
    is!(("3.0+3.0*3.0>3.0*3.0*3.0"), false) // 0x1.8p+1 == 3.0
    is!(("3.0+3.0+3.0<3.0+3.0*3.0"), true);
    is!(("3.0*3.0*3.0>3.0+3.0*3.0"), true);
}

#[test] fn testNorm2() {
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
    skip(
        is!("i=1;x=‖-3‖>i", 1);
        is!("f=4;x=‖-3‖<f", 1);
        is!("i=1;x=‖-3‖<i", 0);
        is!("f=4;x=‖-3‖>f", 0);
    );
}

#[test] fn testNorm() {
    testNorm2();
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

#[test] fn testMathOperators() {
    //	is!(("42 2 *"), 84);
    is!("- -3", 3);
    is!("1- -3", 4);
    is!("1 - -3", 4);
    skip(
        is!("1 - - 3", 4); // -1 uh ok?
        assert_throws("1--3"); // should throw, variable missed by parser! 1 OK'ish
    );

    //	is!("1--3", 4);// should throw, variable missed by parser! 1 OK'ish

    is!("‖-3‖", 3);
    is!("-‖-3‖", -3);
    is!("‖-3‖+1", 4);
    assert_is(("7%5"), 2);
    assert_is(("42/2"), 21);
    //			WebAssembly.Module doesn't validate: control flow returns with unexpected type. F32 is not a I32, in function at index 0
    is!(("42/2"), 21);
    is!(("42*2"), 84);
    is!(("42+2"), 44);
    is!(("42-2"), 40);
    is!(("3+3*3"), 12);
    is!(("3+3*3>3+3+3"), true);
    is!(("3+3*3<3*3*3"), true);
    is!(("3+3*3<3+3+3"), false);
    is!(("3+3*3>3*3*3"), false);
    is!(("3+3+3<3+3*3"), true);
    is!(("3*3*3>3+3*3"), true);
    is!("i=3;i*-1", -3);
    assert_is("3*-1", -3);
    is!("3*-1", -3);
    is!("-√9", -3);

    is!("i=3.7;.3+i", 4);
    is!("i=3.71;.3+i", 4.01);
#[cfg(feature = "WASM")]{
    is!("i=3.70001;.3+i", 4.0000100000000005); // lol todo?
#else
    is!("i=3.70001;.3+i", 4.00001);
}
    assert_is("4-1", 3); //

    is!("i=3;i++", 4);
    is!("- √9", -3);
    is!("i=-9;-i", 9);
#[cfg(feature = "WASM")]{
    is!("√ π ²", 3.141592653589793); // fu ;);
#else
    is!("√ π ²", 3.1415926535896688);
}
    is!(("3²"), 9);
    skip(
        is!(("3⁰"), 1); // get UNITY of set (1->e let cast ok?);
        is!(("3¹"), 3);
        is!(("3³"), 27); // define inside wasp!
        is!(("3⁴"), 9 * 9);
    );

    is!("i=3.70001;.3+i", 4);
    is!("i=3.7;.3+i", 4);
}

#[test] fn testMathOperatorsRuntime() {
    is!("3^2", 9);
    is!("3^1", 3);
    is!("42^2", 1764); // NO SUCH PRIMITIVE
    is!("√3^0", 1);
    is!("√3^0", 1.0);
#[cfg(feature = "WASM")]{
    is!("√3^2", 2.9999999999999996); // bad sqrt!?
    assert_is("π**2", (double) 9.869604401089358);
#else
    is!("√3^2", 3);
    assert_is("π**2", (double) 9.869604401089358);
}
}

#[test] fn testComparisonMath() {
    // may be evaluated by compiler!
    is!(("3*42>2*3"), 1);
    is!(("3*1<2*3"), 1);
    is!(("3*42≥2*3"), 1);
    is!(("3*2≥2*3"), 1);
    is!(("3*2≤2*3"), 1);
    is!(("3*2≤24*3"), 1);
    is!(("3*13!=14*3"), 1);
    is!(("3*13<=14*3"), 1);
    is!(("3*15>=14*3"), 1);
    is!(("3*42<2*3"), False);
    is!(("3*1>2*3"), False);
    is!(("3*452!=452*3"), False);
    is!(("3*13>=14*3"), False);
    is!(("3*15<=14*3"), False);
    is!(("3*42≥112*3"), false);
    is!(("3*2≥112*3"), false);
    is!(("3*12≤2*3"), false);
    is!(("3*112≤24*3"), false);

    //    is!(("3*452==452*3"), 1) // forces runtime
    //    is!(("3*13==14*3"), False);
}
#[test] fn testComparisonId() {
    // may be evaluated by compiler!
    is!("id(3*42 )> id 2*3", 1);
    is!("id(3*1)< id 2*3", 1);
    skip(
        is!("id(3*452)==452*3", 1);
        is!("452*3==id(3*452)", 1);
        is!("452*3==id 3*452", 1);
        is!("id(3*452)==452*3", 1);
        is!(("id(3*13)==14*3"), False);
    );
    is!(("id(3*42)≥2*3"), 1);
    is!(("id(3*2)≥2*3"), 1);
    is!(("id(3*2)≤2*3"), 1);
    is!(("id(3*2)≤24*3"), 1);
    is!(("id(3*13)!=14*3"), 1);
    is!(("id(3*13)<= id 14*3"), 1);
    is!(("id(3*13)<= id 14*3"), 1);

    is!(("id(3*15)>= id 14*3"), 1);
    is!(("id(3*42)< id 2*3"), False);
    is!(("id(3*1)> id 2*3"), False);
    is!(("id(3*452)!=452*3"), False);
    is!(("id(3*13)>= id 14*3"), False);
    is!(("id(3*15)<= id 14*3"), False);
    is!(("id(3*13)<= id 14*3"), 1);
    is!(("id(3*42)≥112*3"), false);
    is!(("id(3*2)≥112*3"), false);
    is!(("id(3*12)≤2*3"), false);
    is!(("id(3*112)≤24*3"), false);
}

#[test] fn testComparisonIdPrecedence() {
    // may be evaluated by compiler!
    skip(
        is!("id 3*452==452*3", 1) // forces runtime
        is!(("id 3*13==14*3"), False);

        //	Ambiguous mixing of functions `ƒ 1 + ƒ 1 ` can be read as `ƒ(1 + ƒ 1)` or `ƒ(1) + ƒ 1`
        is!("id 3*42 > id 2*3", 1);
        is!("id 3*1< id 2*3", 1);
    );
    is!("id(3*42)> id 2*3", 1);
    is!("id(3*1)< id 2*3", 1);
    is!(("id 3*42≥2*3"), 1);
    is!(("id 3*2≥2*3"), 1);
    is!(("id 3*2≤2*3"), 1);
    is!(("id 3*2≤24*3"), 1);
    is!(("id 3*13!=14*3"), 1);
    is!(("id 3*13<= id 14*3"), 1);
    is!(("id 3*13<= id 14*3"), 1);

    is!(("id 3*15>= id 14*3"), 1);
    is!(("id 3*42< id 2*3"), False);
    is!(("id 3*1> id 2*3"), False);
    is!(("id 3*452!=452*3"), False);
    is!(("id 3*13>= id 14*3"), False);
    is!(("id 3*15<= id 14*3"), False);
    is!(("id 3*13<= id 14*3"), 1);
    is!(("id 3*42≥112*3"), false);
    is!(("id 3*2≥112*3"), false);
    is!(("id 3*12≤2*3"), false);
    is!(("id 3*112≤24*3"), false);
}

#[test] fn testComparisonPrimitives() {
    is!(("42>2"), 1);
    is!(("1<2"), 1);
    is!(("42≥2"), 1);
    is!(("2≥2"), 1);
    is!(("2≤2"), 1);
    is!(("2≤24"), 1);
    is!(("13!=14"), 1);
    is!(("13<=14"), 1);
    is!(("15>=14"), 1);
    is!(("42<2"), False);
    is!(("1>2"), False);
    is!(("452!=452"), False);
    is!(("13>=14"), False);
    is!(("15<=14"), False);
    is!(("42≥112"), false);
    is!(("2≥112"), false);
    is!(("12≤2"), false);
    is!(("112≤24"), false);
#[cfg(not(feature = "WASM"))]{
    is!(("452==452"), 1) // forces runtime eq
    is!(("13==14"), False);
}
}

#[test] fn testWasmLogicPrimitives() {
    skip( // todo: if emit returns Node:
        is!(("false").name, False.name); // NO LOL emit only returns number
        is!(("false"), False);
    );

    is!("true", True);
    is!("true", true);
    is!("true", 1);

    is!("false", false);
    is!("false", False);
    is!("false", (int64) 0);

    is!("nil", false);
    is!("null", false);
    is!("null", (int64) 0);
    is!("null", (int64) nullptr);
    is!("ø", false);
    is!("nil", NIL);
}
#[test] fn testWasmVariables0() {
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
    skip(
        is!("i=0.0;i", 0.0);
        is!("i=ø;i", nullptr);
        is!("i=123.4;i", 123.4); // main returning int
    );
    is!("8.33333333332248946124e-03", 0); // todo in wasm
#[cfg(feature = "WASM")]{
    is!("8.33333333332248946124e+01", 83.33333333322489);
#else
    is!("8.33333333332248946124e+01", 83.3333333332248946124);
}

    is!("8.33333333332248946124e+03", 8333.33333332248946124);
    is!("S1  = -1.6666", -1.6666);
    //    is!("grows S1  = -1.6666", -1);
    // may be evaluated by compiler!
}

#[test] fn testWasmIncrement() {
    is!("i=2;i++", 3);
    skip(
        is!("i=0;w=800;h=800;pixel=(1 2 3);while(i++ < w*h){pixel[i]=i%2 };i ", 800 * 800);
        //				assert_error("i:=123;i++", "i is a closure, can't be incremented");
    );
}

#[test] fn testWasmLogicUnaryVariables() {
    is!("i=0.0; not i", true);
    is!("i=false; not i", true);
    is!("i=0; not i", true);
    skip(
        is!("i=true; not i", false);
    );
    is!("i=ø; not i", true);

    is!("i=1; not i", false);
    is!("i=123; not i", false);
}

#[test] fn testSelfModifying() {
    is!("i=3;i*=3", (int64) 9);
    is!("i=3;i+=3", (int64) 6);
    is!("i=3;i-=3", (int64) 0);
    is!("i=3;i/=3", (int64) 1);
    //	is!("i=3;i√=3", (int64) ∛3); NO i TIMES √
    skip(
        is!("i=3^1;i^=3", (int64) 27);
        assert_throws("i*=3"); // well:
        is!("i*=3", (int64) 0);
    );
}

#[test] fn testWasmLogicUnary() {
    is!("not 0.0", true);
    is!("not ø", true);
    is!("not false", true);
    is!("not 0", true);

    is!("not true", false);
    is!("not 1", false);
    is!("not 123", false);
}

#[test] fn testWasmLogicOnObjects() {
    is!("not 'a'", false);
    is!("not {a:2}", false);
    skip(
        is!("not {a:0}", false); // maybe
    );

    is!("not ()", true);
    is!("not {}", true);
    is!("not []", true);
    is!("not ({[ø]})", true); // might skip :);
}

#[test] fn testWasmLogic() {
    skip(
        // should be easy to do, but do we really want this?
        is!("true true and", true);
        is!("false true and", false);
        is!("false false and ", false);
        is!("true false and ", false);
        assert!(parse("false and false").length == 3);
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

#[test] fn testWasmLogicNegated() {
    is!("not true and not true", not true);
    is!("not true and not false", not true);
    is!("not false and not true", not true);
    is!("not false and not false", not false);
    is!("not false or not true and not true", not false); // == not false or (not true);

    is!("not true xor not false", not false);
    is!("not false xor not true", not false);
    is!("not true xor not true", not true);
    is!("not false xor not false", not true);
    is!("not true or not false", not false);
    is!("not true or not true", not true);
    is!("not false or not true", not false);
    is!("not false or not false", not false);
}

#[test] fn testWasmLogicCombined() {
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

#[test] fn testWasmIf() {
    is!("if 2 : 3 else 4", 3);
    is!("if 2 then 3 else 4", 3);
    skip(
        is!("if(2){3}{4}", 3);
        is!("if({2},{3},{4})", 3);
        is!("if(2,3,4)", 3); // bad border case EXC_BAD_ACCESS because not anayized!
        is!("if(condition=2,then=3)", 3);
        is!("if(condition=2,then=3,else=4)", 3); // this is what happens under the hood (?);
        is!("fib:=it<2 then 1 else fib(it-1)+fib(it-2);fib(4)", 5);
        is!("fib:=it<2 and it or fib(it-1)+fib(it-2);fib(7)", 13);
        is!("fib:=it<2 then it or fib(it-1)+fib(it-2);fib(7)", 13);
        is!("fib:=it<2 or fib(it-1)+fib(it-2);fib(4)", 5);
    );
}

#[test] fn testWasmWhile() {
    is!("i=1;while i<9:i++;i+1", 10);
    is!("i=1;while(i<9){i++};i+1", 10);
    is!("i=1;while(i<9 and i > -10){i+=2;i--};i+1", 10);
    is!("i=1;while(i<9)i++;i+1", 10);
    is!("i=1;while i<10 do {i++};i", 10);
    is!("i=1;while i<10 and i<11 do {i++};i", 10);
    is!("i=1;while i<9 or i<10 do {i++};i", 10);
    is!("i=1;while(i<10) do {i++};i", 10);
    skip( // fails on 2nd attempt todo
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);
    );
    is!("i=1;while(i<9)i++;i+1", 10);
}
#[test] fn testWasmMemoryIntegrity() {
    return;
#[cfg(not(feature = "WASM"))]{
}

    if (!MAX_MEM) {
        error("NO MEMORY");
    }
    printf("MEMORY start at %lld\n", (int64) memory);
    printf("current start at %lld\n", (int64) heap_end);
    //	Bus error: 10  if i > MEMORY_SIZE
    // Fails at 100000, works at 100001 WHERE IS THIS SET?
    //	int start=125608;
    int start = __heap_base;
    int64 end = 0x1000000; // MAX_MEM / 4; // /4 because 1 int = 4 bytes
    for (int i = start; i < end; ++i) {
        int tmp = memory[i];
        //		memory[i] = memory[i]+1;
        //		memory[i] = memory[i]-1;
        memory[i] = i;
        //		if(i%10000==0)logi(i);// logi USES STACK, so it can EXHAUST if called too often!
        if (memory[i] != i) {
            printf("MEMORY CORRUPTION at %d", i);
            proc_exit(0);
        }
        memory[i] = tmp; // else test fail
    }
}

#[test] fn testSquarePrecedence() {
    // todo!
    is!("π/2^2", pi / 4);
    is!("(π/2)^2", pi * pi / 4);
}

#[test] fn testSquares() {
    // occasionally breaks in browser! even though right code is emitted HOW??
    is!("square 3", 9);
    is!("1+2 + square 1+2", (int64) 12);
    is!("1+2 + square 3+4", (int64) 52);
    is!("4*5 + square 2*3", (int64) 56);
    is!("3 + square 3", (int64) 12);
    is!("1 - 3 - square 3+4", (int64) -51); // OK!
    is!("square(3*42) > square 2*3", 1);
    skip(
        testSquarePrecedence();
    );
}

// ⚠️ CANNOT USE is!
 in WASM! ONLY via testRun();
#[test] fn testOldRandomBugs() {
    // ≈ testRecentRandomBugs();
    // some might break due some testBadInWasm() BEFORE!
    is!("-42", -42) // OK!?!
    skip(
        is!("x:=41;if x>1 then 2 else 3", 2);
        is!("x=41;if x>1 then 2 else 3", 2);
        is!("x:41;if x>1 then 2 else 3", 2);
        is!("x:41;if x<1 then 2 else 3", 3);
        is!("x:41;x+1", 42);
        is!("grows := it * 2 ; grows(4)", 8);
        is!("grows:=it*2;grows(4)", 8);
    );

    //		testGraphQlQuery();
    //	assert_is("x", Node(false));// passes now but not later!!
    //	assert_is("x", false);// passes now but not later!!
    //	assert_is("y", false);
    //	assert_is("x", false);

    //0 , 1 , 1 , 2 , 3 , 5 , 8 , 13 , 21 , 34 , 55 , 89 , 144

    //	exit(1);
    //	const Node &node1 = parse("x:40;x++;x+1");
    //	assert!(node.length==3);
    //	assert!(node[0]["x"]==40);
    //	exit(1);
}

//#[test] fn testRefactor(){ 
//	wabt::Module *module = readWasm("t.wasm");
//	refactor_wasm(module, "__original_main", "_start");
//	module = readWasm("out.wasm");
//	assert!(module->funcs.front()->name == "_start");
//}

#[cfg(feature = "WABT_MERGE")]{
//
}

#[test] fn testMergeWabt() {
#[cfg(feature = "WABT_MERGE")]{
    merge_files({"test/merge/main.wasm", "test/merge/lib.wasm"});
}
}
#[test] fn testMergeWabtByHand() {
#[cfg(feature = "WABT_MERGE")]{ // ?? ;);
    merge_files({"./playground/test-lld-wasm/main.wasm", "./playground/test-lld-wasm/lib.wasm"});
    wabt::Module *main = readWasm("test-lld-wasm/main.wasm");
    wabt::Module *module = readWasm("test-lld-wasm/lib.wasm");
    refactor_wasm(module, "b", "neu");
    remove_function(module, "f");
    Module *merged = merge_wasm2(main, module);
    save_wasm(merged);
    int ok = run_wasm(merged);
    int ok = run_wasm("a.wasm");
    assert!(ok == 42);
}
}
#[test] fn testWasmRuntimeExtension() {
#[cfg(feature = "TRACE")]{
    printf("TRACE mode currently SIGTRAP's in testWasmRuntimeExtension. OK, Switch to Debug mode. WHY though?");
}

    assert_run("43", 43);
    is!("strlen('123')", 3); // todo broke
    assert_run("strlen('123')", 3); // todo broke
    skip(
        //            todo polymorphism
        assert_run("len('123')", 3);
        assert_run("len('1235')", 4);
    );
    assert_run("parseLong('123')", 123);
    assert_run("parseLong('123'+'456')", 123456);
#[cfg(not(feature = "TRACE"))]{ // todo why??
    assert_run("parseLong('123000') + parseLong('456')", 123456);
    assert_run("x=123;x + 4 is 127", true);
    assert_run("parseLong('123'+'456')", 123456);
    assert_run("'123' is '123'", true);
    assert_run("'123' + '4' is '1234'", true); // ok
}
    assert_throws("not_ok"); // error
    skip(
        // WORKED before we moved these to test_functions.h
        // todo activate in wasp-runtime-debug.wasm instead of wasp-runtime.wasm
        assert_run("test42+1", 43);
        assert_run("test42i(1)", 43);

        assert_run("test42f(1)", 43);
        assert_run("test42f(1.0)", 43.0);
        assert_run("42.5", 42.5); // truncation ≠ proper rounding!
        assert_run("42.6", 42.6); // truncation ≠ proper rounding!
        assert_run("test42f(1.7)", 43.7);
        assert_run("test42f", 41.5); //default args don't work in wasm! (how could they?);
        assert_run("test42f", 41.5); /// … expected f32 but nothing on stack
    );
    //	functionSignatures["int"].returns(int32);
    //	assert_run("printf('123')", 123);
    // works with ./wasp but breaks in webapp
    // works with ./wasp but breaks now:

    //	assert_run("okf(1)", 43);
    //	assert_run("puts 'hello' 'world'", "hello world");
    //	assert_run("hello world", "hello world");// unresolved symbol printed as is

    skip(
        assert_run("x=123;x + 4 is 127", true);
        //	assert_run("'123'='123'", true);// parsed as key a:b !?!? todo!
        //	assert_run("'123' = '123'", true);
    );
    assert_run("'123' == '123'", true);
    assert_run("'123' is '123'", true);
    assert_run("'123' equals '123'", true);
    assert_run("x='123';x is '123'", true);
    //	assert_run("string('123') equals '123'", true); // string() makes no sense in angle:
    //	assert_run("'123' equals string('123')", true);//  it is internally already a string whenever needed
    //	assert_run("atoi0(str('123'))", 123);
    //	assert_run("atoi0(string('123'))", 123);

    //	assert_run("oki(1)", 43);
    //	is!("puts('123'+'456');", 123456);// via import not via wasp!
    //is!("grows := it * 2 ; grows(4)", 8);
    //	assert!(Primitive::charp!=Valtype::pointer);

    skip(
        assert_run("'123'", 123); // result printed and parsed?
        assert_run("printf('123')", 123); // result printed and parsed?
    );
    skip( // if not compiled as RUNTIME_ONLY library:
        assert!(functionSignatures.has("tests"));
        assert_run("tests", 42);
    );
}

#[test] fn testStringConcatWasm() {
    is!("'Hello, ' + 'World!'", "Hello, World!");
}

#[test] fn testStringIndicesWasm() {
    is!("'abcde'#4", 'd'); //
    is!("x='abcde';x#4", 'd'); //
    is!("x='abcde';x#4='x';x#4", 'x');

    is!("x='abcde';x#4='x';x#4", 'x');
    is!("x='abcde';x#4='x';x#5", 'e');

    is!("x='abcde';x#4='x';x[3]", 'x');
    is!("x='abcde';x#4='x';x[4]", 'e');
    is!("i=0;x='abcde';x#4='x';x[4]", 'e');

    is!("'hello';(1 2 3 4);10", 10); // -> data array […;…;10] ≠ 10

    //	is!("'world'[1]", 'o');
    is!("'world'#1", 'w');
    is!("'world'#2", 'o');
    is!("'world'#3", 'r');
    skip( // todo move angle syntax to test_angle
        is!("char #1 in 'world'", 'w');
        is!("char 1 in 'world'", 'w');
        is!("2nd char in 'world'", 'o');
        is!("2nd byte in 'world'", 'o');
        is!("'world'#-1", 'd');
    );

    is!("hello='world';hello#1", 'w');
    is!("hello='world';hello#2", 'o');
    //	is!("pixel=100 int(s);pixel#1=15;pixel#1", 15);
    skip(
        is!("hello='world';hello#1='W';hello#1", 'W'); // diadic ternary operator
        is!("hello='world';hello[0]='W';hello[0]", 'W'); // diadic ternary operator
    );
    //	is!("hello='world';hello#1='W';hello", "World");
    //	exit(0);
}

#[test] fn testObjectPropertiesWasm() {
    is!("x={a:3,b:4,c:{d:true}};x.a", 3);
    is!("x={a:3,b:true};x.b", 1);
    is!("x={a:3,b:4,c:{d:true}};x.c.d", 1);
    //is!("x={a:3,b:'ok',c:{d:true}};x.b", "ok");
    is!("x={a:3,b:'ok',c:{d:5}};x.c.d", 5); //deep
}

#[test] fn testArrayIndicesWasm() {
#[cfg(not(feature = "WEBAPP"))]{
    assert_throws("surface=(1,2,3);i=1;k#i=4;k#i") // no such k!
    //	caught in wrong place?
}

    //	testArrayIndices(); //	assert! node based (non-primitive) interpretation first
    //	data_mode = true;// todo remove hack
    is!("x={1 2 3}; x#3=4;x#3", 4);
#[cfg(feature = "WASM")]{
    is!("puts('ok');", -1); // todo: fix puts return
#elif WASMEDGE
    is!("puts('ok');", 8);
#else
    is!("puts('ok');", 0);
}
    is!("puts('ok');(1 4 3)#2", 4);
    is!("{1 4 3}#2", 4);

    is!("x={1 4 3};x#2", 4);
    is!("{1 4 3}[1]", 4);
    is!("(1 4 3)[1]", 4);
    assert_throws("(1 4 3)#0");

#[cfg(not(feature = "WASM"))]{ // TODO!
    is!("'αβγδε'#3", U'γ');
    is!("i=3;k='αβγδε';k#i", u'γ');
}
    skip(
        is!("i=3;k='αβγδε';k#i='Γ';k#i", u'Γ'); // todo setCharAt
        is!("[1 4 3]#2", 4); // exactly one op expected in emitIndexPattern
        assert_is("[1 2 3]#2", 2); // assert! node based (non-primitive) interpretation first
        assert_throws("(1 4 3)#4"); // todo THROW!
        // todo patterns as lists
    );
    //	Node empty_array = parse("pixel=[]");
    //	assert!(empty_array.kind==patterns);
    //
    //	Node construct = analyze(parse("pixel=[]"));
    //	assert!(construct["rhs"].kind == patterns or construct.length==1 and construct.first().kind==patterns);
    //	emit("pixel=[]");
    //	exit(0);
}
// random stuff todo: put in proper tests
#[test] fn testWasmStuff() {
    //	is!("grows := it * 2 ; grows(4)", 8);
    is!("-42", -42);
    is!("x=41;x+1", 42);
    is!("x=40;y=2;x+y", 42);
    is!("id(4*42) > id 2+3", 1);
    skip(
        is!("grows x := x * 2 ; grows(4)", 8);
        is!("grows := it * 2 ; grows(4)", 8);
        is!("grows:=it*2; grows 3", 6);
        is!("add1 x:=x+1;add1 3", (int64) 4);
        is!("fib x:=if x<2 then x else fib(x-1)+fib(x-2);fib(7)", 13);
        is!("fib x:=if x<2 then x else{fib(x-1)+fib(x-2)};fib(7)", 13);
    );
}

bool testRecentRandomBugsAgain = true;

// ⚠️ CANNOT USE is!
 in WASM! ONLY via #[test] fn testRun();
#[test] fn testRecentRandomBugs() {
    // fixed now thank god
    if (!testRecentRandomBugsAgain)return;
    testRecentRandomBugsAgain = false;
    is!("-42", -42);
    is!("‖3‖-1", 2);
#[cfg(not(feature = "WASMTIME"))]{
    is!("test42+1", 43); // OK in WASM too? todo
    is!("square 3*42 > square 2*3", 1);
#[cfg(not(feature = "WASM"))]{
    testSquares();
}
}
    //			WebAssembly.Module doesn't validate: control flow returns with unexpected type. F32 is not a I32, in function at index 0
    assert_is(("42/2"), 21) // in WEBAPP

    is!(("42.1"), 42.1);
    // main returns int, should be pointer to value! result & array_header_32 => smart pointer!
    //			Ambiguous mixing of functions `ƒ 1 + ƒ 1 ` can be read as `ƒ(1 + ƒ 1)` or `ƒ(1) + ƒ 1`
    is!("id 3*42 > id 2*3", 1);
    is!("1-‖3‖/-3", 2);
    is!("i=true; not i", false);
    // these fail LATER in tests!!

    skip(
        testLengthOperator();
        is!("i=3^1;i^=3", (int64) 27);
        assert_throws("i*=3"); // well:
        is!("i*=3", (int64) 0);
    );
    is!("maxi=3840*2160", 3840 * 2160);
    is!("√π²", 3);
    is!("i=-9;√-i", 3);
    is!("1- -3", 4);
    is!("width=height=400;height", 400);
    skip(
        assert_throws("1--3"); // should throw, variable missed by parser! 1 OK'ish
        is!("x=0;while x++<11: nop;x", 11);
        assert_throws("x==0;while x++<11: nop;x");
    );
    is!("‖-3‖", 3);
    is!("√100²", 100);
    //    is!("puts('ok');", 0);
    assert_parsesx("{ç:☺}");
    assert(result["ç"] == "☺");
#[cfg(not(feature = "WASMTIME"))]{ and not LINUX // todo why
    assert_run("x=123;x + 4 is 127", true);
    is!("n=3;2ⁿ", 8);
    //	function attempted to return an incompatible value WHAT DO YOU MEAN!?
}
    // move to tests() once OK'
    skip(
        is!("i=ø; not i", true); // i not a setter if value ø
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);
    );
    is!("add1 x:=x+1;add1 3", (int64) 4);
    // is!("for i in 1 to 5 : {puti i};i", 6);// EXC_BAD_ACCESS TODO!!
}
#[test] fn testSquareExpWasm() {
    let π = pi; //3.141592653589793;
    // todo smart pointer return from main for floats!
    is!("3²", 9);
    is!("3.0²", 9);
    is!("√100²", 100);
    is!("√ π ²", π);
    is!("√π ²", π);
    is!("√ π²", π);
    is!("√π²", π);
    is!("π²", π * π);
    is!("π", pi);
    is!("int i=π*1000000", 3141592);
#[cfg(feature = "WASM")]{
    is!("π*1000000.", 3141592.653589793);
#else
    is!("π*1000000.", 3141592.6535897);
}
    is!("i=-9;-i", 9);
    is!("- √9", -3);
    is!(".1 + .9", 1);
    is!("-.1 + -.9", -1);
    is!("√9", 3);
    //	is!("√-9 is -3i", -3);// if «use complex numbers»
    is!(".1", .1);
#[cfg(not(feature = "WASMTIME"))]{ and not LINUX // todo why
    skip(
        is!("i=-9;√-i", 3);
    is!("n=3;2ⁿ", 8);
    is!("n=3.0;2.0ⁿ", 8);
    //	function attempted to return an incompatible value WHAT DO YOU MEAN!?
    );
}
}

#[test] fn testRoundFloorCeiling() {
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
#[test] fn testWasmTypedGlobals() {
    //    is!("global int k", 7);//   empty global initializer for int
    is!("global long k=7", 7);
    //    is!("global int k=7", 7); // type mismatch
    is!("global const int k=7", 7); //   all globals without value are imports??
    is!("global mutable int k=7", 7); //   all globals without value are imports??
    is!("global mut int k=7", 7); //   all globals without value are imports??
}

#[test] fn testWasmMutableGlobal() {
    //	is!("$k=7",7);// ruby style, conflicts with templates `hi $name`
    //    is!("k::=7", 7);// global variable not visually marked as global, not as good as:
    is!("global k=7", 7); // python style, as always the best
    is!("global k:=7", 7); //  global or function?
    is!("global k;k = 7", 7); // python style, as always the best
    //    is!("global.k=7", 7);//  currently all globals are exported
    skip(testWasmMutableGlobal2());
    skip(testWasmTypedGlobals());
    //    testWasmMutableGlobalImports();
}

#[test] fn testWasmMutableGlobal2() {
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

#[test] fn testWasmMutableGlobalImports() {
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
    // this does not impede the above, as global exports are not properties, but something to keep in mind
}

#[test] fn testCustomOperators() {
    is!(("suffix operator ⁰ := 1; 3⁰"), 1); // get UNITY of set (1->e let cast ok?);
    is!(("suffix ⁰ := 1; 3⁰"), 1); // get UNITY of set (1->e let cast ok?);
    is!(("suffix operator ³ := it*it*it; 3³"), 27); // define inside wasp!
    is!(("suffix operator ³ := it*it*it; .5³"), 1 / 8);
    is!(("suffix ³ := it*it*it; 3³"), 27); // define inside wasp!

    //	is!(("alias to the third = ³"),1);
    //	is!(("3⁴"),9*9);
}

#[test] fn testIndexWasm() {
    is!("i=1;k='hi';k#i", 'h'); // BUT IT WORKS BEFORE!?! be careful with i64 smarty return!
    is!("i=1;k='hi';k[i]", 'i');
    //	assert_throws("i=0;k='hi';k#i")// todo internal boundary assert!s? nah, later ;) done by VM:
    // WASM3 error: [trap] out of bounds memory accessmemory size: 65536; access offset: 4294967295
    is!("k='hi';k#1=97;k#1", 'a');
    is!("k='hi';k#1='a';k#1", 'a');
    is!("k='hi';i=1;k#i=97;k#i", 'a');
    is!("k=(1,2,3);i=1;k#i=4;k#i", 4);
    is!("k=(1,2,3);i=1;k#i=4;k#1", 4);

    is!("k='hi';k#1=65;k#2", 'i');
    is!("k=(1,2,3);i=1;k#i=4;k#i", 4);
    is!("i=2;k='hio';k#i", 'i');
}
#[test] fn testImportWasm() {
    //	Code fourty_two=emit(analyze(parse("ft=42")));
    //	fourty_two.save("fourty_two.wasm");
    is!("import fourty_two;ft*2", 42 * 2);
    is!("import fourty_two", 42);
    is!("include fourty_two", 42);
    is!("require fourty_two", 42);
    is!("include fourty_two;ft*2", 42 * 2);
    is!("require fourty_two;ft*2", 42 * 2);
}

#[test] fn testMathLibrary() {
    // todo generic power i as builtin
#[cfg(not(feature = "WASMTIME"))]{
    skip(
        // REGRESSION 2023-01-20 variable x-c in context wasp_main emitted as node data:
        is!("x=3;y=4;c=1;r=5;((‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);
    );
}
    is!("i=-9;√-i", 3);
    is!("i=-9;√ -i", 3);
    //		is!("use math;√π²", 3);
}

#[test] fn testSmartReturnHarder() {
    is!("'a'", Node('a'));
    is!("'a'", Node(u'a'));
    is!("'a'", Node(U'a'));
    is!("'a'", String('a'));
    is!("'a'", String(u'a'));
    is!("'a'", String(U'a'));
    //    is!("'a'", 'a'); // … should be 97
    //    is!("'a'", u'a');
    //    is!("'a'", U'a');
    is!("10007.0%10000.0", 7);
    is!("10007.0%10000", 7);
#[cfg(not(feature = "WASM"))]{
    is!("x='abcde';x#4='f';x[3]", 'f');
    is!("x='abcde';x#4='x';x[3]", 'x');
    is!("x='abcde';x[3]", 'd');
}
    //    is!("x='abcde';x[3]", (int) 'd');// currently FAILS … OK typesafe!
}
#[test] fn testSmartReturn() {
#[cfg(not(feature = "WASM"))]{
    testSmartReturnHarder(); // todo
}

    is!("1", 1);
    is!("-2000000000000", (int64) -2000000000000l);
    is!("2000000000000", (int64) 2000000000000l) // let int64
    is!("42.0/2.0", 21);
    is!("42.0/2.0", 21.);
    is!("- √9", -3);
    is!("42/4.", 10.5);
    skip(
        is!("42/4", 10.5);
    );

    assert_is(("42.0/2.0"), 21);

    is!(("-1.1"), -1.1);
    is!("'OK'", "OK");
}
#[test] fn testMultiValue() {
#[cfg(feature = "MULTI_VALUE")]{
    is!("1,2,3", Node(1, 2, 3, 0));
    is!("1;2;3", 3);
    is!("'OK'", "OK");
}
}

#[test] fn testAssertRun() {
    // all these have been tested with is!
 before. now assert! that it works with runtime
    testWasmRuntimeExtension();

    assert_run("42", 42);
    assert_run("x=123;x + 4 is 127", true); //  assert_run sometimes causes Heap corruption! test earlier
    assert_run("x='123';x is '123'", true); // ok
    assert_run("'hello';(1 2 3 4);10", 10); // -> data array […;…;10] ≠ 10
#[cfg(not(feature = "TRACE"))]{
    assert_run("x='123';x + '4' is '1234'", true); // ok
    assert_run("'123' + '4' is '1234'", true); // ok needs runtime for concat();
    assert_run("x='123';x=='123'", true); // ok needs runtime for eq();
}
}
#[test] fn testLogarithm() {
    skip(
        is!("use log; log10(100)", 2.);
    );
}

#[test] fn testLogarithm2() {
    //	float ℯ = 2.7182818284590;
    Function &function = functions["log10"];
    assert!(function.is_import);
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
    skip(
        eq!(-ln(0), Infinity);
        eq!(ln(0), -Infinity);
        is!("ln(ℯ)", 1.);
    );
}

#[test] fn testForLoopClassic() {
    is!("for(i=0;i<10;i++){puti i};i", 10);
    is!("sum = 0; for(i=0;i<10;i++){sum+=i};sum", 45);
}

#[test] fn testForLoops() {
#[cfg(not(feature = "WASM"))]{ // todo: fix for wasm
    testForLoopClassic();
}
    // is!("for i in 1 to 5 : {print i};i", 6);
    // todo: generic dispatch print in WasmEdge
#[cfg(feature = "WASM")]{ // cheat!
    is!("for i in 1 to 5 : {print i};i", 6);
    is!("for i in 1 to 5 : {print i};i", 6); // EXC_BAD_ACCESS as of 2025-03-06 under SANITIZE
    is!("for i in 1 to 5 {print i}", 5);
    is!("for i in 1 to 5 {print i};i", 6); // after loop :(
    is!("for i in 1 to 5 : print i", 5);
    is!("for i in 1 to 5\n  print i", 5);
    // is!("for i in 1 to 5\n  print i\ni", 6);
#else // todo : why puti not in WASM??
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
    skip(
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
#[test] fn testAssert() {
    is!("assert 1", 1);
    assert_throws("assert 0"); // todo make wasm throw, not compile error?
}
// test once by looking at the output wasm/wat
#[test] fn testNamedDataSections() {
    is!("fest='def';test='abc'", "abc");
    exit(0);
}

#[test] fn testAutoSmarty() {
    is!("11", 11);
    is!("'c'", 'c');
    is!("'cc'", "cc");
    is!("π", pi);
    //    is!("{a:b}", new Node{.name="a"));
}

#[test] fn testArguments() {
    is!("#params", 0); // no args, but create empty List anyway
    // todo add context to wasp variable $params
}

#[test] fn testFibonacci() {
    is!("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
    is!("int fib(int n){n < 2 ? n : fib(n - 1) + fib(n - 2)}\nfib(10)", 55);
    skip( // TODO!!!
        is!("fib(int n) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
        is!("fib(int n) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
        is!("fib(number n) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
        is!("fib(n) = n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
        is!("fib(n){n < 2 ? n : fib(n - 1) + fib(n - 2)}\nfib(10)", 55);
        is!("fib(n) := n < 2 ? n : fib(n - 1) + fib(n - 2)\nfib(10)", 55);
        is!("fib = it < 2 ? 1 : fib(it - 1) + fib(it - 2)\nfib(10)", 55);
        // todo worked until number was own type
        is!("fib number := if number<2 : 1 else fib(number - 1) + fib it - 2;fib(9)", 55); // home.md MUST WORK
    );
}

#[test] fn testHostDownload() {
#[cfg(not(feature = ""))]{WASMEDGE
    is!("download http://pannous.com/files/test", "test 2 5 3 7");
}
}
#[test] fn testSinus2() {
    is!(r#"double sin(double x){
    x = modulo_double(x,tau);
    double z = x*x
    double w = z*z
    S1  = -1.66666666666666324348e-01,
    S2  =  8.33333333332248946124e-03,
    S3  = -1.98412698298579493134e-04,
    S4  =  2.75573137070700676789e-06,
    S5  = -2.50507602534068634195e-08,
    S6  =  1.58969099521155010221e-10
    if(x >= pi) return -sin(modulo_double(x,pi));
    double r = S2 + z*(S3 + z*S4) + z*w*(S5 + z*S6);
    return x + z*x*(S1 + z*r);
}; sin π/2"#, 1); // IT WORKS!!!
}

#[test] fn testSinus() {
    //k=78; fucks it up!!
    is!("double sin(double x){\n"
                "\tx = modulo_double(x,tau)\n"
                "\tdouble z = x*x\n"
                "\tdouble w = z*z\n"
                "\tS1  = -1.66666666666666324348e-01, /* 0xBFC55555, 0x55555549 */\n"
                "\tS2  =  8.33333333332248946124e-03, /* 0x3F811111, 0x1110F8A6 */\n"
                "\tS3  = -1.98412698298579493134e-04, /* 0xBF2A01A0, 0x19C161D5 */\n"
                "\tS4  =  2.75573137070700676789e-06, /* 0x3EC71DE3, 0x57B1FE7D */\n"
                "\tS5  = -2.50507602534068634195e-08, /* 0xBE5AE5E6, 0x8A2B9CEB */\n"
                "\tS6  =  1.58969099521155010221e-10  /* 0x3DE5D93A, 0x5ACFD57C */\n"
                //	            "\ttau =  6.283185307179586 // 2π\n"
                "\tif(x >= pi) return -sin(modulo_double(x,pi))\n"
                "\tdouble r = S2 + z*(S3 + z*S4) + z*w*(S5 + z*S6)\n"
                "\treturn x + z*x*(S1 + z*r)\n"
                "};sin π/2", 1.0000000002522271); // IT WORKS!!! todo: why imprecision?
    //    exit(1);
}
#[test] fn testEmitBasics() {
    is!("true", true);
    is!("false", false);
    is!("8.33333333332248946124e-03", 8.33333333332248946124e-03);
    is!("42", 42);
    is!("-42", -42);
    is!("3.1415", 3.1415);
    is!("-3.1415", -3.1415);
    is!("40", 40);
    is!("41", 41);
    is!("1 ∧ 0", 0);
    skip(
        // see testSmartReturn
        is!("'ok'", "ok"); // BREAKS wasm !!
        is!("'a'", "a");
        is!("'a'", 'a');
    );
}
#[test] fn testMathExtra() {
    assert_is("15÷5", 3);
    is!("15÷5", 3);
    is!("3⋅5", 15);
    is!("3×5", 15);
    skip(
        is!("3**3", 27);
        is!("√3**2", 3);
        is!("3^3", 27);
        is!("√3^2", 3); // in test_squares
        assert_is("one plus two times three", 7);
    );
}

#[test] fn testRoot() {
    skip(
        assert_is("40+√4", 42, 0);
        assert_is("√4", 2);
        assert_is("√4+40", 42);
        assert_is("40 + √4", 42);
    ); // todo tokenized as +√
}

#[test] fn testRootFloat() {
    //	skip(  // include <cmath> causes problems, so skip
    assert_is("√42.0 * √42.0", 42.);
    assert_is("√42 * √42.0", 42.);
    assert_is("√42.0*√42", 42);
    assert_is("√42*√42", 42); // round AFTER! ok with f64! f32 result 41.99999 => 41
}
#[test] fn testNodeDataBinaryReconstruction() {
    eq!(parse("y:{x:2 z:3}").serialize(), "y{x:2 z:3}"); // todo y:{} vs y{}
    is!("y:{x:2 z:3}", parse("y:{x:2 z:3}")); // looks trivial but is epitome of binary (de)serialization!
}
#[test] fn testWasmString() {
#[cfg(feature = "WASM")]{
    return; // todo!
}
    is!("“c”", 'c');
    is!("“a”", "a");
    is!("“b”", "b");
    is!("\"d\"", 'd');
    is!("'e'", 'e');
#[cfg(feature = "WASM")]{
    is!("'f'", u'f');
    is!("'g'", U'g');
}
    is!("'h'", "h");
    is!("\"i\"", "i");
    is!("'j'", Node("j"));
#[cfg(not(feature = "WASM"))]{ // todo
    wasm_string x = reinterpret_cast<wasm_string>("\03abc");
    String y = String(x);
    assert!(y == "abc");
    assert!(y.length == 3);
    is!("“hello1”", Node(String("hello1"))); // Invalid typed array length: 12655
    is!("“hello2”", Node("hello2").setKind(strings)); // Invalid typed array length: 12655
    is!("“hello3”", Node("hello3"));
    is!("“hello4”", "hello4");
}
}
#[test] fn testFixedInBrowser() {
    testMathOperatorsRuntime(); // 3^2
    testIndexWasm();
    testStringIndicesWasm();
    is!("(2+1)==(4-1)", true); // suddenly passes !? not with above line commented out BUG <<<
    is!("(3+1)==(5-1)", true);
    assert_is("(2+1)==(4-1)", true);
    is!("3==2+1", 1);
    is!("3 + √9", (int64) 6);
    is!("puti 3", (int64) 3);
    is!("puti 3", 3); //
    is!("puti 3+3", 6);
    // #[cfg(feature = "WASM")]{
    //     return;
    // }

    testWasmString(); // with length as header
    is!("x='abcde';x[3]", 'd');
    testCall();
    testArrayIndicesWasm();
    testSquarePrecedence();
}
//testWasmControlFlow

#[test] fn testBadInWasm();

// SIMILAR AS:
#[test] fn testTodoBrowser() {
    testFixedInBrowser();
    testOldRandomBugs(); // currently ok

    skip( // still breaking! (some for good reason);
        // OPEN BUGS
        testBadInWasm(); // NO, breaks!
    );
}
// ⚠️ ALL tests containing is!
 must go here! testCurrent() only for basics
#[test] fn testAllWasm() {
    // called by testRun() OR synchronously!
    is!("42", 42);
    is!("42+1", 43);
    // assert_run("test42+2", 44); // OK in WASM too ? deactivated for now
    testSinus(); // still FRAGILE!

    testAssertRun();
    testTodoBrowser(); // TODO!
    skip(
        is!("putf 3.1", 3);
        is!("putf 3.1", 3.1);
    );

    skip(
        testWasmGC(); // WASM EDGE Error message: type mismatch
        testStruct(); // TODO get pointer of node on stack
        testStruct2();
    );
#[cfg(feature = "WEBAPP")]{ or MY_WASM
    testHostDownload();
}
    // Test that IMPLICITLY use runtime /  assert_run
    // is!("x=(1 4 3);x#2", 4);
    // is!("n=3;2ⁿ", 8);
    // is!("k=(1,2,3);i=1;k#i=4;k#i", 4);

    is!("√9*-‖-3‖/-3", 3);
    skip(
        is!("x=3;y=4;c=1;r=5;((‖(x-c)^2+(y-c)^2‖<r)?10:255", 255);
        is!("i=3;k='αβγδε';k#i='Γ';k#i", u'Γ'); // todo setCharAt
        testGenerics();
    );
    testImplicitMultiplication(); // todo in parser how?
    testForLoops();
    testGlobals();
    testFibonacci();
    testAutoSmarty();
    testArguments();
    skip(
        testWasmGC();
        is!("τ≈6.2831853", true);
        eq!("τ≈6.2831853", true);
        is!("a = [1, 2, 3]; a[1] == a#1", false);
        is!("a = [1, 2, 3]; a[1] == a#1", 0);
    );
    //	data_mode = false;
    testWasmMemoryIntegrity();
#[cfg(feature = "RUNTIME_ONLY")]{
    puts("RUNTIME_ONLY");
    puts("NO WASM emission...");
    //	return;
}

    //	assert_run not compatible with Wasmer, don't ask why, we don't know;);
    //    skip(
    //            testCustomOperators();
    //            testWasmMutableGlobal();
    //    );

    testMathOperators();
    testWasmLogicPrimitives();
    testWasmLogicUnary();
    testWasmLogicUnaryVariables();
    testWasmLogic();
    testWasmLogicNegated();
    testSquareExpWasm();
    testGlobals();

    testComparisonIdPrecedence();
    testWasmStuff();
    testFloatOperators();
    testConstReturn();
    testWasmIf();
    testMathPrimitives();
    testSelfModifying();
    testNorm();
    testComparisonPrimitives();
    testComparisonMath();
    testComparisonId();
    testWasmTernary();
    testSquareExpWasm();
    testRoundFloorCeiling();
    testWasmTernary();
    testWasmFunctionCalls();
    testWasmFunctionDefiniton();
    testWasmWhile();

    // the following need MERGE or RUNTIME! todo : split
    testWasmVariables0();
    testLogarithm();
    testMergeWabtByHand();
    testMergeWabt();
    testMathLibrary();
    testWasmLogicCombined();
    testMergeWabt();

    //	exit(21);
    testWasmIncrement();
    // TRUE TESTS:
    testRecentRandomBugs();
    // testOldRandomBugs();
    assert_is("١٢٣", 123); //  numerals are left-to-right (LTR) even in Arabic!

    skip(
        testMergeOwn();
        testMergeRelocate();
    );
    test_get_local();
    skip( // new stuff :
        testObjectPropertiesWasm();
        testWasmLogicOnObjects();
        testCustomOperators();
    );
}
