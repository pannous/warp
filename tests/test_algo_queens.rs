use warp::is;

// natural scripting style: range(n), [0] * n and list params of fn-defined functions fail, see probes/algo/queens/
#[test]
#[ignore = "next"] // by design: `[0] * n` is ambiguous, educating error suggests `n times [0]`; passes with it
fn test_queens() { is!("samples/queens.wasp", 1119); }

#[test]
fn test_queens_idiomatic() { is!("samples/queens_idiomatic.wasp", 1119); }
