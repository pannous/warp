use wasp::wasp_parser::WaspParser;

#[test]
fn test_mixed_keys() {
    let tests = vec![
        ("{a:1, b:2}", "symbol keys"),
        (r#"{"a":1, "b":2}"#, "quoted keys"),
        (r#"{a:1, "b":2}"#, "mixed: symbol then quoted"),
        (r#"{"a":1, b:2}"#, "mixed: quoted then symbol"),
    ];
    
    for (input, desc) in tests {
        print!("  Testing {}: ", desc);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let _node = WaspParser::parse(input);
        println!("OK");
    }
}
