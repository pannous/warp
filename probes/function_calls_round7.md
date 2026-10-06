# round 7 of ported call forms: code ||| expected
f(x) = 2x + 1; f(3) ||| 7
sq = x -> x^2; sq(4) ||| 16
map(x -> x * 2, [1, 2, 3]) ||| [2 4 6]
let inc = fun x -> x + 1 in inc 4 ||| 5
List.map (fun x -> x * 2) [1; 2; 3] ||| [2 4 6]
double = func(x int) int { return x * 2 }; double(4) ||| 8
twice := func(x int) int { return x * 2 }; twice(4) ||| 8
sq = @(x) x.^2; sq(3) ||| 9
sapply(c(1, 2, 3), function(x) x * 2) ||| [2 4 6]
int add({required int a, required int b}) => a + b; add(a: 1, b: 2) ||| 3
mk = () => ({a: 1}); mk().a ||| 1
const f = x => y => x + y; f(1)(2) ||| 3
add = lambda a, b=2: a + b; add(1) ||| 3
f = lambda *xs: sum(xs); f(1, 2, 3) ||| 6
(x -> x + 1)(2) ||| 3
[1, 2, 3] |> map(x -> x * 10) ||| [10 20 30]
fn apply<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 { f(x) }; apply(|x| x + 1, 2) ||| 3
let add = |a, b| a + b; add(2, 3) ||| 5
func apply(f func(int) int, x int) int { return f(x) }; apply(func(x int) int { return x + 1 }, 2) ||| 3
square(x) := x * x; square.([1, 2, 3]) ||| [1 4 9]
function f(x) { return x > 0 ? "pos" : "neg" }; f(-1) ||| neg
def f(x): return "pos" if x > 0 else "neg"; f(1) ||| pos
fact = n -> n <= 1 ? 1 : n * fact(n - 1); fact(5) ||| 120
