// Probe test to understand function definition AST structure
use warp::wasp_parser::parse;

#[test]
fn probe_function_def_ast() {
    // Test various function definition syntaxes

    println!("\n=== fib(n) = n + 1 ===");
    let node = parse("fib(n) = n + 1");
    println!("{:#?}", node);

    println!("\n=== fib := it + 1 ===");
    let node = parse("fib := it + 1");
    println!("{:#?}", node);

    println!("\n=== fib(n:int) = n + 1 ===");
    let node = parse("fib(n:int) = n + 1");
    println!("{:#?}", node);

    println!("\n=== square(x) = x * x ===");
    let node = parse("square(x) = x * x");
    println!("{:#?}", node);

    println!("\n=== fib(10) ===");
    let node = parse("fib(10)");
    println!("{:#?}", node);

    println!("\n=== fib(it - 2) ===");
    let node = parse("fib(it - 2)");
    println!("{:#?}", node);

    println!("\n=== fib(it - 1) + fib(it - 2) ===");
    let node = parse("fib(it - 1) + fib(it - 2)");
    println!("{:#?}", node);

    println!("\n=== it < 2 ? it : fib(it - 1) + fib(it - 2) ===");
    let node = parse("it < 2 ? it : fib(it - 1) + fib(it - 2)");
    println!("{:#?}", node);

    println!("\n=== fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\\nfib(10) ===");
    let node = parse("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)");
    println!("{:#?}", node);
}
