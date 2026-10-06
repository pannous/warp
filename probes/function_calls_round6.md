# round 6 of ported call forms: code ||| expected
$sq = fn($x) => $x * $x; $sq(4) ||| 16
int sq(int x) => x * x; sq(5) ||| 25
var sq = (int x) => x * x; sq(3) ||| 9
let sq x = x * x; sq 6 ||| 36
let add a b = a + b; add 2 3 ||| 5
def add(x: Int)(y: Int): Int = x + y; add(1)(2) ||| 3
func makeAdder(_ k: Int) -> (Int) -> Int { return { $0 + k } }; makeAdder(2)(3) ||| 5
function add(...nums) { return nums.reduce((a, b) => a + b, 0) }; add(1, 2, 3) ||| 6
const add = (a, b = a * 2) => a + b; add(3) ||| 9
def f(a, *, b): return a * b; f(2, b=5) ||| 10
def f(*args): return len(args); f(1, 2, 3) ||| 3
fun sum(vararg xs: Int): Int = xs.sum(); sum(1, 2, 3) ||| 6
func sum(_ xs: Int...) -> Int { xs.reduce(0, +) }; sum(1, 2, 3) ||| 6
static int Sum(params int[] xs) => xs.Sum(); Sum(1, 2, 3) ||| 6
def fact(n) = n <= 1 ? 1 : n * fact(n - 1); fact(5) ||| 120
fact(n) = n <= 1 ? 1 : n * fact(n - 1); fact(5) ||| 120
compose = (f, g) => x => f(g(x)); inc = x => x + 1; dbl = x => x * 2; compose(inc, dbl)(5) ||| 11
apply = (f, x) => f(x); apply(x => x * 3, 4) ||| 12
[1, 2, 3].map(x => x * x).filter(x => x > 1) ||| [4 9]
[1, 2, 3].reduce((a, b) => a + b) ||| 6
xs = [3, 1, 2]; xs.sort() ||| [1 2 3]
square x := x * x; sum square [1 2 3] ||| 14
def greet(name: str = "you") -> str: return f"hi {name}"; greet() ||| hi you
fun greet(name: String = "you") = "hi $name"; greet("Bo") ||| hi Bo
func greet(name: String = "you") -> String { "hi \(name)" }; greet() ||| hi you
