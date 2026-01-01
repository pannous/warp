
// Function tests
// Migrated from tests_*.rs files

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
fn test2Def() {
    // parse("def test1(x){x+1};def test2(x){x+1};test2(3)");
    is!("def test1(x){x+1};def test2(x){x+1};test2(3)", 4);
    is!("def test1(x){x+3};def test2(x){x+1};test2(3)", 6);
}

#[test]
fn testFunctionDeclaration() {
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
fn testFunctionDeclarationParse() {
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
    skip!(

        assert!(*functions["test"].body == analyze(parse("return a*2"))); // why != ok but == not?
        eq!(*functions["test"].body, analyze(parse("return a*2")));
    );
}

#[test]
fn testRenameWasmFunction() {
    Module & module1 = loadModule("samples/test.wasm");
    module1.functions.at(0).name = "test";
    module1.save("samples/test2.wasm");
    // todo: assert! by loadModule("samples/test2.wasm");
}

#[test]
fn testWitFunction() {
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
fn testFloatReturnThroughMain() {
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
        printf("%llx\n", y);
    }
    y = 0x00FF000000000000; // -> 0.000000 OK
//     x = *(double *) & y;
    printf("%lf\n", x);
}

#[test]
fn testGraphParams() {
//     assert_parses("{\n  empireHero: hero(episode: EMPIRE){\n    name\n  }\n"
//                   "  jediHero: hero(episode: JEDI){\n    name\n  }\n}");
    Node & hero = result["empireHero"];
    hero.print();
    assert(hero["episode"] == "EMPIRE");
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
fn testParams() {
    //	eq!(parse("f(x)=x*x").param->first(),"x");
    //    data_mode = true; // todo ?
    body = assert_parses("body(style='blue'){a(link)}");
    assert(body["style"] == "blue");

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
assert(body2["style"] ==
            "green", 0)); // body has prescedence over param, semantically param provide extra data to body
        assert(body2[".style"] == "blue");
    );
    //	assert_parses("a(href='#'){'a link'}");
    //	assert_parses("(markdown link)[www]");
}

#[test]
fn testParamizedKeys() {
    //	<label for="pwd">Password</label>

    // 0. parameters accessible
    label0 = parse("label(for:password)");
    label0.print();
    Node & node = label0["for"];
    eq!(node, "password");
    eq!(label0["for"], "password");

    // 1. paramize keys: label{param=(for:password)}:"Text"
    label1 = parse("label(for:password):'Passwort'"); // declaration syntax :(
    // Node label1 = parse("label{for:password}:'Passwort'");
    // Node label1 = parse("label[for:password]:'Passwort'");
    label1.print();
    eq!(label1, "Passwort");
    eq!(label1["for"], "password");
    //	eq!(label1["for:password"],"Passwort");

    // 2. paramize values
    // TODO 1. move params of Passwort up to lable   OR 2. preserve Passwort as object in stead of making it string value of label!
    skip!(

        Node label2 = parse("label:'Passwort'(for:password)");
        assert!(label2 == "Passwort");
        eq!(label2, "Passwort");
        eq!(label2["for"], "password");
        eq!(label2["for"], "password"); // descend value??
        eq!(label2["Passwort"]["for"], "password");
    );

    skip!(

        //	3. relative equivalence? todo not really
        eq!(label1, label2);
        Node label3 = parse("label:{for:password 'Password'}");
    );
}

#[test]
fn testStackedLambdas() {
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

