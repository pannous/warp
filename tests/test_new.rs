use wasp::extensions::print;
use wasp::is;

#[test]
#[ignore]
fn test_function_definitions() {
	print("Testing function definitions...");
	is!("def add(a,b): a+b; add(2,3)", 5);
	is!("def square(x): x*x; square(4)", 16);
	print("✓ Function definition tests passed");
}

#[test]
#[ignore]
fn test_variables() {
	print("Testing variables...");
	is!("x=42; x", 42);
	is!("y=3; y", 3);
	print("✓ Variable tests passed");
}

#[test]
#[ignore]
fn test_fibonacci() {
	print("Testing fibonacci...");
	is!(
		"fib := it < 2 ? it : fib(it - 1) + fib(it - 2); fib(10)",
		55
	);
	print("✓ Fibonacci tests passed");
}
