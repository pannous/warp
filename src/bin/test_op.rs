use wasp::wasp_parser::parse;
use wasp::node::Node;

fn check_node(node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        Node::Key(left, op, right) => {
            println!("{}Key: op={}", indent, op);
            println!("{}  left:", indent);
            check_node(left, depth + 2);
            println!("{}  right:", indent);
            check_node(right, depth + 2);
        }
        Node::List(items, _bracket, _sep) => {
            println!("{}List with {} items", indent, items.len());
            for (i, item) in items.iter().enumerate() {
                println!("{}  [{}]:", indent, i);
                check_node(item, depth + 2);
            }
        }
        Node::Number(n) => {
            println!("{}Number: {:?}", indent, n);
        }
        Node::Symbol(s) => {
            println!("{}Symbol: {}", indent, s);
        }
        Node::Meta { node, data: _ } => {
            println!("{}Meta wrapping:", indent);
            check_node(node, depth + 1);
        }
        Node::Empty => {
            println!("{}Empty", indent);
        }
        _ => {
            println!("{}Other", indent);
        }
    }
}

fn main() {
    println!("=== Testing: 5 < 10 ===");
    let result = parse("5 < 10");
    check_node(&result, 0);

    println!("\n=== Testing: a ===");
    let result = parse("a");
    check_node(&result, 0);

    println!("\n=== Testing: a + b ===");
    let result = parse("a + b");
    check_node(&result, 0);

    // This one seems to hang - using symbols with <
    println!("\n=== Testing: x < y ===");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
    let result = parse("x < y");
    check_node(&result, 0);
}
