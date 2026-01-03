use wasp::wasp_parser::parse;

#[test]
fn test_simple_separators() {
    // Simple case: space then comma
    let r1 = parse("a b, c d");
    println!("\n=== a b, c d ===");
    println!("Full: {}", r1);
    println!("Length: {}", r1.length());
    if r1.length() > 0 {
        println!("r1[0]: {}", r1[0]);
        println!("r1[0] length: {}", r1[0].length());
    }
    if r1.length() > 1 {
        println!("r1[1]: {}", r1[1]);
    }
}

#[test]
fn test_expected_structure() {
    // What does the C++ version of "a b c" parse to?
    let r = parse("a b c");
    println!("\n=== a b c ===");
    println!("Full: {}", r);
    println!("Length: {}", r.length());
}
