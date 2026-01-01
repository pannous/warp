

// Parser and syntax test functions
// Tests migrated from tests_*.rs files

// Basic parsing


// Data mode and representations


// Significant whitespace





// Dedentation


// Mark (data notation) tests
#[test]
fn testMarkSimple() {
    is!("html{body{p:'hello'}}", html(body(p("hello"))));
}





// GraphQL parsing






// Division parsing



// Group and cascade




// Root parsing


// Parameters

// Serialization


#[test]
fn testDeepColon() {
    let mut result = parse("current-user: func() -> string");
    eq!(result.kind, key);
    eq!(result.values().name, "func");
    eq!(result.values().values().name, "string");
}

#[test]
fn testDeepColon2() {
    let mut result = parse("a:b:c:d");
    eq!(result.kind, key);
    eq!(result.values().name, "b");
    eq!(result.values().values().values().name, "d");
}


fn testHypenVersusMinus() {
    // Needs variable register in parser.
    is!("a=-1 b=2 b-a", 3);
    is!("a-b:2 c-d:4 a-b", 2);
}

#[test]
fn testKebabCase() {
    testHypenVersusMinus();
}


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
fn testColonImmediateBinding() {
    // colon closes with space, not semicolon !
    let mut result = parse("a: float32, b: float32");
    assert!(result.length == 2);
    assert!(result["a"] == "float32");
    assert!(result[0] == Node("a").add(Node("float32")));
    assert!(result[1] == Node("b").add(Node("float32")));
}

// https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-use
#[test]
fn testUse() {
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

#[test]
fn testGroupCascade0() {
    result = parse("x='abcde';x#4='y';x#4");
    assert!(result.length == 3);
}


#[test]
fn testSignificantWhitespace() {
    skip!(
testDataMode());
    result = parse("a b (c)");
    assert!(result.length == 3);
    result = parse("a b(c)");
    assert!(result.length == 2 or result.length == 1);
    result = parse("a b:c");
    assert!(result.length == 2); // a , b:c
    assert!(result.last().kind == key); // a , b:c
    //     result = parse("a: b c d", { colon_immediate: false });
    assert!(result.length == 3);
    assert!(result.name == "a"); // "a"(b c d), NOT ((a:b) c d);
    assert!(result.kind == groups); // not key!
    //     result = parse("a b : c", { colon_immediate: false });
    assert!(result.length == 1 or result.length == 2); // (a b):c
    eq!(result.kind, key);
    skip!(

        assert(eval("1 + 1 == 2"));
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);

    );
    //1 + 1 ≠ 1 +1 == [1 1]
    //	is!("1 +1", parse("[1 1]"));
    skip!(

        assert(eval("1 +1 == [1 1]"));
        is!("1 +1", Node(1, 1, 0));
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        assert(eval("1 +1 ≠ 1 + 1"));
    );
}


#[test]
fn testEmptyLineGrouping() {
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

#[test]
fn testDedent2() {
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
