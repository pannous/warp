# more ported call forms: code ||| expected
fun f(a: Int = 1, b: Int = 2): Int { return a * b }; f(b = 5) ||| 5
fun f(a: Int, b: Int): Int = a + b; f(1, 2) ||| 3
function f(x; y=2) x*y end; f(3) ||| 6
function f(x; y=2) x*y end; f(3; y=4) ||| 12
f(x) = 2x; f(4) ||| 8
def f(*args): return len(args); f(1,2,3) ||| 3
def f(**kw): return kw["a"]; f(a=1) ||| 1
int Add(int a, int b = 2) { return a + b; }; Add(1) ||| 3
static int Add(int a, int b) => a + b; Add(1, 2) ||| 3
const f = (a, b = 2) => a * b; f(4) ||| 8
let f = (a: number): number => a * 2; f(4) ||| 8
def f(a, b: 2) a * b end; f(3) ||| 6
def f(a, b: 2) a * b end; f(3, b: 5) ||| 15
func f(_ a: Int, times b: Int) -> Int { a * b }; f(3, times: 4) ||| 12
[1,2,3].forEach { print(it) } ||| 3
xs = [1,2,3]; xs.each { |x| print x } ||| 3
def twice(f, x){ f(f(x)) }; twice(x => x + 1, 5) ||| 7
compose = (f, g) => x => f(g(x)); h = compose(x => x + 1, x => x * 2); h(5) ||| 11
add = a => b => a + b; add(1)(2) ||| 3
