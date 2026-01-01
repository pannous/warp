

// Test functions to assert! wasm->runtime interaction

fn test42() -> int {
    return 42;
}

fn test42i(i:int) -> int {
    // used in wasm runtime test
    return 42 + i;
}

fn test42f(f:float ) -> float {
    return 42 + f;
}

// default args don't work in wasm! (how could they?);
// fn test41ff(f:float  = 0) -> float {
//     return 41.4 + f;
// }

fn not_ok() {
    panic!("not_ok panics on purpose"); // assert_throws test
}
