# Reading literal Rust (card rust-reading)

How close warp comes to running Rust as written. Probe: `probes/rust_reading/forms.sh [warp binary]`, 37 typical
forms, each with its expected value (2026-10-13, branch hint-cards).

## Works (30 of 37 on main, 2026-10-17)
`let`, `let mut`, `let x: i32 = 3`, `if c { a } else { b }`, `match n { 1 => …, _ => … }` (literal arms), `for i in 0..5`,
`1..=5` (in `for`, match arms and as a value), `while`, `fn square(x: i32) -> i32 { x * x }` (with or without `return`;
a function named like a type word, `fn double`, is refused by decision P20), closures `|a, b| a + b`, `3.7 as i32`,
tuples `t.1`, shadowing `let x = x * 2`, `//` and `///` comments, generic functions `fn id<T>(x: T) -> T`,
`const N: i32 = 10`.
Macros (card rust-macros, src/warp_parser/rust_macros.rs): `println!`/`print!`/`format!` with `{}`, `{0}`, `{x}` and
`{:?}` placeholders, `vec![…]`, `assert!`/`assert_eq!`/`assert_ne!`, `panic!`/`todo!`/`unreachable!`, any other
`name!(args)` the call `name(args)`.
Paths (card rust-paths, src/lowering/welcoming/rust_paths.rs): turbofish `sum::<i32>()` dropped, `std::cmp::max`,
`f64::sqrt`, `std::f64::consts::PI`, `i32::MAX`, `String::from`/`String::new`, `Vec::new`, `HashMap::new`, enum
`C::Red`, `.iter()`/`.into_iter()`/`.collect()` the list itself, `.to_string()`.
References (card rust-refs): `&x`, `&mut x`, `&&x`, `&5`, `*r`, `**r`, `*(&x)` read as the value itself. A `*name`
stays a star (rest or spread) after a comma or an opening bracket, and at a statement start before `,` or `=`
(src/warp_parser/definitions/functions.rs starts_rust_reference).

## Rule (user decision rust-forms-welcome, notes/decisions.md)
warp may read Rust's own forms, generics `<T>` included, as long as each one has a nicer warp form of its own: the
Rust form is an accepted alternative, the warp form stays canonical (the normalizer may hint it). Each gap card below
names its warp form.

## Gaps, by how much Rust they unlock
1. Option and control: `Some(v)`/`None`, `if let Some(v) = o`, a match of Some/None arms, `unwrap_or`/`expect`/
   `is_some`, `loop { … break }` work (card rust-option, probes/rust_reading/option.warp). Still open: Result
   `Ok`/`Err` and the `?` operator, `loop { break value }`, `-> Option<i32>` as a return type (card rust-generic-result).
2. Items: `struct P { x: i32 }` followed by more code ("undefined variable: let"), `impl P { fn sum(&self) … }`,
   `trait T { fn f(&self) -> i32; }`, `impl T for S`, `enum` before code (card rust-items). The class machinery exists
   (classes.md); the Rust spellings need mapping onto it.
3. `fn main() { … }` beside the program's own main: "duplicate export name main" (card rust-fn-main).
4. Smaller cards: rust-map-insert (`m.insert(k, v)` on a map), rust-map-type (`HashMap<K, V>` as a type),
   rust-generic-result (`fn f() -> Vec<i32> { … }` leaves f undefined), rust-vec-repeat (`vec![0; n]`),
   rust-generics (`<T>` in struct type parameters).

Warp forms the gaps map onto:
| card | Rust | warp (canonical) |
|---|---|---|
| (works) | `fn square(x: i32) -> i32 { x * x }` | `square(x:int) := x * x` |
| (works) | `println!("{}", x)`, `format!("a={}", 1)`, `vec![1, 2, 3]` | `print "\(x)"`, `"a=\(1)"`, `[1 2 3]` |
| (works) | `C::Red`, `String::from("hi")`, `HashMap::new()` | `C.Red`, `"hi"`, `{}` |
| (works) | `&x`, `*r` | `x` (values need no references) |
| rust-option | `Some(v)`/`None`, `if let Some(v) = o`, `o.unwrap_or(0)` | the value / `ø`, `if o {…}`, `o ?? 0` |
| rust-items | `struct P { x: i32 }`, `impl P { fn sum(&self) … }`, `impl T for S` | `class P { x:int; sum() := x + y }`, `class S {…} is T` |
| rust-generics | `fn id<T>(x: T) -> T`, `Vec<Person>` | untyped `id(x) := x`, the plural `persons` (wiki/array.md) |

## Shape
Each gap is a parser or lowering alias of something warp has: no Rust mode, the same as the Python, Swift and Kotlin
forms (tests/*/test_*_ported.rs, test_python_*.rs). Rust types (`i32`, `Vec<i32>`, `&self`) are read as warp types or
dropped. Ownership, lifetimes and borrow checking stay out of scope: `&`, `&mut` and `'a` are read and ignored.
