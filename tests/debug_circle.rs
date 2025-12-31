use wasp::wasp_parser::WaspParser;
use std::fs;
use std::path::Path;
use wasp::node::Node;

#[test]
fn test_all_samples_unmodified() {
    println!("\nTesting ALL samples using fs::read_dir:\n");
    
    let samples_dir = Path::new("samples");
    let entries = fs::read_dir(samples_dir).expect("Failed to read samples directory");
    
    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) != Some("wasp") {
            continue;
        }
        
        let filename = path.file_name().unwrap().to_str().unwrap();
        print!("  Parsing {}... ", filename);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        
        match fs::read_to_string(&path) {
            Ok(content) => {
                let node = WaspParser::parse(&content);
                if let Node::Error(e) = &node {
                    println!("✗ Parse error: {}", e);
                } else {
                    println!("✓");
                    if filename == "circle.wasp" {
                        println!("    [CIRCLE.WASP] → About to print debug...");
                        std::io::Write::flush(&mut std::io::stdout()).unwrap();
                    }
                    println!("    → {:?}", node);
                }
            }
            Err(e) => {
                println!("✗ Read error: {}", e);
            }
        }
    }
}
