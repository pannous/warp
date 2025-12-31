use wasp::wasp_parser::WaspParser;
use std::fs;

#[test]
fn test_full_html_wasp() {
    println!("\nTesting full html.wasp:\n");
    
    let content = fs::read_to_string("samples/html.wasp")
        .expect("Failed to read html.wasp");
    
    print!("  Parsing html.wasp... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    
    let node = WaspParser::parse(&content);
    println!("DONE");
    
    print!("  Printing result... ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    println!("{:?}", node);
    println!("DONE");
}
