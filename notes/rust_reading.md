# Reading literal Rust (card rust-reading)

How close warp comes to running Rust as written. Probe: `probes/rust_reading/forms.sh [warp binary]`, 37 typical
forms, each with its expected value (2026-10-13, branch hint-cards).

## Works (14 of 37)
`let`, `let mut`, `let x: i32 = 3`, `if c { a } else { b }`, `match n { 1 => …, _ => … }` (literal arms), `for i in 0..5`,
`while`, closures `|a, b| a + b`, `3.7 as i32`, tuples `t.1`, shadowing `let x = x * 2`, `//` and `///` comments,
generic functions `fn id<T>(x: T) -> T`, `const N: i32 = 10`.

## Gaps, by how much Rust they unlock
1. Return type after `->` (card rust-fn-arrow): `fn double(x: i32) -> i32 { … }` fails with "double is a type". This
   blocks almost every real function; `fn id<T>(x: T) -> T` works, so the plain `-> i32` reading is the issue.
2. Macros `name!(…)` (card rust-macros): `!` reads as `not`. `println!("{}", x)`, `format!`, `vec![…]`, `assert!`,
   `panic!` cover most uses: `{}` placeholders filled in order, `vec!` a list.
3. `..=` (card rust-inclusive-range): an inclusive range in `for` and in match arms; the parser stops at `=`.
4. Items: `struct P { x: i32 }` followed by more code ("undefined variable: let"), `impl P { fn sum(&self) … }`,
   `trait T { fn f(&self) -> i32; }`, `impl T for S` (card rust-items). The class machinery exists (classes.md); the
   Rust spellings need mapping onto it.
5. Paths `A::b` (card rust-paths): `C::Red` of an enum, `String::from("hi")`, `HashMap::new()`, turbofish
   `sum::<i32>()` and `collect::<Vec<i32>>()` (the `<` opens a group that never closes).
6. Option and control (card rust-option): `Some(v)`/`None`, `if let Some(v) = o`, `?`, `.unwrap()`, and `loop { … break }`.
7. References (card rust-refs): `&x` and `*r` read as the value itself (`*r` stays the symbol).

## Shape
Each gap is a parser or lowering alias of something warp has: no Rust mode, the same as the Python, Swift and Kotlin
forms (tests/*/test_*_ported.rs, test_python_*.rs). Rust types (`i32`, `Vec<i32>`, `&self`) are read as warp types or
dropped. Ownership, lifetimes and borrow checking stay out of scope: `&`, `&mut` and `'a` are read and ignored.
