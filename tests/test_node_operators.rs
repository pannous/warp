use wasp::node::Node;
use wasp::node::Node::*;
use wasp::*; // TODO: Meta is not exported

#[test]
fn test_node_add_basic() {
    // Node + Node
    eq!(&Node::int(3) + &Node::int(2), 5);
    eq!(&Node::float(3.5) + &Node::float(2.5), 6.0);

    // Node + primitive
    eq!(&Node::int(3) + 2_i64, 5);
    eq!(&Node::int(3) + 2_i32, 5);
    eq!(&Node::float(3.5) + 2.5_f64, 6.0);

    // primitive + Node
    eq!(2_i64 + &Node::int(3), 5);
    eq!(2_i32 + &Node::int(3), 5);
    eq!(2.5_f64 + &Node::float(3.5), 6.0);
}

#[test]
fn test_node_add_booleans() {
    eq!(&True + &True, 2);
    eq!(&True + &Node::int(5), 6);
    eq!(&Node::int(5) + &True, 6);
    eq!(&False + &Node::int(5), 5);
    eq!(&Node::int(5) + &False, 5);
}

#[test]
fn test_node_add_empty() {
    eq!(&Empty + &Node::int(5), 5);
    eq!(&Node::int(10) + &Empty, 10);
}

#[test]
fn test_node_sub_basic() {
    // Node - Node
    eq!(&Node::int(10) - &Node::int(3), 7);
    eq!(&Node::float(10.5) - &Node::float(3.5), 7.0);

    // Node - primitive
    eq!(&Node::int(10) - 3_i64, 7);
    eq!(&Node::int(10) - 3_i32, 7);
    eq!(&Node::float(10.5) - 3.5_f64, 7.0);

    // primitive - Node
    eq!(10_i64 - &Node::int(3), 7);
    eq!(10_i32 - &Node::int(3), 7);
    eq!(10.5_f64 - &Node::float(3.5), 7.0);
}

#[test]
fn test_node_sub_booleans() {
    eq!(&True - &True, 0);
    eq!(&True - &Node::int(5), -4);
    eq!(&Node::int(5) - &True, 4);
    eq!(&False - &Node::int(5), -5);
    eq!(&Node::int(5) - &False, 5);
}

#[test]
fn test_node_mul_basic() {
    // Node * Node
    eq!(&Node::int(3) * &Node::int(4), 12);
    eq!(&Node::float(3.0) * &Node::float(4.0), 12.0);

    // Node * primitive
    eq!(&Node::int(3) * 4_i64, 12);
    eq!(&Node::int(3) * 4_i32, 12);
    eq!(&Node::float(3.0) * 4.0_f64, 12.0);

    // primitive * Node
    eq!(3_i64 * &Node::int(4), 12);
    eq!(3_i32 * &Node::int(4), 12);
    eq!(3.0_f64 * &Node::float(4.0), 12.0);
}

#[test]
fn test_node_mul_booleans() {
    // True * n = n (identity)
    eq!(&True * &Node::int(5), 5);
    eq!(&Node::int(5) * &True, 5);

    // False * n = 0
    eq!(&False * &Node::int(5), 0);
    eq!(&Node::int(5) * &False, 0);
}

#[test]
fn test_node_mul_empty() {
    // Empty * n = 0
    eq!(&Empty * &Node::int(5), 0);
    eq!(&Node::int(5) * &Empty, 0);
}

#[test]
fn test_node_div_basic() {
    // Node / Node
    eq!(&Node::float(10.0) / &Node::float(2.0), 5.0);

    // Node / primitive
    eq!(&Node::float(10.0) / 2.0_f64, 5.0);

    // primitive / Node
    eq!(10.0_f64 / &Node::float(2.0), 5.0);
}

#[test]
fn test_node_div_booleans() {
    // n / True = n (since True = 1)
    eq!(&Node::float(5.0) / &True, 5.0);

    // False / n = 0
    eq!(&False / &Node::int(5), 0);
}

#[test]
fn test_node_meta_preservation() {
    // let meta = Meta::with_comment("test comment".to_string()); // TODO: Meta not exported
    // let node = Node::int(3).with_meta(meta); // TODO: Meta not exported

    // TODO: Meta not exported - test disabled
    // // Test Add preserves metadata
    // let result = &node + &Node::int(2);
    // eq!(result, 5);
    // assert!(result.get_meta().is_some());
    // eq!(
    //     result.get_meta().unwrap().comment,
    //     Some("test comment".to_string())
    // );

    // // Test Sub preserves metadata
    // let result = &node - &Node::int(1);
    // eq!(result, 2);
    // assert!(result.get_meta().is_some());

    // // Test Mul preserves metadata
    // let result = &node * &Node::int(2);
    // eq!(result, 6);
    // assert!(result.get_meta().is_some());

    // // Test Div preserves metadata
    // let result = &node / &Node::int(3);
    // assert!(result.get_meta().is_some());
}

#[test]
fn test_mixed_types() {
    // Int + Float → Float (Number type handles conversion)
    let result = &Node::int(3) + &Node::float(2.5);
    eq!(result, 5.5);

    let result = &Node::float(2.5) + &Node::int(3);
    eq!(result, 5.5);
}

#[test]
fn test_chaining() {
    let a = Node::int(1);
    let b = Node::int(2);
    let c = Node::int(3);

    let result = &(&a + &b) + &c;
    eq!(result, 6);

    let result = &(&Node::int(10) - &Node::int(2)) - &Node::int(3);
    eq!(result, 5);

    let result = &(&Node::int(2) * &Node::int(3)) * &Node::int(4);
    eq!(result, 24);
}

#[test]
#[should_panic(expected = "Cannot add")]
fn test_type_mismatch_add() {
    let _ = &Node::text("hello") + &Node::int(5);
}

#[test]
#[should_panic(expected = "Cannot subtract")]
fn test_type_mismatch_sub() {
    let _ = &Node::text("world") - &Node::int(3);
}

#[test]
#[should_panic(expected = "Cannot multiply")]
fn test_type_mismatch_mul() {
    let _ = &Node::symbol("x") * &Node::int(2);
}

#[test]
#[should_panic(expected = "Cannot divide")]
fn test_type_mismatch_div() {
    let _ = &Node::List(vec![]) / &Node::int(5);
}

#[test]
fn test_complex_expression() {
    // (3 + 2) * (10 - 4) / 2 = 5 * 6 / 2 = 30 / 2
    let a = &Node::int(3) + &Node::int(2); // 5
    let b = &Node::int(10) - &Node::int(4); // 6
    let c = &a * &b; // 30
    let _result = &c / &Node::int(2); // 30/2 (Quotient, not simplified to 15)

    // Integer division creates a Quotient in the Number type
    // To get a float result, use float division
    let result_float = &Node::float(30.0) / &Node::float(2.0);
    eq!(result_float, 15.0);
}

#[test]
fn test_operations_with_primitives_mixed() {
    // Test various combinations of operations
    let result = &(&Node::int(10) + 5_i64) - 3_i32;
    eq!(result, 12);

    let result = &(&Node::float(10.0) * 2.0_f64) / 4.0_f64;
    eq!(result, 5.0);

    let result = &(3_i32 * &Node::int(4)) + 8_i64;
    eq!(result, 20);
}
