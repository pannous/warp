use wasp::wasp_parser::WaspParser;
use wasp::node::Node;

fn main() {
    println!("=== WASP Comment Support Demo ===\n");

    // Example 1: Line comments
    let wasp1 = r#"
        // User configuration
        name: "Alice"
        // Age in years
        age: 30
    "#;

    println!("Example 1: Line comments");
    println!("WASP:\n{}", wasp1);
    let node1 = WaspParser::parse(wasp1).unwrap();
    println!("Parsed: {:?}\n", node1);

    // Example 2: Block comments
    let wasp2 = r#"
        /* Configuration for the application server
           Port and host settings */
        server {
            port: 8080
            host: "localhost"
        }
    "#;

    println!("Example 2: Block comments");
    println!("WASP:\n{}", wasp2);
    let node2 = WaspParser::parse(wasp2).unwrap();
    println!("Parsed: {:?}\n", node2);

    // Example 3: Metadata accessors
    println!("Example 3: Using metadata");
    let node = Node::int(42).with_comment("The answer to everything".to_string());
    println!("Node with comment: {:?}", node);

    if let Some(meta) = node.get_meta() {
        println!("Comment: {:?}", meta.comment);
    }
    println!("Unwrapped value: {:?}\n", node.unwrap_meta());

    // Example 4: HTML-like structure with comments
    let wasp4 = r#"
        html{
            // Header section
            header{ title:"My Site" }

            // Main content area
            body{
                // Welcome message
                h1:"Welcome"
                /* Paragraph with description
                   Can span multiple lines */
                p:"Hello World"
            }
        }
    "#;

    println!("Example 4: HTML structure with comments");
    println!("WASP:\n{}", wasp4);
    let node4 = WaspParser::parse(wasp4).unwrap();
    println!("Parsed: {:?}", node4);

    // Convert to JSON
    let json = node4.to_json().unwrap();
    println!("\nJSON output:\n{}\n", json);

    println!("=== Demo Complete ===");
}
