# The wasp guide

wasp is a data notation that is also a programming language. Everything you write is a value: a number, a text, a
list, an object, a piece of markup, a function. warp compiles it to WebAssembly while you type. Each chapter starts
easy and ends with the details; press **try ▶** next to a snippet to load it into the editor.

## Hello and comments

A program is a list of expressions; the page shows the value of the last one. `print` writes a line on the way.
`//` starts a comment to the end of the line, `/* … */` a comment anywhere.

```wasp => "Hello, world"
print "starting" // a comment
/* a block comment */
"Hello, world"
```

Separate expressions with new lines or `;`.

```wasp => 6
x = 3; x * 2
```

Examples: welcome; samples: hello

## Numbers

Integers are exact and unbounded, division gives an exact fraction where it can, and units travel with their numbers.

```wasp => 1267650600228229401496703205376
2 ^ 100
```

```wasp => 3.5
7 / 2
```

```wasp => 0.5
1/3 + 1/6
```

```wasp => 3200 m
3 km + 200 m
```

`mod` is the remainder, `√` the root (`**` works as a power too, `^` is preferred). Hexadecimal, underscores and
exponents are written as usual:

```wasp => [256 1 4 31 1000000 1500]
[2 ^ 8, 10 mod 3, √16, 0x1F, 1_000_000, 1.5e3]
```

`0.1 + 0.2` is `0.3`. `as int` converts, `round` and `abs` are words:

```wasp => [3 2 7]
[(22 / 7) as int, round 2.5, abs -7]
```

Examples: constants; samples: factorial, power, gcd, collatz, quadratic

## Text

Texts are written in double quotes and joined with `+`. `\(…)` puts a value into a text (`${…}` works too).

```wasp => "Hi Ada!"
name = "Ada"
"Hi \(name)" + "!"
```

```wasp => [5 'h' "HELLO"]
["hello".length, "hello"#1, upper "hello"]
```

A text of one character is a codepoint, printed in single quotes. `split` and `join` take a text apart and back:

```wasp => "a, b, c"
join "a b c".split(" ") ", "
```

Examples: arguments; samples: levenshtein, json_parser

## Truth

`true` and `false` are the numbers 1 and 0, so every comparison is a number too.

```wasp => [1 1 0]
[3 > 2, "a" < "b", 2 == 3]
```

`and`, `or` and `not` combine them.

```wasp => 1
3 > 2 and not 2 > 3
```

## Variables

`=` names a value, `+=` and friends change it. `:` adds a type.

```wasp => 4
x = 3
x += 1
x
```

```wasp => 3
x : int = 3
```

Constants such as `pi` cannot be changed: `pi = 3` is an error.

Examples: constants

## Lists

A list is written in square brackets, its items separated by spaces or commas. `#` counts, and `xs#1` is the first
item, counting from 1 (`xs[0]` counts from 0, `#` is preferred).

```wasp => [3 10 30]
xs = [10 20 30]
[#xs, xs#1, xs#3]
```

```wasp => [6 3 1]
xs = [3 1 2]
[sum xs, max xs, min xs]
```

```wasp => [1 2 3 4]
xs = [1 2]
xs += [3 4]
xs
```

A range `1..5` stops before 5, `1 to 5` includes it. Comprehensions and `where` build new lists:

```wasp => [1 4 9 16]
[x * x for x in 1..5]
```

```wasp => [3 4]
[1 2 3 4] where it > 2
```

```wasp => 4950
sum 1..100
```

Examples: lists, "lazy ranges", "linear arrays"; samples: sorting, quicksort, primes, sieve

## Data

Objects are written in curly braces with `key: value` pairs. A dot or brackets read a field, and fields change like
variables.

```wasp => 37
person = {name: "Ada" age: 36}
person.age += 1
person.age
```

```wasp => "Ada"
person = {name: "Ada" age: 36}
person["name"]
```

Objects nest, and the path reads straight through:

```wasp => "London"
person = {name: "Ada" address: {city: "London"}}
person.address.city
```

Examples: data, welcome; samples: data_structures, json_parser

## Conditions

`if … then … else` is an expression with a value; `x ? a : b` is a short form, though `if` reads better.

```wasp => "big"
x = 3
if x > 2 then "big" else "small"
```

