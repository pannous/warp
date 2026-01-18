use warp::wasp_parser::parse;

#[test]
fn probe_def_syntax() {
    println!("\n=== def test1(x){{x+1}} ===");
    let node = parse("def test1(x){x+1}");
    println!("kind: {:?}", node.kind());
    println!("serialized: {}", node.serialize());
    println!("debug: {:?}", node);
}

#[test]
fn probe_multiple_syntaxes() {
    // Current working syntaxes
    println!("\n=== fib(n) = body ===");
    let node = parse("fib(n) = n + 1");
    println!("serialized: {}", node.serialize());
    
    println!("\n=== fib := body ===");
    let node = parse("fib := it + 1");
    println!("serialized: {}", node.serialize());
    
    // def syntax variants
    println!("\n=== def fib(n){{body}} ===");
    let node = parse("def fib(n){n+1}");
    println!("serialized: {}", node.serialize());

    println!("\n=== def fib(n): body ===");
    let node = parse("def fib(n): n+1");
    println!("serialized: {}", node.serialize());

    println!("\n=== def add(a,b): a+b ===");
    let node = parse("def add(a,b): a+b");
    println!("serialized: {}", node.serialize());
}

#[test]
fn probe_def_add() {
    use warp::wasm_emitter::eval;
    println!("\n=== Testing def add(a,b): a+b; add(2,3) ===");
    let result = eval("def add(a,b): a+b; add(2,3)");
    println!("Result: {:?}", result);
}

#[test]
fn probe_parse_add_call() {
    println!("\n=== add(2,3) parsing ===");
    let node = parse("add(2,3)");
    println!("serialized: {}", node.serialize());
    println!("debug: {:?}", node);

    println!("\n=== def add(a,b): a+b; add(2,3) full ===");
    let node = parse("def add(a,b): a+b; add(2,3)");
    println!("serialized: {}", node.serialize());
    if let warp::Node::List(items, _, _) = node.drop_meta() {
        println!("Top-level items: {}", items.len());
        for (i, item) in items.iter().enumerate() {
            println!("  [{}] kind={:?} = {}", i, item.kind(), item.serialize());
            if let warp::Node::List(inner, _, _) = item.drop_meta() {
                if !inner.is_empty() {
                    println!("      inner[0] kind={:?} = {}", inner[0].kind(), inner[0].serialize());
                }
            }
        }
    }
}

#[test]
fn probe_def_block_syntax() {
    use warp::wasm_emitter::eval;
    println!("\n=== Testing def test1(x){{x+1}}; test1(3) ===");
    let result = eval("def test1(x){x+1}; test1(3)");
    println!("Result: {:?}", result);
    assert_eq!(result, 4);
}

#[test]
fn probe_test2_check() {
    use warp::wasm_emitter::eval;
    println!("\n=== Two function definitions ===");
    let result = eval("def test1(x){x+1};def test2(x){x+1};test2(3)");
    println!("Result: {:?}", result);
    assert_eq!(result, 4);
}

#[test]
fn probe_dollar_param() {
    use warp::wasm_emitter::eval;
    // $0 references first parameter in function definitions
    let result = eval("add1(x):=$0+1; add1(3)");
    assert_eq!(result, 4);
}
