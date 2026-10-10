# warp primer

warp is a data notation and a programming language compiled to WebAssembly GC. A program is a value: its last
expression is the result. Below, a block headed `=> v` evaluates to `v`; a `printed` block shows its output.
Every snippet is run by tests/web/test_primer.rs.

## Data
JSON5-like: quotes, commas and semicolons optional; `//` and `/* */` comments; `name{…}` is a tagged object.
```warp => 30
person = { name: "Ada" age: 30 tags: ["math" "code"] address: {city: "London"} }
person.age
```
```warp => "London"
person = {name: "Ada", address: {city: "London"}}; person.address.city
```

## Numbers
Integers are unbounded, decimals and fractions exact, `/` never truncates (`//` floors); `√ ∛ ² π` are operators.
```warp => [1267650600228229401496703205376 yes yes 3.5 3 2]
[2^100, 0.1 + 0.2 == 0.3, 1/3 + 1/6 == 1/2, 7/2, 7//2, √4]
```
Numbers carry units, converted on arithmetic; `±` is an uncertain value.
```warp => 150min
2h + 30min
```
```warp => [5.7 6.3]
x = 6.0 ± 0.3; [x.low, x.high]
```

## Text
`+` joins (numbers too), `\(…)` interpolates, `#t` counts characters, `3 times "ab"` repeats.
```warp => ["abcd" "n=7" 4 "WARP" "ababab" "rp"]
n = 7; ["ab" + "cd", "n=\(n)", #"warp", "warp".upper, 3 times "ab", "warp"#(3…)]
```

## Truth and conditions
`yes`/`no` (true/false); `and or not` (`&& || !` work, `&`/`|` are logical too); only `0`, `no`, `ø` and empty are falsy.
`if` is an expression with `:`, `then`, or braces; `else if` chains; `c ? a : b` works too; `a or b` gives a fallback.
```warp => ["mid" "big" 5 "none"]
x = 5
size = if x > 9 : "big" else if x > 3 : "mid" else "small"
[size, if x > 3 then "big" else "small", if x > 3 then x else 0, "" or "none"]
```
```warp => "three"
switch 3 { 1: "one" 3: "three" default: "many" }
```

## Variables
`=` binds and rebinds; `+= -= *= ++`; `:=` defines a value that recomputes from what it reads (a function of `it`).
```warp => [6 8]
x = 3; y = x * 2; z := x * 2; x = 4; [y, z]
```

## Lists, ranges, maps
Items separated by spaces or commas; `xs#1` is the first item (`xs[0]` too: brackets count from 0); `#xs` counts;
`a…b` and `a to b` include b, `a..b` excludes it; slices `xs#(2…3)`, `xs#(2…)`.
```warp => [4 3 [1 2 3 4] 10 [1 2]]
xs = [3 1 2]; xs.add(4); [#xs, xs#1, sort xs, sum xs, xs#(2…3)]
```
```warp => [[1 4 9] [2 4 6] [3] [1 2 3]]
[[1 2 3].map(x => x * x), (1…3).map(it * 2), [1 2 3].filter(it > 2), 1..4]
```
`xs where …` keeps the items a condition holds for (`it` is the item), in comprehensions too (or `if`); a function of a
number broadcasts over a list.
```warp => [[3 4] [9 16] [1 4 9]]
square := it * it; [[1 2 3 4] where it > 2, [x² for x in 1…4 where x > 2], square [1 2 3]]
```
```warp => [42 ["ann" "bob"]]
ages = {"ann": 31, "bob": 42}; [ages["bob"], keys ages]
```

## Loops
`for x in …`, `for …` with `it`, `while`; a body follows `:`, braces, or indented lines; `for char in text`.
```warp
for i in 1 to 3 { print i }
for 1…2: print it * 10
for char in "ab": print it
n = 0
while n < 2
	n += 1
print n
```
```printed
1
2
3
10
20
a
b
2
```
```warp => 14
total = 0; for (a, b) in [(1, 2), (3, 4)] { total += a * b }; total
```
A type or class name as the loop variable matches by type: `for int in xs` visits only the ints, named `int` in the
body (with a notice when some items may be skipped); `for character in chars(s)` visits every codepoint.
```warp => 4
sum = 0; for int in [1, "a", 3] { sum += int }; sum
```

## Functions
`f(x) := …` or `def f(x) = …` or `def f(x) { … }`; `return`; a one-parameter definition may use `it`; calls need no
parentheses (`f 3-1` is `f(3-1)`); lambdas `x => …`, or just `it` where a function is expected (`xs.map(it * 2)`);
functions are values and closures.
```warp => [55 42 7 7]
fib(n) := if n < 2 then n else fib(n - 1) + fib(n - 2)
twice := it * 2
add(a) := b => a + b
apply(f, x) := f(f(x))
[fib 10, twice 21, add(3)(4), apply(x => x + 3, 1)]
```
```warp => yes
infix operator divides(d: int, n: int) := n % d == 0; 3 divides 12
```

