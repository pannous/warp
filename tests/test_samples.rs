use warp::is;

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
