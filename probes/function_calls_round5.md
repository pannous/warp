# round 5 of ported call forms: code ||| expected
fun add(a: Int, b: Int): Int { return a + b }; add(2, 3) ||| 5
fun Int.half() = this / 2; 8.half() ||| 4
val sq = { x: Int -> x * x }; sq(5) ||| 25
listOf(1, 2, 3).map { it * 2 } ||| [2 4 6]
func add(_ a: Int, to b: Int) -> Int { a + b }; add(1, to: 2) ||| 3
let sq = { (x: Int) -> Int in x * x }; sq(4) ||| 16
static int Add(int a, int b) => a + b; Add(2, 3) ||| 5
Func<int, int> sq = x => x * x; sq(6) ||| 36
const add = (a: number, b: number): number => a + b; add(1, 2) ||| 3
function add(a, b = 10) { return a + b }; add(1) ||| 11
def add(a, b) = a + b; add(1, 2) ||| 3
sq = ->(x) { x * x }; sq.(3) ||| 9
sq = lambda { |x| x * x }; sq.call(3) ||| 9
add(x, y) = x + y; add(2, 3) ||| 5
sq(x) = x^2; sq.([1, 2, 3]) ||| [1 4 9]
local function add(a, b) return a + b end; add(1, 2) ||| 3
int add(int a, int b) => a + b; add(1, 2) ||| 3
proc add(a, b: int): int = a + b; add(1, 2) ||| 3
def sq(x : Int) : Int; x * x; end; sq(3) ||| 9
fn add(a: i32, b: i32) -> i32 { a + b }; add(1, 2) ||| 3
def add(a: Int, b: Int) = a + b; add(1, 2) ||| 3
square x := x * x; square [1 2 3] ||| [1 4 9]
square x := x * x; xs = [1, 2]; square all xs ||| [1 4]
max(1, 5, 3) ||| 5
f = (x) -> x + 1; f(1) ||| 2
