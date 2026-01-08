use wasp::is;
use wasp::wasm_gc_emitter::eval;
use wasp::node::Node;
use wasp::Number;

// DONE: Fix global variable shadowing - globals were being added to local scope

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.0001
}

fn assert_float_eq(code: &str, expected: f64) {
    let result = eval(code);
    match result {
        Node::Number(Number::Float(f)) => {
            assert!(approx_eq(f, expected), "{} = {} but expected {}", code, f, expected);
        }
        Node::Number(Number::Int(i)) => {
            assert!(approx_eq(i as f64, expected), "{} = {} but expected {}", code, i, expected);
        }
        other => panic!("{} returned {:?}, expected float {}", code, other, expected),
    }
}

#[test]
fn test_int_plus_pi_type_upgrading() {
    use std::f64::consts::PI;
    // Integer + Float = Float (type upgrading)
    is!("1+π", 1.0 + PI);
}

#[test]
fn test_float_plus_int_type_upgrading() {
    // Float + Integer = Float
    assert_float_eq("3.14+2", 5.14);
}

#[test]
fn test_float_operations() {
    is!("1.5+2.5", 4.0);
    is!("10.0-3.0", 7.0);
    is!("2.5*4.0", 10.0);
    is!("10.0/4.0", 2.5);
}

#[test]
fn test_global_with_float() {
    use std::f64::consts::PI;
    // global x = 1 + π should return the float value
    is!("global x=1+π", 1.0 + PI);
}

#[test]
fn test_global_with_reference() {
    // global x = 10; x + 5 should return 15
    is!("global x=10; x+5", 15);
}
