use warp::is;

// Working samples - all passing
#[test]
fn test_fibonacci() { is!("samples/fibonacci.wasp", 55); }

#[test]
fn test_factorial() { is!("samples/factorial.wasp", 120); }

#[test]
fn test_primes() { is!("samples/primes.wasp", 1); }

#[test]
fn test_gcd() { is!("samples/gcd.wasp", 6); }

#[test]
fn test_sum() { is!("samples/sum.wasp", 55); }

#[test]
fn test_power() { is!("samples/power.wasp", 1024); }

#[test]
fn test_collatz() { is!("samples/collatz.wasp", 111); }

#[test]
fn test_ackermann() { is!("samples/ackermann.wasp", 61); }

#[test]
fn test_quadratic() { is!("samples/quadratic.wasp", 6); }

#[test]
fn test_fizzbuzz() { is!("samples/fizzbuzz.wasp", "FizzBuzz"); }

// Additional simple samples to test
#[test]
fn test_simple() { is!("samples/simple.wasp", 9); }

#[test]
#[ignore = "uses #use lib directive which needs module system"]
fn test_main() { is!("samples/main.wasp", 42); }

#[test]
#[ignore = "complex function with fractions, τ, and π constants"]
fn test_sine() { is!("samples/sine.wasp", 1); }

#[test]
#[ignore = "string concatenation needs implementation"]
fn test_hello() { is!("samples/hello.wasp", "Hello 🌍2026"); }
