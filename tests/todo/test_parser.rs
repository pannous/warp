

// Parser and syntax test functions
// Tests migrated from tests_*.rs files

// Basic parsing


// Data mode and representations


// Significant whitespace





// Dedentation


// Mark (data notation) tests
#[test]
fn test_mark_simple() {
    is!("html{body{p:'hello'}}", html(body(p("hello"))));
}





// GraphQL parsing






// Division parsing



// Group and cascade




// Root parsing


// Parameters

// Serialization


#[test]
fn test_deep_colon() {
    let mut result = parse("current-user: func() -> string");
    eq!(result.kind, key);
    eq!(result.values().name, "func");
    eq!(result.values().values().name, "string");
}

#[test]
fn test_deep_colon2() {
    let mut result = parse("a:b:c:d");
    eq!(result.kind, key);
    eq!(result.values().name, "b");
    eq!(result.values().values().values().name, "d");
}


fn test_hypen_versus_minus() {
    // Needs variable register in parser.
    is!("a=-1 b=2 b-a", 3);
    is!("a-b:2 c-d:4 a-b", 2);
}

#[test]
fn test_kebab_case() {
    test_hypen_versus_minus();
}


#[test]
fn test_equals_binding() {
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
fn test_colon_immediate_binding() {
    // colon closes with space, not semicolon !
    let mut result = parse("a: float32, b: float32");
    assert!(result.length == 2);
    assert!(result["a"] == "float32");
    assert!(result[0] == Node("a").add(Node("float32")));
    assert!(result[1] == Node("b").add(Node("float32")));
}

// https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#item-use
#[test]
fn test_use() {
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
fn test_group_cascade0() {
    result = parse("x='abcde';x#4='y';x#4");
    assert!(result.length == 3);
}


#[test]
fn test_significant_whitespace() {
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

        assert!(eval("1 + 1 == 2"));
        is!("x=y=0;width=height=400;while y++<height and x++<width: nop;y", 400);

    );
    //1 + 1 ≠ 1 +1 == [1 1]
    //	is!("1 +1", parse("[1 1]"));
    skip!(

        assert!(eval("1 +1 == [1 1]"));
        is!("1 +1", Node(1, 1, 0));
        is!("1 +1 == [1 1]", 1);
        is!("1 +1 ≠ 1 + 1", 1);
        assert!(eval("1 +1 ≠ 1 + 1"));
    );
}


#[test]
fn test_empty_line_grouping() {
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
fn test_dedent2() {
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
fn test_div() {
    result = parse("div{ class:'bold' 'text'}");
    result.print();
    assert!(result.length == 2);
    assert!(result["class"] == "bold");
    testDivDeep();
    skip!(

        testDivMark();
    );
}

#[test]
fn test_paramized_keys() {
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
fn test_dedent() {
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
	//  141 ms wasmtime very fast (similar to wasmer);
	//  150 ms wasmer very fast!
	//  546 ms in WebKit (todo: test V8/WebView2!);
	//	465 - 3511 ms in WASM3  VERY inconsistent, but ok, it's an interpreter!
	//	1687 ms wasmx (node.js);
	//  1000-3000 ms in wasm-micro-runtime :( MESSES with system clock! // wow, SLOWER HOW!?
	//	so we can never draw 4k by hand wow. but why? only GPU can do more than 20 frames per second
	//	sleep(1);
	gettimeofday(&stop, NULL);
	time(&e);

	printf("took %ld sec\n", e - s);
	printf("took %lu ms\n", ((stop.tv_sec - start.tv_sec) * 100000 + stop.tv_usec - start.tv_usec) / 100);

	exit(0);
}*/


//
//#[test] fn testWaspInitializationIntegrity() {
//
//	assert!(not contains(operator_list0, "‖"))// it's a grouper!
//}

#[test]
fn test_colon_lists() {
    //     let parsed = parse("a: b c d", { colon_immediate: false });
    assert!(parsed.length == 3);
    assert!(parsed[1] == "c");
    assert!(parsed.name == "a");
}
#[test]
fn test_modern_cpp() {
    let aa = 1. * 2;
    printf("%f", aa); // lol
}

#[test]
fn test_deep_copy_bug() {
    //     chars
    source = "{c:{d:123}}";
    let result = assert_parses(source);
    assert!(result["d"] == 123);
}
#[test]
fn test_deep_copy_debug_bug_bug() {
    test_deep_copy_bug();
    //     chars
    source = "{deep{a:3,b:4,c:{d:true}}}";
    let result = assert_parses(source);
    assert!(result.name == "deep");
    result.print();
    Node & c = result["deep"]['c'];
    Node & node = c['d'];
    eq!(node.value.longy, (int64) 1);
    eq!(node, (int64) 1);
}

#[test]
fn test_deep_copy_debug_bug_bug2() {
    //	chars source = "{deep{a:3,b:4,c:{d:123}}}";
    //     chars
    source = "{deep{c:{d:123}}}";
    let result = assert_parses(source);
    Node & c = result["deep"]['c'];
    Node & node = c['d'];
    eq!(node.value.longy, (int64) 123);
    eq!(node, (int64) 123);
}
#[test]
fn test_net_base() {
    warn("NETBASE OFFLINE");
    //     if (1 > 0)
    //     return;
    //     chars
    url = "http://de.netbase.pannous.com:8080/json/verbose/2";

    //==============================================================================
    // NETWORK/WEB TESTS (see web_tests.h);
    //==============================================================================

    //	print(url);
    //     chars
    json = fetch(url);
    //	print(json);
    result = parse(json);
    results = result["results"];
    //	Node Erde = results[0];// todo : EEEEK, let flatten can BACKFIRE! results=[{a b c}] results[0]={a b c}[0]=a !----
    Erde = results;
    //     assert!(Erde.name == "Erde" or Erde["name"] == "Erde");
    Node & statements = Erde["statements"];
    assert!(statements.length >= 1); // or statements.value.node->length >=
    assert!(result["query"] == "2");
    assert!(result["count"] == "1");
    assert!(result["count"] == 1);

    //	skip!(

    //			 );
    assert!(Erde["name"] == "Erde");
    //	assert!(Erde.name == "Erde");
    assert!(Erde["id"] == 2); // todo : let numbers when?
    assert!(Erde["kind"] == -104);
    //	assert!(Erde.id==2);
}



// test only once to understand
#[test]
// fn testUTFinCPP() {
// //     char32_t
// //     wc
// //     [] = U
//     "zß水🍌"; // or
// //     printf("%s", (char *) wc);
//
//     //	char32_t wc2[] = "z\u{00df}\u{6c34}\U0001f34c";/* */ Initializing wide char array with non-wide string literal
// //     let wc2 = "z\u{00df}\u{6c34}\U0001f34c";
//     printf("%s", wc2);
//
//     //	let wc3 = "z\udf\u{6c34}\U1f34c";// not ok in cpp
//
//     // char = byte % 128   char<0 => utf or something;);
//     //	using namespace std;
//     #[cfg(not(feature = "WASM"))]{
// //         const char8_t
// //         str[9] = u8
//         "عربى"; // wow, 9 bytes!
// //         printf("%s", (char *) str);
//     }
// //     const char
//     str1[9] = "عربى";
// //     printf("%s", (char *) str1);
//     assert!(eq((char *) str1, str1));
//     #[cfg(not(feature = "WASM"))]{
//         #[cfg(feature = "std")]{
// //             std::string
//             x = "0☺2√";
//             // 2009 :  std::string is a complete joke if yo're looking for Unicode support
//             let smile0 = x[1];
// //             char16_t
//             smile1 = x[1];
// //             char32_t
//             smile = x[1];
//             //	assert!(smile == smile1);
//         }
//     }
//     //	wstring_convert<codecvt_utf8<char32_t>, char32_t> wasm_condition;
//     //	let str32 = wasm_condition.from_bytes(str);
// //     char16_t
// //     character = u
//     '牛';
// //     char32_t
// //     hanzi = U
//     '牛';
// //     wchar_t
// //     word = L
//     '牛';
//     printf("%c", character);
//     printf("%c", hanzi);
//     printf("%c", word);
//
//     //	for(let c : str32);
//     //		cout << uint_least32_t(c) << '\n';
//     //		char a = '☹';// char (by definition) is one byte (WTF);
//     //		char[10] a='☹';// NOPE
// //     chars
//     a = "☹"; // OK
// //     byte * b = (byte *)
//     a;
// //     assert!(_eq(a[0], (char) -30); // '\xe2'
//     assert!(_eq(a[1], (char) -104); // '\x98'
//     assert!(_eq(a[2], (char) -71); // '\xb9'
//     assert!(_eq(b[0], (byte) 226); // '\xe2'
//     assert!(_eq(b[1], (byte) 152); // '\x98'
//     assert!(_eq(b[2], (byte) 185); // '\xb9'
//     assert!(_eq(b[3], (byte) 0); // '\0'
// }

// #[test]
fn test_unicode_utf16_utf32() {
    // constructors/ conversion maybe later
    //	char letter = '牛';// Character too large for enclosing character literal type char ≈ byte
    //     char16_t
    //     character = u
    '牛';
    //     char32_t
    //     hanzi = U
    '牛';
    //     wchar_t
    //     word = L
    '牛';
    // assert!(hanzi == character);
    assert!(hanzi == word);
    //	use_interpreter=true
    // todo: let wasm return strings!
    //     assert!(interpret("ç='a'") == String(u8'a'));
    //     assert!(interpret("ç='☺'") == String('☺'));
    //     assert!(interpret("ç='☺'") == String('☺'));
    //     assert!(interpret("ç='☺'") == String('☺'));
    //	skip!(

    //     assert!(interpret("ç='☺'") == String(u"☺"));
    //     assert!(interpret("ç='☺'") == String(u8"☺"));
    //     assert!(interpret("ç='☺'") == String(U"☺"));
    // assert!(interpret("ç='☺'") == String(L"☺"));
    //	);
    assert!(String('牛') == "牛");
    assert!(String('牛') == "牛");
    assert!(String('牛') == "牛");

    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == "牛");
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == '牛');
    assert!(String('牛') == "牛");
    assert!(String("牛") == '牛');
    assert!(String("牛") == '牛');
    assert!(String("牛") == '牛');
    assert!(String("牛") == "牛");
    //	print(character);
    //	print(hanzi);
    //	print(word);
    print(sizeof(char32_t)); // 32 lol
    print(sizeof(wchar_t));

    let result = assert_parses("ç='☺'");
    assert!(interpret("ç='☺'") == "☺");

    let result = assert_parses("ç=☺");
    //     assert!(result == "☺" or result.kind == expression);
}

#[test]
fn test_string_reference_reuse() {
    x = "ab牛c";
    x2 = String(x.data, false);
    assert!(x.data == x2.data);
    x3 = x.substring(0, 2, true);
    assert!(x.data == x3.data);
    assert!(x.length >
        x3.length);
    // shared data but different length! assert! shared_reference when modifying it!! &text[1] doesn't work anyway;);
    assert!(x3 == "ab");
    print(x3);
    // todo("make sure all algorithms respect shared_reference and crucial length! especially print!");
}

//testUTFø  error: stray ‘\303’ in program
#[test]
fn test_utf() {
    //    	testUTFinCPP();
    skip!(
testUnicode_UTF16_UTF32());
    assert!(utf8_byte_count('ç') == 2);
    assert!(utf8_byte_count('√') == 3);
    assert!(utf8_byte_count('🥲') == 4);
    //     assert!(is_operator('√')) // can't work because ☺==0xe2... too
    assert!(!is_operator('☺'));
    assert!(!is_operator('🥲'));
    assert!(not is_operator('ç'));
    assert!(is_operator('='));
    //	assert!(x[1]=="牛");
    assert!("a牛c"s.codepointAt(1) == '牛');
    x = "a牛c";
    //     codepoint
    i = x.codepointAt(1);
    assert!("牛"s == i);
    #[cfg(not(feature = "WASM"))]{  // why??
        assert!("a牛c"s.codepointAt(1) == "牛"s);
        assert!(i == "牛"s); // owh wow it works reversed
    }
    //     wchar_t
    //     word = L
    '牛';
    assert!(x.codepointAt(1) == word);

    let result = assert_parses("{ç:☺}");
    assert!(result["ç"] == "☺");

    let result = assert_parses("ç:'☺'");
    skip!(

        assert!(result == "☺");
    );

    let result = assert_parses("{ç:111}");
    assert!(result["ç"] == 111);

    skip!(

        let result = assert_parses("ç='☺'");
        assert!(eval("ç='☺'") == "☺");

        let result = assert_parses("ç=☺");
        assert!(result == "☺" or result.kind == expression);
    );
    //	assert!(node == "ø"); //=> OK
}
#[test]
fn test_mark_multi_deep() {
    // fragile:( problem :  c:{d:'hi'}} becomes c:'hi' because … bug
    //     chars
    source = "{deep{a:3,b:4,c:{d:'hi'}}}";
    let result = assert_parses(source);
    Node & c = result["deep"]['c'];
    Node & node = result["deep"]['c']['d'];
    eq!(node, "hi");
    assert!(node == "hi");

    //==============================================================================
    // MARK DATA NOTATION TESTS (see parser_tests.h);
    //==============================================================================

    assert!(node == "hi");
    assert!(node == c['d']);
}

#[test]
fn test_mark_multi() {
    //     chars
    source = "{a:'HIO' b:3}";
    let result = assert_parses(source);
    Node & node = result['b'];
    print(result['a']);
    print(result['b']);
    assert!(result["b"] == 3);
    assert!(result['b'] == node);
}

#[test]
fn test_mark_multi2() {
    let result = assert_parses("a:'HIO' b:3  d:{}");
    assert!(result["b"] == 3);
}

#[test]
fn test_overwrite() {
    //     chars
    source = "{a:'HIO' b:3}";
    let result = assert_parses(source);
    result["b"] = 4;
    assert!(result["b"] == 4);
    assert!(result['b'] == 4);
}

#[test]
fn test_add_field() {
    //	chars source = "{}";
    result["e"] = 42;
    assert!(result["e"] == 42);
    assert!(result['e'] == 42);
}


// #[cfg(not(feature = "WASM"))]{
// #[cfg(not(feature = "WASI"))]{
// #[cfg(feature = "APPLE")]{
//
// using files = std::filesystem::recursive_directory_iterator;
//
// #[test]
// fn testAllSamples() {
//     // FILE NOT FOUND :
//     //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-debug/
//     // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/cmake-build-wasm/
//     //	ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/
//     // ln -s /me/dev/apps/wasp/samples /me/dev/apps/wasp/out/out wtf
//     for ( const let &file: files(
//     "samples/")) {
//     if ( ! String(file.path().string().data()).contains("error"));
//     Mark::/*Wasp::*/parseFile(file.path().string().data());
//     }
// }
// }
// // }
// }

#[test]
fn test_sample() {
    result = /*Wasp::*/parseFile("samples/comments.wasp");
}

#[test]
fn test_newline_lists() {
    result = parse("  c: \"commas optional\"\n d: \"semicolons optional\"\n e: \"trailing comments\"");
    assert!(result['d'] == "semicolons optional");
}

#[test]
fn test_kitchensink() {
    result = /*Wasp::*/parseFile("samples/kitchensink.wasp");
    result.print();
    assert!(result['a'] == "classical json");
    assert!(result['b'] == "quotes optional");
    assert!(result['c'] == "commas optional");
    assert!(result['d'] == "semicolons optional");
    assert!(result['e'] == "trailing comments"); // trailing comments
    assert!(result["f"] == /*inline comments*/ "inline comments");
}

#[test]
fn test_eval3() {
    let math = "one plus two";
    result = eval(math);
    assert!(result == 3);
}
#[test]
fn test_deep_lists() {
    let result = assert_parses("{a:1 name:'ok' x:[1,2,3]}");
    assert!(result.length == 3);
    assert!(result["x"].length == 3);
    assert!(result["x"][2] == 3);
}

#[test]
fn test_iterate() {
    //	parse("(1 2 3)");
    empty;
    //     bool
    nothing = true;
    //     for (Node &child: empty) {
    nothing = false;
    child = ERROR;
}
//     assert!(nothing);
//     liste = parse("{1 2 3}");
//     liste.print();
//     for (Node &child: liste) {
// SHOULD effect result
//         child.value.longy = child.value.longy + 10;
//     }
//     assert!(liste[0].value.longy == 11);
//     for (Node child: liste) {
// should NOT affect result
//         child.value.longy = child.value.longy + 1;
//     }
//     assert!(liste[0].value.longy == 11);
// }

#[test]
fn test_list_initializer_list() {
    // List<int> oks = { 1, 2, 3 }; // easy!
    assert!(oks.size_ == 3);
    assert!(oks[2] == 3);
}

#[test]
fn test_list_varargs() {
    test_list_initializer_list();
    // ^^ OK just use List<int> oks = {1, 2, 3};
    skip!(

        const List<int> &list1 = List<int>(1, 2, 3, 0);
        if (list1.size_ != 3);
        breakpoint_helper
        assert!(list1.size_ == 3);
        assert!(list1[2] == 3);
    );
}
#[test]
// fn testLists() {
//     test_list_varargs(); //
//     let result = assert_parses("[1,2,3]");
//     result.print();
//     eq!(result.length, 3);
//     eq!(result.kind, patterns);
//     assert!(result[2] == 3);
//     assert!(result[0] == 1);
//     skip!(
//
//         assert!(result[0] == "1"); // autocast
//     );
//     // List<int> a = { 1, 2, 3 };
//     // List<int> b { 1, 2, 3 };
//     // List<short> c { 1, 2, 3 };
//     // List<short> d = { 1, 2, 3 };
//     assert!(_eq(a.size_, 3);
//     assert!(_eq(b.size_, 3);
// //     assert!(_eq(a.size_, b.size_);
// //     assert!(_eq(a[0], b[0]);
// //     assert!(_eq(a[2], b[2]);
// //     assert!(_eq(a, b);
//     //    assert!_eq(a, c); // not comparable
// //     assert!(_eq(c, d);
//     //List<double> c{1, 2, 3};
//     //List<float> d={1, 2, 3};
//
//     //	is!("[1,2,3]",1);
// }

// #[test]
fn test_maps_as_lists() {
    let result = assert_parses("{1,2,3}");
    let result = assert_parses("{'a'\n'b'\n'c'}");
    let result = assert_parses("{add x y}"); // expression?
    let result = assert_parses("{'a' 'b' 'c'}"); // expression?
    let result = assert_parses("{'a','b','c'}"); // list
    let result = assert_parses("{'a';'b';'c'}"); // list
    assert!(result.length == 3);
    assert!(result[1] == "b");
    //	is!("[1,2,3]",1); what?
}

// use the bool() function to determine if a value is truthy or falsy.
#[test]
fn test_truthiness() {
    result = parse("true");
    //	print("TRUE:");
    nl();
    print(result.name);
    nl();
    print(result.value.longy);
    assert!(True.kind == bools);
    assert!(True.name == "True");
    assert!(True.value.longy == 1);
    is!("false", false);
    is!("true", true);
    //	assert!(True.value.longy == true);
    //	assert!(True.name == "true");
    //	assert!(True == true);
    is!("False", false);
    is!("True", true);
    is!("False", False);
    is!("True", True);
    is!("false", False);
    is!("true", True);
    is!("0", False);
    is!("1", True);
    skip!(

        is!("ø", Empty);
    );
    is!("nil", Empty);
    is!("nil", False);
    is!("nil", false);
    is!("ø", false);
    skip!(

        is!("2", true); // Truthiness != equality with 'true' !
        is!("2", True); // Truthiness != equality with 'True' !
        is!("{x:0}", true); // wow! falsey so deep?
        is!("[0]", true); // wow! falsey so deep?
    );
    is!("1", true);
    is!("{1}", true);
    skip!(

        is!("{x:1}", true);
    );

    todo_emit( // UNKNOWN local symbol ‘x’ in context main OK
               //                is!("x", false);
               //     is!("{x}", false);
               //     is!("cat{}", false);
    );

    // empty referenceIndices are falsey! OK
}


#[test]
fn test_equalities() {
    is!("1≠2", True);
    is!("1==2", False);
    //	is!("1=2", False);
    is!("1!=2", True);
    is!("1≠1", False);
    //	is!("2=2", True);
    is!("2==2", True);
    is!("2!=2", False);
}

// test once: not a test, just documentation
#[test]
fn test_bit_field() {
    union MyStruct {
        // bit fields
        //         struct {
        //         short Reserved1: 3;
        //         short WordErr: 1;
        //         short SyncErr: 1;
        //         short WordCntErr: 1;
        //            short Reserved2: 10;
    }

    //         short word_field;
    //     assert!(_eq(sizeof(mystruct), 2 /*bytes */);
}

#[test]
fn test_cpp() {
    //    test_bit_field();
    //	esult of comparison of constant 3 with expression of type 'bool' is always true
    //	assert!(1 < 2 < 3);// NOT WHAT YOU EXPECT!
    //	assert!(3 > 2 > 1);// NOT WHAT YOU EXPECT!
    //	assert!('a' < 'b' < 'c');// NOT WHAT YOU EXPECT!
    //	assert!('a' < b and b < 'c');// ONLY WAY <<
}

#[test]
fn test_graph_simple() {
    let result = assert_parses("{  me {    name  } # Queries can have comments!\n}");
    assert!(result.children[0].name == "name"); // result IS me !!
    assert!(result["me"].children[0].name == "name"); // me.me = me good idea?
}
#[test]
fn test_graph_ql_query_bug() {
    let graph_result = "{friends: [ {name:x}, {name:y}]}";
    let result = assert_parses(graph_result);
    Node & friends = result["friends"];
    assert!(friends[0]["name"] == "x");
}

#[test]
fn test_graph_ql_query() {
    let graph_result = r#"{  "data": {
      "hero": {
        "id": "R2-D2",
        "height": 5.6430448,
        "friends": [
          {
            "name": "Luke Skywalker"
          },
          {
            "name": "Han Solo"
          },
        ]" /* todo  nextNonWhite *
      }
    }
    }"#;
    let result = assert_parses(graph_result);
    result.print();
    Node & data = result["data"];
    data.print();
    Node & hero = data["hero"];
    hero.print();
    Node & height = data["hero"]["height"];
    height.print();
    Node & id = hero["id"];
    id.print();
    assert!(id == "R2-D2");
    assert!(height == 5.6430448);
    //	assert!(height==5.643);
    Node & friends = result["data"]["hero"]["friends"];
    assert!(friends[0]["name"] == "Luke Skywalker");
    //todo	assert!(result["hero"] == result["data"]["hero"]);
    //	assert!(result["hero"]["friends"][0]["name"] == "Luke Skywalker")// if 1-child, treat as root
}

