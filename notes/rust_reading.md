# Reading literal Rust (card rust-reading)

How close warp comes to running Rust as written. Probe: `probes/rust_reading/forms.sh [warp binary]`, 37 typical
forms, each with its expected value (2026-10-13, branch hint-cards).

## Works (14 of 37)
`let`, `let mut`, `let x: i32 = 3`, `if c { a } else { b }`, `match n { 1 => …, _ => … }` (literal arms), `for i in 0..5`,
`while`, closures `|a, b| a + b`, `3.7 as i32`, tuples `t.1`, shadowing `let x = x * 2`, `//` and `///` comments,
generic functions `fn id<T>(x: T) -> T`, `const N: i32 = 10`.

## Rule (user decision rust-forms-welcome, notes/decisions.md)
warp may read Rust's own forms, generics `<T>` included, as long as each one has a nicer warp form of its own: the
Rust form is an accepted alternative, the warp form stays canonical (the normalizer may hint it). Each gap card below
names its warp form.

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
7. References (card rust-refs, done): `&x`, `&mut x`, `&&x`, `&5`, `*r`, `**r`, `*(&x)` read as the value itself. A `*name` stays a star (rest or spread) after a comma or an opening bracket, and at a statement start before `,` or `=` (src/warp_parser/definitions/functions.rs starts_rust_reference).
8. Generics `<T>` in fn and struct type parameters (card rust-generics).

Warp forms the gaps map onto:
| card | Rust | warp (canonical) |
|---|---|---|
| rust-fn-arrow | `fn double(x: i32) -> i32 { x * 2 }` | `double(x:int) := x * 2` |
| rust-macros | `println!("{}", x)`, `format!("a={}", 1)`, `vec![1, 2, 3]` | `print "\(x)"`, `"a=\(1)"`, `[1 2 3]` |
| rust-inclusive-range | `1..=5` | `1…5`, `1 to 5` |
| rust-items | `struct P { x: i32 }`, `impl P { fn sum(&self) … }`, `impl T for S` | `class P { x:int; sum() := x + y }`, `class S {…} is T` |
| rust-paths | `C::Red`, `String::from("hi")`, `HashMap::new()` | `C.Red`, `"hi"`, `{}` |
| rust-option | `Some(v)`/`None`, `if let Some(v) = o`, `o.unwrap_or(0)` | the value / `ø`, `if o {…}`, `o ?? 0` |
| rust-refs | `&x`, `*r` | `x` (values need no references) |
| rust-generics | `fn id<T>(x: T) -> T`, `Vec<Person>` | untyped `id(x) := x`, the plural `persons` (wiki/array.md) |

## Shape
Each gap is a parser or lowering alias of something warp has: no Rust mode, the same as the Python, Swift and Kotlin
forms (tests/*/test_*_ported.rs, test_python_*.rs). Rust types (`i32`, `Vec<i32>`, `&self`) are read as warp types or
dropped. Ownership, lifetimes and borrow checking stay out of scope: `&`, `&mut` and `'a` are read and ignored.
