# round 4 of ported call forms: code ||| expected
func sum(nums ...int) int { total := 0; for _, n := range nums { total += n }; return total }; sum(1, 2, 3) ||| 6
func divmod(a, b int) (int, int) { return a / b, a % b }; q, r := divmod(7, 2); q * 10 + r ||| 31
function add(a, b) return a + b end; add(1, 2) ||| 3
function f($a) { return $a * 2; }; f(4) ||| 8
f = fn x -> x * 2 end; f.(4) ||| 8
f <- function(x) x * 2; f(4) ||| 8
int twice(int x) => x * 2; twice(4) ||| 8
def twice(x: Int): Int = x * 2; twice(4) ||| 8
twice x = x * 2; twice 4 ||| 8
fun f(a: Int, b: Int = 2) = a * b; f(b = 3, a = 4) ||| 12
[1,2,3].map { $0 * 2 } ||| [2 4 6]
def f(a, b = 2) a + b end; f(1) ||| 3
def each_twice; yield 1; yield 2; end; total = 0; each_twice { |x| total += x }; total ||| 3
(function(){ return 5 })() ||| 5
(() => 7)() ||| 7
(lambda x: x + 1)(2) ||| 3
const f = ({a, b}) => a + b; f({a: 1, b: 2}) ||| 3
def f(**kw): return kw["a"]; f(a=1) ||| 1
func apply(_ f: (Int) -> Int, _ x: Int) -> Int { f(x) }; apply({ $0 + 1 }, 2) ||| 3
xs.reduce(0) { $0 + $1 } where xs = [1,2,3] ||| 6
let xs = [1,2,3]; xs.reduce(0, +) ||| 6
sum(x * x for x in [1, 2, 3]) ||| 14
[x * 2 for x in [1, 2, 3] if x > 1] ||| [4 6]