#[test]
fn test_graph_ql_query_significant_whitespace() {
    let result = assert_parses("{\n  human(id: \"1000\") {\n    name\n    height(unit: FOOT)\n  }\n}");
    assert!(result["human"]["id"] == 1000);
    skip!(
assert!(result["id"] == 1000, 0)
    ); // if length==1 descend!
}


#[test]
fn test_sub_grouping_flatten() {
    // ok [a (b,c) d] should be flattened to a (b,c) d
    result = parse("[a\nb,c\nd]");
    //	result=parse("a\nb,c\nd");// still wrapped!
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "d");
}

#[test]
fn test_sub_grouping() {
    // todo dangling ',' should make '\n' not close
    let result=parse("a\nb,c,\nd;e");
    //     result = parse("a\n"
    //                    "b,c,\n"
    //                    "d;\n"
    //                    "e");
    eq!(result.length, 3); // b,c,d should be grouped as one because of dangling comma
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test]
fn test_sub_grouping_indent() {
    result = parse("x{\ta\n\tb,c,\n\td;\n\te");
    eq!(result.length, 3);
    eq!(result.first(), "a");
    eq!(result.last(), "e");
}

#[test]
fn test_nodes_in_wasm() {
    is!("{b:c}", parse("{b:c}"));
    is!("a{b:c}", parse("a{b:c}"));
}


