// Probe test to understand fibonacci parsing
use warp::wasp_parser::parse;

#[test]
fn probe_fib_parsing() {
    println!("\n=== fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\\nfib(10) ===");
    let node = parse("fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)");
    println!("{:#?}", node);
}

#[test]
fn probe_simple_fib_call() {
    println!("\n=== fib(it - 2) ===");
    let node = parse("fib(it - 2)");
    println!("{:#?}", node);
}

#[test]
fn probe_addition_with_fib() {
    println!("\n=== fib(it - 1) + fib(it - 2) ===");
    let node = parse("fib(it - 1) + fib(it - 2)");
    println!("{:#?}", node);
}

#[test]
fn probe_ternary_with_fib() {
    println!("\n=== it < 2 ? it : fib(it - 1) + fib(it - 2) ===");
    let node = parse("it < 2 ? it : fib(it - 1) + fib(it - 2)");
    println!("{:#?}", node);
}