```wasp => "small"
x = 1
if x > 2 then "big" else "small"
```

Blocks in braces hold several lines:

```wasp => 10
x = 5
if x > 2 { x = x * 2 } else { x = 0 }
x
```

Examples: ambiguity; samples: control_flow, fizzbuzz

## Loops

`for` walks a range or a list, `while` repeats while its condition holds.

```wasp => 10
n = 0
for i in 1 to 4 { n += i }
n
```

```wasp => 8
n = 1
while n < 5 { n = n * 2 }
n
```

Examples: "game of life"; samples: fizzbuzz, sieve, life, mandelbrot

## Functions

`def` defines a function. A definition with `:=` takes its argument as `it`, and `=>` writes a function as a value.

```wasp => 9
def square(x) = x * x
square(3)
```

```wasp => 16
square := it * it
square 4
```

```wasp => 14
twice = x => x * 2
twice(7)
```

Functions call themselves:

```wasp => 55
def fib(n) = if n < 2 then n else fib(n - 1) + fib(n - 2)
fib(10)
```

Examples: functions, arguments, "polyglot calls"; samples: functions, fibonacci, ackermann, factorial

## Broadcasting

A function of one number applied to a list applies to each item.

```wasp => [1 4 9]
square := it * it
square [1 2 3]
```

```wasp => [2 4 6]
[1 2 3].map(x => x * 2)
```

Examples: broadcasting, welcome

## Closures

A function keeps the variables it was made with.

```wasp => 7
def adder(k) = x => x + k
add3 = adder(3)
add3(4)
```

```wasp => 2
def counter() { n = 0; () => { n += 1; n } }
next = counter()
next(); next()
```

Examples: closures

## Classes and types

`type` tells the type of a value, `is` checks it and `as` converts.

```wasp => [1 3 43]
[3 is int, "3" as int, int("42") + 1]
```

A class names the fields its objects have; its functions see them.

```wasp => 3
class Point { x: int y: int; def sum() = x + y }
Point{x: 1 y: 2}.sum()
```

Examples: classes, properties; samples: types, polymorphism

## Errors

An error is a value: it says what went wrong and where. `raise` makes one.

```wasp => Error("divide by zero")
1 / 0
```

```wasp => Error("boom")
raise "boom"
```

When code could mean two things, warp warns and says which one it took; the page offers the fix.

Examples: "welcoming errors", ambiguity

## Modules

`use` brings a module in. `sqrt 2` stays the exact `√2`; with `use math` it is a decimal number.

```wasp => 1.4142135623730951
use math
sqrt 2
```

Examples: "C libraries"; samples: lib

## Signals

A variable that a definition with `:=` reads is a signal: the definition follows it. `whenever` runs a block each
time its condition becomes true.

```wasp => 8
x = 3
twice := x * 2
x = 4
twice
```

Examples: signals, "signals in lists", welcome

## Events and timers

`on` listens to an event, `emit` sends one; timers run code later or repeatedly, and the page's mouse and keys
are events too.

Examples: "emit and on", listeners, events, timers, animation, mouse, "on exit", "system values"

## Tasks and channels

Tasks run at the same time; a channel passes values between them.

Examples: channels; samples: async, threads

## Markup and the web

Markup is wasp data: tags with braces hold their content, and a value made of tags shows as a page. Attributes are
written as in HTML, `name="value"`, and styles as in CSS.

```wasp => p{class:"note" "hello"}
p{ class="note" "hello" }
```

`on click { … }` inside an element handles its clicks; after a handler only what changed changes on the page.
A function that returns markup is a component. Inputs bind to variables, routes map addresses to pages, and
`local["key"]` and `session["key"]` keep values in the browser; `clipboard.write(text)` copies.

Examples: markup, "element events", "fine updates", components, cleanup, "keyed list", "form binding", styles, routes, transitions; samples: html, html_dsl

## WebAssembly and foreign code

warp compiles to WebAssembly GC. Modules written in other languages, WebAssembly components and C libraries are
called like wasp functions.

Examples: "wasm components", "C libraries", "polyglot calls"; samples: wasm_interop

## Laws

A law states what must always hold; warp checks it.

```wasp => 9
square(x) := x * x
law square(-x) == square(x)
square(3)
```

Examples: "kitchen sink"; samples: laws, kitchensink