## Types and classes
`type(x)`, `x is T` tests (`3 is number`), `x as T` converts; a bare type name is a type value and `==` between
types is exact (`type(3) == int`, `type(3) == number` is `no`); classes declare typed fields or defaults, methods with `:=`, `extends`.
Instances are shared references: `==` compares values, `===` identity, `copy(x)` duplicates.
```warp => [int yes 3 "Rex barks"]
class Animal { name: "?"; speak := "..." }
class Dog extends Animal { speak := name + " barks" }
[type(3), "3" is text, "3" as int, Dog{name: "Rex"}.speak]
```
```warp => [150 no yes]
class Account { owner: text; balance: int }
a = Account("Ada", 100); b = a; b.balance = 150; c = copy(a)
[a.balance, c === a, c == a]
```

## Errors
Errors are values: `error("…")` returns one and `if r failed then …` handles it, no catch needed; a raised error
(`raise`, an index out of range) propagates to `try … else` or `catch e`, never a crash.
```warp => ["caught" "bad input" "neg"]
r = try { [1]#5 } else { "caught" }
check(x) := if x < 0 then raise "bad input" else x
f(x) := if x < 0 then error("neg") else x
s = f(-1)
[r, try check(-1) catch e: e.message, if s failed then s.message else s]
```

## Effects
The compiler infers what a function does (IO, …); `! Pure` declares none. Effect handlers: `emit word` asks, the
caller answers with `on word { … } in { … }`.
```warp => 120
price(net) := net * (1 + emit tax_rate)
on tax_rate { 0.2 } in { price(100) }
```
```warp => [IO Pure]
log(x) := print x
square(x) := x * x ! Pure
[effects of log, effects of square]
```

## Events and reactive values
`emit name{fields}` fires, `on name { … event … }` handles; `whenever cond { … }` runs when cond becomes true;
`on change expr { … }`.
```warp => [5 2]
alarms = 0; on alarm { alarms += event.level }; emit alarm{level: 2}; emit alarm{level: 3}
t = 20; waves = 0; whenever t > 30 { waves += 1 }; t = 35; t = 36; t = 10; t = 40
[alarms, waves]
```

## Tasks and channels
`go { … }` starts a task, `await` its value; `shared` values; `channel()` with `send`, `close`, `for v in ch`.
```warp => [42 4 6]
job = go { 6 * 7 }
shared count = 0; jobs = []
for k in 1 to 4 { jobs.add(go { count += 1 }) }
await all jobs
ch = channel(); go { for k in 1 to 3 { ch.send(k) }; ch.close() }
received = 0; for v in ch { received += v }
[await job, count, received]
```

## Modules and foreign code
`use math` (also text, list, os, time, json, …) imports a standard module; `import f from "lib"` calls C or Rust.
```warp => [6 yes 6]
use math; [factorial 3, is_prime 7, gcd(12, 18)]
```
```warp compiles
import cbrt from "m"
cbrt(27.0)
```

## Pages, servers, databases
Tags are values: `tag{attributes children}`, shown as HTML by the CLI and the page; `li all xs` repeats a tag. One source holds the server and its page:
`get`/`post` routes, `route` pages, `server def` functions the page calls. `stored xs: [Class]` is a database table
(SQLite natively, IndexedDB in the browser); `where` filters run as SQL.
```warp => div{h1:"Fruit" ul{li:"apple" li:"pear"}}
fruits = ["apple" "pear"]; div{ h1{"Fruit"} ul{ li all fruits } }
```
```warp compiles
class User { name: text; age: int }
stored users: [User]
if count(users) == 0 { users.add(User("Ann", 31)); users.add(User("Bo", 17)) }
get "/api/users" { users }
post "/api/users" { users.add(User(request.body, 0)); count(users) }
get "/api/users/:id:int" { users#id }
route "/" { div{ h1{ "Adults: " + count(users where age > 18) } } }
```

## Graphics
`paint(pixels or shader, width, height, values)` draws; `shader { … }` holds WGSL that reads `values`.
```warp compiles
rings = shader {
	@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
		return vec4f(at.x / values.size, 0.5, 0.5, 1.0);
	}
}
paint(rings, 64, 64, {size: 64})
```

## Laws and tests
`test cond` checks at run time (a script with tests reports them: `✓ 1 test passed`); `law` states a property,
proved with Lean when installed, else checked.
```warp
square(x) := x * x; law square(-x) == square(x); test square(3) == 9; square(3)
```

## Decided differences from other languages
- `7/2` is `3.5` (exact); `0.1+0.2 == 0.3`; `010` is 10; integers never overflow.
- `xs#1` is the first item, `xs[0]` too; `last(xs)` the last (negative indexes don't wrap).
- `1 == "1"` is `no`: no loose equality; `"0"` is truthy.
- `=` inside an `if`/`while` condition compares; `3 > 2 > 1` chains like math; `-2^2` is `-4`, `2^3^2` is 512.
- `&`/`|` are logical; next to an ungrouped comparison they are an error: group them. Bits: `xor`.
- A newline ends a statement. A braceless argument spans arithmetic: `f 3-1` is `f(3-1)`; `square 3 + square 3` is an
  ambiguity error that offers both readings.
- An ambiguous or unknown construct is a warning or an error with fixes, never a silent guess.
- A free variable that changes after a function's definition must be declared `global` in it; loop variables are
  captured per iteration.
- A script's last value is echoed by `warp file.warp` (as `» value`) after its printed output.
