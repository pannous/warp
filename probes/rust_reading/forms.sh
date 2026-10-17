#!/bin/bash
# Card rust-reading: how close warp comes to reading literal Rust. Each form runs through warp; the line shows the
# value, the expected value after `want`. Usage: probes/rust_reading/forms.sh [warp binary]
WARP="${1:-warp}"
run() { # name, expected, code
	local got
	got=$(WARP_NO_WINDOW=1 timeout 20 "$WARP" "$3" 2>&1 | grep -v '^hide\|^hint\|^      ' | tr '\n' ' ' | sed 's/^» //; s/ *$//')
	local mark="ok "
	[ "$got" = "$2" ] || mark="-- "
	printf '%s%-22s want %-10s got %s\n' "$mark" "$1" "$2" "$got"
}
run let 3 'let x = 3; x'
run let-mut 4 'let mut x = 3; x += 1; x'
run typed-let 3 'let x: i32 = 3; x'
run fn-return 9 'fn square(x: i32) -> i32 { x * x } square(3)'
run fn-explicit-return 9 'fn square(x: i32) -> i32 { return x * x; } square(3)'
run fn-main 7 'fn main() { println!("{}", 7); }'
run println-macro 7 'println!("{}", 7)'
run if-else 1 'let x = 5; if x > 3 { 1 } else { 0 }'
run match-literal '"two"' 'let n = 2; match n { 1 => "one", 2 => "two", _ => "many" }'
run match-range '"big"' 'let n = 50; match n { 0..=9 => "small", _ => "big" }'
run for-range 10 'let mut s = 0; for i in 0..5 { s += i; } s'
run for-inclusive 15 'let mut s = 0; for i in 1..=5 { s += i; } s'
run while 8 'let mut x = 1; while x < 5 { x *= 2; } x'
run loop-break 3 'let mut i = 0; loop { i += 1; if i == 3 { break; } } i'
run vec-macro 3 'let v = vec![1, 2, 3]; v.len()'
run vec-index 2 'let v = vec![1, 2, 3]; v[1]'
run iter-sum 6 'let v = vec![1, 2, 3]; v.iter().sum::<i32>()'
run iter-map-collect '[2 4 6]' 'let v = vec![1, 2, 3]; v.iter().map(|x| x * 2).collect::<Vec<i32>>()'
run closure 5 'let add = |a, b| a + b; add(2, 3)'
run struct-literal 3 'struct P { x: i32, y: i32 } let p = P { x: 3, y: 4 }; p.x'
run impl-method 7 'struct P { x: i32, y: i32 } impl P { fn sum(&self) -> i32 { self.x + self.y } } let p = P { x: 3, y: 4 }; p.sum()'
run enum-match 1 'enum C { Red, Green } let c = C::Red; match c { C::Red => 1, C::Green => 2 }'
run option-some 3 'let o = Some(3); match o { Some(v) => v, None => 0 }'
run if-let 3 'let o = Some(3); if let Some(v) = o { v } else { 0 }'
run string-from '"hi"' 'let s = String::from("hi"); s'
run format-macro '"a=1"' 'format!("a={}", 1)'
run as-cast 3 'let x = 3.7 as i32; x'
run tuple 2 'let t = (1, 2); t.1'
run shadowing 6 'let x = 3; let x = x * 2; x'
run ref-deref 3 'let x = 3; let r = &x; *r'
run line-comment 3 '// note
let x = 3; x'
run doc-comment 3 '/// doc
fn three() -> i32 { 3 } three()'
run question-mark 2 'fn f(x: Option<i32>) -> Option<i32> { let v = x?; Some(v + 1) } f(Some(1)).unwrap()'
run trait-impl 4 'trait Area { fn area(&self) -> i32; } struct S { a: i32 } impl Area for S { fn area(&self) -> i32 { self.a * self.a } } S { a: 2 }.area()'
run generic-fn 3 'fn id<T>(x: T) -> T { x } id(3)'
run const 10 'const N: i32 = 10; N'
run use-std 3 'use std::collections::HashMap; let mut m = HashMap::new(); m.insert("a", 3); m["a"]'
