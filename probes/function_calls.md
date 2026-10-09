# defaults (Python def f(a, b=2))
f(a, b=2) := a + b; f(1) ||| 3
f(a, b=2) := a + b; f(1, 5) ||| 6
def f(a, b=2){a+b}; f(1) ||| 3
def f(a=1, b=2){a*10+b}; f() ||| 12
def f(a=1, b=2){a*10+b}; f(b=5) ||| 15
def greet(name, greeting="Hello"){greeting + " " + name}; greet("Bob") ||| Hello Bob
def greet(name, greeting="Hello"){greeting + " " + name}; greet("Bob", greeting="Hi") ||| Hi Bob
# default depending on earlier param (Kotlin/Swift/JS)
def f(a, b=a*2){a+b}; f(3) ||| 9
def f(a, b=a*2){a+b}; f(3, 1) ||| 4
fun area(w:int, h:int=w){w*h}; area(4) ||| 16
# typed defaults
def f(a:int, b:int=2) {a+b}; f(1) ||| 3
def f(x:float=1.5){x*2}; f() ||| 3
# keyword args mixed (Python/C#/Swift)
def f(a, b, c){a*100+b*10+c}; f(1, c=3, b=2) ||| 123
def f(a, b, c){a*100+b*10+c}; f(c:3, a:1, b:2) ||| 123
def f(a, b=5, c=7){a*100+b*10+c}; f(1, c=3) ||| 153
# variadic (Python *args, JS ...rest, Kotlin vararg)
def total(xs...){sum xs}; total(1,2,3) ||| 6
def total(...xs){sum xs}; total(1,2,3) ||| 6
def count(xs...){#xs}; count(1,2,3,4) ||| 4
def first(a, rest...){a}; first(7,8,9) ||| 7
def f(a, rest...){#rest}; f(7,8,9) ||| 2
# spread (JS f(...xs), Python f(*xs))
def f(a,b,c){a+b+c}; xs=[1,2,3]; f(...xs) ||| 6
# optional/nullable (Swift/Kotlin)
def f(x?){x ?? 5}; f() ||| 5
def f(x?){x ?? 5}; f(2) ||| 2
# returns
def f(x):int{return x+1}; f(1) ||| 2
def f(x){if x>0 {return 1}; return 2}; f(-1) ||| 2
def divmod(a,b){(a//b, a%b)}; q,r = divmod(7,2); q*10+r ||| 31
# overloading by arity
def f(a){a}; def f(a,b){a+b}; f(1)*10 + f(2,3) ||| 15
def f(x:int){1}; def f(x:text){2}; f(3)*10+f("a") ||| 12
# closures (JS/Ruby/Julia)
def adder(n){ x => x + n }; add2 = adder(2); add2(3) ||| 5
def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c() ||| 2
make_multiplier(k) := { it * k }; times3 = make_multiplier(3); times3(4) ||| 12
# partial application
add(a,b) := a+b; inc = add(1, _); inc(5) ||| 6
# higher-order
apply(f, x) := f(x); apply(x=>x*x, 4) ||| 16
twice(f, x) := f(f(x)); twice(x=>x+3, 1) ||| 7
map([1,2,3], x=>x*2) ||| [2 4 6]
[1,2,3].map(x=>x*2) ||| [2 4 6]
compose(f,g) := x => f(g(x)); h = compose(x=>x+1, x=>x*2); h(5) ||| 11
# recursion
fact(n) := n<2 ? 1 : n*fact(n-1); fact(5) ||| 120
def gcd(a,b){ b==0 ? a : gcd(b, a%b) }; gcd(48,18) ||| 6
def ack(m,n){ if m==0 {n+1} else if n==0 {ack(m-1,1)} else {ack(m-1, ack(m, n-1))} }; ack(2,3) ||| 9
# mutual recursion
def even(n){ n==0 ? 1 : odd(n-1) }; def odd(n){ n==0 ? 0 : even(n-1) }; even(10) ||| 1
# without parentheses (warp/Ruby style)
square(n) := n*n; square 3 ||| 9
square(n) := n*n; square square 2 ||| 16
add(a,b) := a+b; add 1 2 ||| 3
def f(a, b=2){a+b}; f 1 ||| 3
# Julia one-liners
f(x, y=10) = x + y; f(1) ||| 11
# Ruby keyword args
def f(a, b: 2){a+b}; f(1) ||| 3
def f(a, b: 2){a+b}; f(1, b: 5) ||| 6
def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c() ||| 2
def counter(){ n=0; ()=>{ n=n+1; n } }; c=counter(); c(); c() ||| 2
def counter(){ n=0; return ()=>{ n+=1; return n } }; c=counter(); c(); c() ||| 2
def f(a:int, b=2) {a+b}; f(1) ||| 3
def f(a, b:int=2) {a+b}; f(1) ||| 3
def f(b:int=2) {b}; f() ||| 2
f(a:int, b:int=2) := a+b; f(1) ||| 3
def f(a:int, b:int=2) {a+b}; f(1, 4) ||| 5
def f(a){a}; def f(a,b){a+b}; f(2,3) ||| 5
def f(a,b){a+b}; def f(a){a}; f(2) ||| 2
mk(k) := { return x => x * k }; t = mk(3); t(4) ||| 12
mk(k) := x => x * k; t = mk(3); t(4) ||| 12
mk(k) := { it * k }; t = mk(3); t(4) ||| 12
square(n) := n*n; square(square 2) ||| 16
def apply_n(f, n, x){ for i in 1...n { x = f(x) }; x }; apply_n(x=>x*2, 3, 1) ||| 8
filter([1,2,3,4], x => x%2==0) ||| [2 4]
reduce([1,2,3,4], (a,b) => a+b) ||| 10
[1,2,3].reduce((a,b)=>a*b) ||| 6
fold([1,2,3], 0, (a,b)=>a+b) ||| 6
sorted([3,1,2], (a,b) => b-a) ||| [3 2 1]
def make_adder(n) = x -> x + n; make_adder(5)(1) ||| 6
adder = (a, b) => a + b; adder(2, 3) ||| 5
mul = fn(a, b) { a * b }; mul(2, 3) ||| 6
f = lambda x: x * 2; f(4) ||| 8
sq = { |x| x * x }; sq(5) ||| 25
sq = x -> x * x; sq(5) ||| 25
(x => x + 1)(2) ||| 3
square(x) := x*x; def f(g){ g(2) }; f(square) ||| 4
square(x) := x*x; [1,2,3].map(square) ||| [1 4 9]
square(x) := x*x; map([1,2,3], square) ||| [1 4 9]
def f(x: int) -> int { x + 1 }; f(1) ||| 2
fun f(x: Int): Int = x + 1; f(1) ||| 2
func f(x: Int) -> Int { return x + 1 }; f(1) ||| 2
function f(x: number): number { return x + 1 }; f(1) ||| 2
int f(int x) { return x + 1; }; f(1) ||| 2
def sign(x){ if x < 0 { return -1 }; if x == 0 { return 0 }; 1 }; sign(-5) + sign(0) + sign(7) ||| 0
def first_even(xs){ for x in xs { if x % 2 == 0 { return x } }; -1 }; first_even([1,3,4,5]) ||| 4
def fib(n){ n < 2 ? n : fib(n-1) + fib(n-2) }; fib(15) ||| 610
def power(b, e){ if e == 0 {1} else { b * power(b, e-1) } }; power(2, 10) ||| 1024
def sum_list(xs){ if #xs == 0 {0} else { xs#1 + sum_list(xs[1:]) } }; sum_list([1,2,3]) ||| 6
def hanoi(n){ n == 0 ? 0 : 2*hanoi(n-1)+1 }; hanoi(10) ||| 1023
func greet(person name: String) -> String { "Hi " + name }; greet(person: "Bob") ||| Hi Bob
def f(a, b) { a - b }; f(b: 1, a: 10) ||| 9
def twice(x) { x * 2 }; twice 21 ||| 42
def add(a, b) { a + b }; add 1, 2 ||| 3
def greet(name) { "hi " + name }; greet "bob" ||| hi bob
puts(x) := x; puts 3 ||| 3
def f(a, b=max(a, 3)){ b }; f(1) ||| 3
def f(xs=[]){ #xs }; f() ||| 0
def f(xs=[1,2]){ #xs }; f() ||| 2
def f(s="a"){ s + s }; f() ||| aa
def minmax(xs){ return min(xs), max(xs) }; lo, hi = minmax([3,1,2]); lo*10+hi ||| 13
def swap(a, b){ (b, a) }; x, y = swap(1, 2); x*10+y ||| 21
def counter_factory(){ count = 0; def inc(){ nonlocal count; count += 1; count }; inc }; c = counter_factory(); c(); c() ||| 2
x = 10; def f(){ x * 2 }; f() ||| 20
def outer(){ y = 5; def inner(){ y + 1 }; inner() }; outer() ||| 6
