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
def divmod(a,b){(a/b, a%b)}; q,r = divmod(7,2); q*10+r ||| 31
# overloading by arity
def f(a){a}; def f(a,b){a+b}; f(1)*10 + f(2,3) ||| 15
def f(x:int){1}; def f(x:text){2}; f(3)*10+f("a") ||| 12
# closures (JS/Ruby/Julia)
def adder(n){ x => x + n }; add2 = adder(2); add2(3) ||| 5
def counter(){ n=0; ()=>{ n+=1; n } }; c=counter(); c(); c() ||| 2
make_multiplier(k) := { it * k }; times3 = make_multiplier(3); times3(4) ||| 12
# partial application
add(a,b) := a+b; inc = add(1); inc(5) ||| 6
# higher-order
apply(f, x) := f(x); apply(x=>x*x, 4) ||| 16
twice(f, x) := f(f(x)); twice(x=>x+3, 1) ||| 7
map([1,2,3], x=>x*2) ||| [2, 4, 6]
[1,2,3].map(x=>x*2) ||| [2, 4, 6]
compose(f,g) := x => f(g(x)); h = compose(x=>x+1, x=>x*2); h(5) ||| 11
# recursion
fact(n) := n<2 ? 1 : n*fact(n-1); fact(5) ||| 120
def gcd(a,b){ b==0 ? a : gcd(b, a%b) }; gcd(48,18) ||| 6
def ack(m,n){ if m==0 {n+1} else if n==0 {ack(m-1,1)} else {ack(m-1, ack(m, n-1))} }; ack(2,3) ||| 9
# mutual recursion
def even(n){ n==0 ? 1 : odd(n-1) }; def odd(n){ n==0 ? 0 : even(n-1) }; even(10) ||| 1
# without parentheses (wasp/Ruby style)
square(n) := n*n; square 3 ||| 9
square(n) := n*n; square square 2 ||| 16
add(a,b) := a+b; add 1 2 ||| 3
def f(a, b=2){a+b}; f 1 ||| 3
# Julia one-liners
f(x, y=10) = x + y; f(1) ||| 11
# Ruby keyword args
def f(a, b: 2){a+b}; f(1) ||| 3
def f(a, b: 2){a+b}; f(1, b: 5) ||| 6
