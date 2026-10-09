//! Generators (card generators-function): a function whose body yields. A loop over its call runs lazily, any other
//! call collects what it yields into a list (src/lowering/generators.rs)
use crate::is;
use warp::ints;

const COUNT_TO: &str = "count_to(n) := { k = 1; while k <= n { yield k; k += 1 } }\n";
const NATURALS: &str = "naturals() := { k = 1; while yes { yield k; k += 1 } }\n";

#[test]
fn a_generator_call_collects_what_it_yields() {
	is!("abc() := { yield 1; yield 2; yield 3 }\nabc()", ints(vec![1, 2, 3]));
	is!(&format!("{COUNT_TO}count_to(4)"), ints(vec![1, 2, 3, 4]));
	is!(&format!("{COUNT_TO}sum(count_to(4))"), 10);
	is!("evens(n) := { for i in 1 to n { if i % 2 == 0 { yield i } } }\nevens(7)", ints(vec![2, 4, 6]));
}

#[test]
fn a_loop_over_a_generator_runs_its_body_per_yield() {
	is!(&format!("{COUNT_TO}s = 0\nfor x in count_to(4) {{ s += x }}\ns"), 10);
	is!(&format!("{COUNT_TO}k = 100\nfor count_to(3) {{ k += it }}\nk"), 106);
	is!("abc() := { yield 1; yield 2; yield 3 }\nn = 0\nfor x in abc() { for y in abc() { n += x * y } }\nn", 36);
}

#[test]
fn break_stops_an_endless_generator() {
	is!(&format!("{NATURALS}t = 0\nfor x in naturals() {{ if x > 5 {{ break }}; t += x }}\nt"), 15);
	is!(&format!("{NATURALS}last = 0\nfor x in naturals() {{ if x > 1000000 {{ break }}; last = x }}\nlast"), 1000000);
	is!("fib() := { a = 0; b = 1; while yes { yield a; t = a; a = b; b = t + b } }\nlast = 0\nfor f in fib { if f > 50 { break }; last = f }\nlast", 34);
}

#[test]
fn continue_goes_on_after_the_yield() {
	is!(&format!("{NATURALS}t = 0\nfor x in naturals() {{ if x == 2 {{ continue }}; if x > 5 {{ break }}; t += x }}\nt"), 13);
}

#[test]
fn return_ends_a_generator() {
	let squares = "squares_below(limit) := { k = 1; while yes { if k * k > limit { return }; yield k * k; k += 1 } }\n";
	is!(&format!("{squares}squares_below(50)"), ints(vec![1, 4, 9, 16, 25, 36, 49]));
	is!(&format!("{squares}q = 0\nfor v in squares_below(50) {{ q += v }}\nq"), 140);
}

#[test]
fn python_generator() {
	is!("def squares(n):\n    for i in range(n):\n        yield i * i\n\nsum(squares(4))", 14);
	is!("def squares(n):\n    for i in range(n):\n        yield i * i\n\nt = 0\nfor s in squares(4):\n    t += s\nt", 14);
}

#[test]
fn a_loop_ending_in_break_has_a_value() {
	is!("while 1 { break }", 0);
	is!("while 1 { while 1 { break }; break }", 0);
}

const COUNTDOWN: &str = "class Countdown {\n  n: int\n  next() := { if n <= 0 { return ø }; n -= 1; n + 1 }\n}\n";

#[test]
fn a_loop_walks_an_object_with_next_until_it_gives_nothing() {
	is!(&format!("{COUNTDOWN}c = Countdown(3)\ns = 0\nfor x in c {{ s = s * 10 + x }}\ns"), 321);
	is!(&format!("{COUNTDOWN}s = 0\nfor x in Countdown(5) {{ if x == 4 {{ continue }}; if x == 1 {{ break }}; s += x }}\ns"), 10);
}

#[test]
fn a_changing_method_returns_early() {
	is!("class C { n: int; step() := { if n <= 0 { return 0 }; n -= 1; n } }\nc = C(1)\na = c.step()\nb = c.step()\n[a, b, c.n]", ints(vec![0, 0, 0]));
	is!("class C { n: int; tick() := { if n > 5 { return }; n += 1 } }\nc = C(5)\nc.tick()\nc.tick()\nc.n", 6);
}
