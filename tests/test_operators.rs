// Operator tests
// Migrated from tests_*.rs files

use wasp::extensions::{assert_throws, todow};
use wasp::node::Node;
use wasp::wasp_parser::parse;
use wasp::{is, skip};

const PI: f64 = std::f64::consts::PI;
const E: f64 = std::f64::consts::E;

#[test]
fn test_not_truthy_falsy() {
    is!("not ''", 1);
    is!("not \"\"", 1);
}

#[test]
fn test_not_negation2() {
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
fn test_not_negation() {
    // test_not_negation2(); // just aliases for 'not'
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
fn test_while_not() {
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
fn test_while_not_call() {
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
fn test_random_parse() {
    let node = parse("x:40;x+1");
    assert!(node.length() == 2);
    assert!(node[0]["x"] == 40); // breaks!?
    // assert!(operator_list.has("+"));
    // assert!(not(bool) Node("x"));
    // assert! _silent(false == (bool) Node("x"));
    // assert!(Node("x") == false);
}

#[test]
fn test_minus_minus() {
    #[cfg(not(feature = "WASM"))]{ // todo square
        is!("1 - 3 - square 3+4",  -51); // OK!
    }

    //    is!("1 -3 - square 3+4",  -51);// warn "mixing math op with list items (1, -3 … ) !"
    //    is!("1--3", 4);// todo parse error
    is!("1- -3", 4); // -1 uh ok?  warn "what are you doning?"
    is!("1 - -3", 4); // -1 uh ok?  warn "what are you doning?"
    //    is!("1 - - 3", 4);// error ok todo parse error
}

#[test]
fn test_exp() {
    // todo parsed same:
    is!("ℯ^0", 1);
    is!("ℯ^1", E);
    is!("π^0", 1);
    is!("π^1", PI);
    is!("π*√163", 40.1091); // ok
    skip!(

        is!("π√163", 40.1091);
        is!("(π*√163)==(π√163)", 1);
        is!("π*√163==(π√163)", 1);
        is!("π*√163==π√163", 1);
        is!("exp(0)", 1); // "TODO rewrite as ℯ^x" OK
    );
    is!("ℯ^(π*√163)", 262537412640768743.99999999999925);
}

#[test]
fn test_matrix_order() {
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
fn test_vector_shim() {
    //    unknown function matrix_multiply (matrix_multiply);
    is!("v=[1 2 3];w=[2 3 4];v*w", 2 + 6 + 12);
}



#[test]
fn test_hypen_versus_minus() {
    // Needs variable register in parser.
//     const char
    let code = "a=-1 b=2 b-a";
    is!(code, 3);
    // kebab case
//     const char
    let data = "a-b:2 c-d:4 a-b";
    is!(data, 2);
    //    testHyphenUnits();

    //    let node : Node = parse(data);
}


#[test]
fn test_import42() {
    is!("import fourty_two", 42);
    is!("include fourty_two", 42);
    is!("require fourty_two", 42);
    is!("import fourty_two;ft*2", 42 * 2);
    is!("include fourty_two;ft*2", 42 * 2);
    is!("require fourty_two;ft*2", 42 * 2);
}

#[test]
fn test_div_deep() {
    let div = parse("div{ span{ class:'bold' 'text'} br}");
    let node : &Node = &div["span"];
    node.print();
    assert!(div["span"].length() == 2);
    assert!(div["span"]["class"] == "bold");
}

#[test]
fn test_div_mark() {
    // use_polish_notation = true;
    let div = parse("{div {span class:'bold' 'text'} {br}}");
    let span : &Node = &div["span"];
    span.print();
    assert!(span.length() == 2);
    assert!(span["class"] == "bold");
    // use_polish_notation = false;
}


#[test]
fn test_errors() {
    // use assert_throws
    // throwing = true;
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
    // assert!(result == ERROR);
    /*
        ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/out
        ln -s /Users/me/dev/apps/wasp/samples /Users/me/dev/apps/wasp/cmake-build-default/ #out/
      */
    // breakpoint_helper todo
    // result = /*Wasp::*/parseFile("samples/errors.wasp");
    // throwing = true;
}


#[test]
fn test_for_each() {
}

#[test]
fn test_logic() {
    is!("true or false", true);
    is!("false or true", true);

    is!("not true", false);
    is!("not false", true); // fourth test fails regardles of complexity?

    is!("false or false", false);
    is!("true or false", true);
    is!("true or true", true);
    //==============================================================================
    // LOGIC/BOOLEAN TESTS (see angle_tests.h + feature_tests.h);
    //==============================================================================

    is!("true and true", true);
    is!("true and false", false);
    is!("false and true", false);
    is!("false and false", false);

    is!("false xor true", true);
    is!("true xor false", true);
    is!("false xor false", false);
    is!("true xor true", false);

    is!("¬ 1", 0);
    is!("¬ 0", 1);

    is!("0 ⋁ 0", 0);
    is!("0 ⋁ 1", 1);
    is!("1 ⋁ 0", 1);
    is!("1 ⋁ 1", 1);

    is!("0 ⊻ 0", 0);
    is!("0 ⊻ 1", 1);
    is!("1 ⊻ 0", 1);
    is!("1 ⊻ 1", 0);

    is!("1 ∧ 1", 1);
    is!("1 ∧ 0", 0);
    is!("0 ∧ 1", 0);
    is!("0 ∧ 0", 0);
}

#[test]
fn test_logic_empty_set() {
    is!("not ()", true); // missing args for operator not
    is!("() xor 1", true);
    is!("1 xor ()", true);
    is!("() xor ()", false);
    is!("1 xor 1", false);
    is!("() or 1", true);
    is!("() or ()", false);
    is!("1 or ()", true);
    is!("1 or 1", true);

    is!("1 and 1", true);
    is!("1 and ()", false);
    is!("() and 1", false);
    is!("() and ()", false);

    is!("not 1", false);
    is!("{} xor 1", true);
    is!("1 xor {}", true);
    is!("{} xor {}", false);
    is!("1 xor 1", false);
    is!("{} or 1", true);
    is!("{} or {}", false);
    is!("1 or {}", true);
    is!("1 or 1", true);

    is!("1 and 1", true);
    is!("1 and {}", false);
    is!("{} and 1", false);
    is!("{} and {}", false);

    is!("not {}", true);
    is!("not 1", false);

    is!("[] or 1", true);
    is!("[] or []", false);
    is!("1 or []", true);
    is!("1 or 1", true);

    is!("1 and 0", false);
    is!("1 and []", false);
    is!("[] and 1", false);
    is!("[] and []", false);

    is!("not []", true);
    is!("not 1", false);
    is!("[] xor 1", true);
    is!("1 xor []", true);
    is!("[] xor []", false);
    is!("1 xor 1", false);
}

#[test]
fn test_logic_operators() {
    is!("¬ 0", 1);
    is!("¬ 1", 0);

    is!("0 ⋁ 0", 0);
    is!("0 ⋁ 1", 1);
    is!("1 ⋁ 0", 1);
    is!("1 ⋁ 1", 1);

    is!("0 ⊻ 0", 0);
    is!("0 ⊻ 1", 1);
    is!("1 ⊻ 0", 1);
    is!("1 ⊻ 1", 0);

    is!("1 ∧ 1", 1);
    is!("1 ∧ 0", 0);
    is!("0 ∧ 1", 0);
    is!("0 ∧ 0", 0);
}

#[test]
fn test_logic01() {
    is!("0 or 0", false);
    is!("0 or 1", true);
    is!("1 or 0", true);
    is!("1 or 1", true);

    is!("1 and 1", true);
    is!("1 and 0", false);
    is!("0 and 1", false);
    is!("0 and 0", false);

    // eor either or
    is!("0 xor 0", false);
    is!("0 xor 1", true);
    is!("1 xor 0", true);
    is!("1 xor 1", false);

    is!("not 0", true);
    is!("not 1", false);
}

#[test]
fn test_length_operator() {
    is!("#'0123'", 4); // todo at compile?
    is!("#[0 1 2 3]", 4);
    is!("#[a b c d]", 4);
    is!("len('0123')", 4); // todo at compile?
    is!("len([0 1 2 3])", 4);
    is!("size([a b c d])", 4);
    is!("#{a b c}", 3);
    is!("#(a b c)", 3); // todo: groups
}





