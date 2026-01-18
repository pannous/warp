use warp::wasp_parser::parse;

#[test]
fn probe_increment() {
    println!("\n=== i++ ===");
    let node = parse("i++");
    println!("serialized: {}", node.serialize());
    println!("debug: {:?}", node);

    println!("\n=== {{i++}} ===");
    let node = parse("{i++}");
    println!("serialized: {}", node.serialize());

    println!("\n=== while 1 {{2}} ===");
    let node = parse("while 1 {2}");
    println!("serialized: {}", node.serialize());

    println!("\n=== while (1) {{2}} ===");
    let node = parse("while (1) {2}");
    println!("serialized: {}", node.serialize());

    println!("\n=== while(1){{2}} no space ===");
    let node = parse("while(1){2}");
    println!("serialized: {}", node.serialize());

    println!("\n=== while(i){{2}} variable ===");
    let node = parse("while(i){2}");
    println!("serialized: {}", node.serialize());

    println!("\n=== i<9 no space ===");
    let node = parse("i<9");
    println!("serialized: {}", node.serialize());

    println!("\n=== while(i<9){{i++}} ===");
    let node = parse("while(i<9){i++}");
    println!("serialized: {}", node.serialize());
}

#[test]
fn probe_while_execution() {
    use warp::wasm_emitter::eval;
    println!("\n=== Testing i=1;while(i<9){{i++}};i+1 ===");
    let result = eval("i=1;while(i<9){i++};i+1");
    println!("Result: {:?}", result);
    assert_eq!(result, 10);
}
