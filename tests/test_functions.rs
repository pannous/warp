
// Function tests
// Migrated from tests_*.rs files

use wasp::{eq, is, skip};
use wasp::node::Node;
use wasp::wasp_parser::parse;

#[test]
fn test2def() {
    // parse("def test1(x){x+1};def test2(x){x+1};test2(3)");
    is!("def test1(x){x+1};def test2(x){x+1};test2(3)", 4);
    is!("def test1(x){x+3};def test2(x){x+1};test2(3)", 6);
}

#[test]
fn test_function_declaration() {
    // THESE NEVER WORKED! should they? YES! partly
    // 'fixing' one broke fib etc :(
    // 💡we already have a working syntax so this has low priority
    // ⚠️ DO we really have a working syntax??
    skip!(
 // TODO!
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
    );
}

#[test]
fn test_function_declaration_parse() {
    //    let node1 = analyze(parse("fn main(){}"));
    //    assert!(node1.kind==declaration);
    //    assert!(node1.name=="main");
    // let node2 = analyze(parse("fun test(float a):int{return a*2}")); // todo: cast return to int and parseDeclaration!
    let node2 = analyze(parse("fun test(float a){return a*2}"));
    // assert!(node2.kind == declaration);
    assert!(node2.name == "test");
    let functions = todo!();
    eq!(functions["test"].signature.size(), 1);
    eq!(functions["test"].signature.parameters[0].name, "a");
    eq!(functions["test"].signature.parameters[0].typo, Type::floats);
    // eq!(functions["test"].signature.parameters[0].typo, Type::reals); // upgrade float to real TODO not if explicit!
    assert!(functions["test"].body);
    // assert!(not(*functions["test"].body != analyze(parse("return a*2"))));
    skip!(
        assert!(*functions["test"].body == analyze(parse("return a*2"))); // why != ok but == not?
        eq!(*functions["test"].body, analyze(parse("return a*2")));
    );
}


#[test]
fn test_rename_wasm_function() {
    Module & module1 = loadModule("samples/test.wasm");
    module1.functions.at(0).name = "test";
    module1.save("samples/test2.wasm");
    // todo: assert! by loadModule("samples/test2.wasm");
}

#[test]
fn test_wit_function() {
    //    funcDeclaration
    // a:b,c vs a:b, c:d

    is!("add: func(a: float32, b: float32) -> float32", 0);
//     Module & mod = read_wasm("test.wasm");
    // print( mod .import_count);
    eq!(mod.import_count, 1);
    eq!(Node().setKind(longs).serialize(), "0");
    eq!(mod.import_names, List<String>{"add"}); // or export names?
}

#[test]
fn test_float_return_through_main() {
//     double
    x = 0.0000001; // 3e...
    //	double x=1000000000.1;// 4...
    //	double x=-1000000000.1;// c1…
    //	double x=9999999999999999.99999999;// 43…
    //	double x=-9999999999999999.99999999;// c3…
    //	double x=1.1;// 3ff199999999999a
    //	double x=-1.1;// bff199999999999a
//     int64
//     y = *(int64 *) & x;
    #[cfg(not(feature = "WASM"))]{
        printf!("%llx\n", y);
    }
    y = 0x00FF000000000000; // -> 0.000000 OK
//     x = *(double *) & y;
    printf!("%lf\n", x);
}

#[test]
fn test_graph_params() {
//     assert_parses("{\n  empireHero: hero(episode: EMPIRE){\n    name\n  }\n"
//                   "  jediHero: hero(episode: JEDI){\n    name\n  }\n}");
    Node & hero = result["empireHero"];
    hero.print();
    assert!(hero["episode"] == "EMPIRE");
//     assert_parses("\nfragment comparisonFields on Character{\n"
//                   "  name\n  appearsIn\n  friends{\n    name\n  }\n }");
    assert_parses("\nfragment comparisonFields on Character{\n  name\n  appearsIn\n  friends{\n    name\n  }\n}");
    // VARIAblE: { "episode": "JEDI" }
//     assert_parses("query HeroNameAndFriends($episode: Episode){\n"
//                   "  hero(episode: $episode){\n"
//                   "    name\n"
//                   "    friends{\n"
//                   "      name\n"
//                   "    }\n"
//                   "  }\n"
//                   "}");
}

#[test]
fn test_params() {
    //	eq!(parse("f(x)=x*x").param->first(),"x");
    //    data_mode = true; // todo ?
    body = assert_parses("body(style='blue'){a(link)}");
    assert!(body["style"] == "blue");

    parse("a(x:1)");
    assert_parses("a(x:1)");
    assert_parses("a(x=1)");
    assert_parses("a{y=1}");
    assert_parses("a(x=1){y=1}");
    skip!(
assert_parses("a(1){1}", 0));
    skip!(
assert_parses("multi_body{1}{1}{1}", 0)); // why not generalize from the start?
    skip!(
assert_parses("chained_ops(1)(1)(1)", 0)); // why not generalize from the start?

    assert_parses("while(x<3){y:z}");
    skip!(

        Node body2 = assert_parses(
            "body(style='blue'){style:green}"); // is that whole xml compatibility a good idea?
        skip!(
assert!(body2["style"] ==
            "green", 0)); // body has prescedence over param, semantically param provide extra data to body
        assert!(body2[".style"] == "blue");
    );
    //	assert_parses("a(href='#'){'a link'}");
    //	assert_parses("(markdown link)[www]");
}


#[test]
fn test_stacked_lambdas() {
    let result = parse("a{x:1}{y:2}{3}");
    result.print();
    assert!(result.length() == 3);
    assert!(result[0] == parse("{x:1}"));
    assert!(result[0] == parse("x:1")); // grouping irrelevant
    assert!(result[1] == parse("{y:2}"));
    assert!(result[2] == parse("{3}"));
    assert!(result[2] != parse("{4}"));

    assert!(parse("a{x}{y z}") != parse("a{x,{y z}}"));
}


#[test]
fn test_modifiers() {
    is!("public fun ignore(){3}", 3);
    is!("public static export import extern external C global inline virtual override final abstract private protected internal const constexpr volatile mutable thread_local synchronized transient native fun ignore(){3}",3);
}
