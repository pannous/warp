use warp::is;

// natural first draft: blocked by `else if` inside blocks, `0..size {…}`, fun params typed Int for list variables, `return count`, tuple destructuring
#[test]
#[ignore = "next"]
fn test_life() { is!("samples/life.wasp", 131); }

#[test]
fn test_life_idiomatic() { is!("samples/life_idiomatic.wasp", 131); }