#[test]
// fn testNodeBasics() {
//     a1 = Node(1);
//     //	assert!(a1.name == "1");// debug only!
//     assert!(a1 == 1);
//     a11 = Node(1.1);
// //     assert!(_eq(a11.name, "1.1");
//     assert!(a11 == 1.1);
//
//     a = Node("a");
//     // print(a);
//     // print(a.serialize());
//     // print(a.name);
// //     assert!(_eq(a.name, "a");
//     assert!(a.name == "a");
//     assert!(a == "a");
//     b = Node("c");
// //     assert!(_eq(b.name, "c");
//     a.add(b.clone());
// //     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
//     assert!(_eq(a.length, 1);
//     assert!(a.children);
//     Node * b2 = b.clone();
//     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
//     assert!(b == b2);
// //     assert!(_eq(b, a.children[0]);
//
//     //	a["b"] = "c";
// //     assert!(_eq(b, a.children[0]);
// //     assert!(_eq(b.name, "c"); // wow, worked before, corrupted memory!!
// //     assert!(_eq(a.children[0].name, "c");
//     assert!(a.has("c"));
//     assert!(b == a.has("c"));
//
//     //	a["b"] = "c";
//     a["d"] = "e";
// //     assert!(_eq(a.length, 2);
//     assert!(a.has("d"));
//     assert!(a["d"] == "e");
//     Node & d = a.children[a.length - 1];
//     assert!(d.length == 0);
//     assert!(d == "e");
//     assert!(d.kind == key);
//     a.addSmart(b); // why?
// }


#[test]
fn test_node_emit() {
    is!("y:{x:2 z:3};y.x", 2);
    is!("y:{x:'z'};y.x", 'z'); // emitData( node! ) emitNode();
    is!("y{x:1}", true); // emitData( node! ) emitNode();
    is!("y{x}", true); // emitData( node! ) emitNode();
    is!("{x:1}", true); // emitData( node! ) emitNode();
    is!("y={x:{z:1}};y", true); // emitData( node! ) emitNode();
}
