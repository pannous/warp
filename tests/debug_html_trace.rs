use std::fs;

#[test]
fn trace_html_parsing() {
    // Read html.wasp
    let content = fs::read_to_string("samples/html.wasp")
        .expect("Failed to read html.wasp");
    
    println!("\n=== Content to parse ===");
    println!("{}", content);
    println!("\n=== Starting parse ===\n");
    
    // We'll manually call the parser with a simple wrapper that tracks iterations
    use wasp::wasp_parser::WaspParser;
    
    std::panic::set_hook(Box::new(|_info| {
        println!("\nPANIC - likely infinite loop detected by timeout");
    }));
    
    let result = WaspParser::parse(&content);
    println!("\n=== Parse completed ===");
    println!("{:?}", result);
}
