# round 3 of ported call forms: code ||| expected
fun Int.twice() = this * 2; 3.twice() ||| 6
fun total(vararg xs: Int): Int = xs.sum(); total(1, 2, 3) ||| 6
int Total(params int[] xs) { return xs.sum(); }; Total(1, 2, 3) ||| 6
def f(a, *, b): return a + b; f(1, b=2) ||| 3
f(x) = x^2; f.([1,2,3]) ||| [1 4 9]
map(x -> x * 2, [1,2,3]) ||| [2 4 6]
function f(a, b = a * 2) { return a + b }; f(3) ||| 9
def fact(n, acc=1){ if n <= 1 { acc } else { fact(n - 1, acc * n) } }; fact(5) ||| 120
add = (a) => (b) => a + b; add(2)(3) ||| 5
def apply(f, *args): return f(*args); apply((a, b) => a * b, 3, 4) ||| 12
func swapped(_ a: Int, _ b: Int) -> (Int, Int) { return (b, a) }; swapped(1, 2) ||| (2 1)
def f(x): return x * 2; list(map(f, [1, 2, 3])) ||| [2 4 6]
square = lambda x: x ** 2; square(5) ||| 25
const inc = x => x + 1; [1,2,3].map(inc) ||| [2 3 4]
fn add(a: i32, b: i32) -> i32 { a + b }; add(2, 3) ||| 5
let twice x = x * 2 in twice 4 ||| 8
def greet(name="World"): return "Hello " + name; greet() ||| Hello World
greet = (name = "World") => `Hello ${name}`; greet("Bob") ||| Hello Bob
def f(n){ return n if n < 2 else f(n-1) + f(n-2) }; f(10) ||| 55
max(3, 7) ||| 7
