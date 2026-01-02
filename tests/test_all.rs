use wasp::node::Node;
use wasp::node::Node::Empty;
use wasp::wasp_parser::parse;
use wasp::{eq, is, skip};

#[test]
fn test_did_you_mean_alias() {
    skip!(

        Node ok1 = assert_parses("printf!('hi')");
        eq!(ok1[".warnings"], "DYM print"); // THIS CAN NEVER HAVED WORKED! BUG IN TEST PIPELINE!
    );
}

#[test]
fn test_empty() {
    let result = parse("{  }");
    eq!(result, Empty);
}

#[test]
fn test_eval() {
    is!("√4", 2);
}

#[test]
fn test_node_name() {
    let a = Node::Symbol("xor".to_string()); // NOT type string by default!
                                             //     bool
    let ok1 = a == "xor";
    eq!(a, "xor");
    eq!(a.name(), "xor");
    assert!(ok1);
}

#[test]
fn test_indent_as_block() {
    // 0x0E 	SO 	␎ 	^N 		Shift Out
    // 0x0F 	SI 	␏ 	^O 		Shift In
    //	indent/dedent  0xF03B looks like pause!?   0xF032…  it does, what's going on CLion? Using STSong!
    //	https://fontawesome.com/v4.7/icon/outdent looks more like it, also matching context  OK in font PingFang HK?
} // 􀖯􀉶𠿜🕻🗠🂿	𝄉

#[test]
fn test_group_cascade1() {
    let result0 = parse("a b; c d");
    eq!(result0.length(), 2);
    eq!(result0[1].length(), 2);
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
    eq!(result1, result);
}

#[test]
fn test_group_cascade() {
    //	test_group_cascade2();
    //	testGroupCascade0();
    //	test_group_cascade1();

    let result = parse(
        r#"{ a b c, d e f; g h i , j k l
              a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2}
              {a3 b3 c3, d3 e3 f3; g3 h3 i3 , j3 k3 l3
              a4 b4 c4 ,d4 e4 f4; g4 h4 i4 ,j4 k4 l4}"#,
    );
    result.print();
    // eq!(result.kind(), groups); // ( {} {} ) because 2 {}!
    let _first = result.first();
    // eq!(first.kind(), objects); // { a b c … }
    // eq!(first.first().kind(), groups); // or expression if x is op
    //     eq!(result.length(), 2) // {…} and {and}
    //     eq!(result[0].length(), 2) // a…  and a2…  with significant newline
    //     eq!(result[0][0].length(), 2) // a b c, d e f  and  g h i , j k l
    //     eq!(result[0][0][0].length(), 2) // a b c  and  d e f
    eq!(result[0][0], parse("a b c, d e f; g h i , j k l")); // significant newline!
    eq!(
        result[0][1],
        parse("a2 b2 c2, d2 e2 f2; g2 h2 i2 , j2 k2 l2")
    ); // significant newline!
       //     eq!(result[0][0][0][0].length(), 3) // a b c
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
    let reparse = parse(result.serialize().as_str());
    // print(reparse.serialize());
    eq!(result, reparse);
}
