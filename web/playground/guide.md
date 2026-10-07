# The wasp guide

Learn wasp one small step at a time. Press **try ▶** to run a snippet, then change it.

## Hello, world

The last line of a program is its result.

```wasp => "Hello, world"
"Hello, world"
```

`print` writes a line along the way.

```wasp => 42
print "Hello"
42
```
```printed
Hello
```

`//` starts a comment. `/* … */` can go anywhere.

```wasp => 3
// this line is ignored
1 + /* so is this */ 2
```

Examples: hello

## Numbers

Whole numbers never overflow.

```wasp => 1267650600228229401496703205376
2 ^ 100
```

Division keeps fractions exact.

```wasp => 0.5
1/3 + 1/6
```

Numbers can carry units.

```wasp => 3200 m
3 km + 200 m
```

`mod` gives the remainder, `√` the square root.

```wasp => [1 4]
[10 mod 3, √16]
```

Write big numbers with `_`, hex with `0x`.

```wasp => [1000000 31]
[1_000_000, 0x1F]
```

Examples: numbers; samples: factorial, power, gcd, collatz, quadratic

## Text

Text goes in double quotes. `+` joins texts.

```wasp => "Hello, world"
"Hello, " + "world"
```

`\(…)` puts a value into a text.

```wasp => "Hi Ada!"
name = "Ada"
"Hi \(name)!"
```

Texts know their length.

```wasp => 5
"hello".length
```

`upper` makes capitals.

```wasp => "HELLO"
upper "hello"
```

Examples: text; samples: levenshtein

## Truth

A comparison is true (1) or false (0).

```wasp => 1
3 > 2
```

Combine them with `and`, `or` and `not`.

```wasp => 1
3 > 2 and not 2 > 3
```

## Variables

`=` gives a value a name.

```wasp => 4
x = 3
x + 1
```

`+=` changes it.

```wasp => 4
x = 3
x += 1
x
```

A type can follow the name.

```wasp => 3
x : int = 3
```

Constants like `pi` never change.

Examples: constants

## Lists

A list goes in square brackets.

```wasp => [1 2 3]
[1 2 3]
```

`#` counts the items.

```wasp => 3
#[10 20 30]
```

`xs#1` is the first item. Counting starts at 1.

```wasp => 10
xs = [10 20 30]
xs#1
```

`sum`, `max` and `min` work on lists.

```wasp => 6
sum [3 1 2]
```

`+=` adds items.

```wasp => [1 2 3 4]
xs = [1 2]
xs += [3 4]
xs
```

`1 to 5` counts to 5. `1..5` stops before 5.

```wasp => 15
sum 1 to 5
```

Build a list from another one.

```wasp => [1 4 9 16]
[x * x for x in 1..5]
```

Keep only some items with `where`.

```wasp => [3 4]
[1 2 3 4] where it > 2
```

Examples: lists, "lazy ranges", "linear arrays"; samples: sorting, quicksort, primes, sieve

## Objects

An object holds named values in curly braces.

```wasp => 36
person = {name: "Ada" age: 36}
person.age
```

Change a value like a variable.

```wasp => 37
person = {name: "Ada" age: 36}
person.age += 1
person.age
```

Objects can hold objects.

```wasp => "London"
person = {name: "Ada" home: {city: "London"}}
person.home.city
```

Examples: data; samples: data_structures, json_parser

## Conditions

`if … then … else` gives a value.

```wasp => "big"
x = 3
if x > 2 then "big" else "small"
```

Use braces for several lines.

```wasp => 10
x = 5
if x > 2 { x = x * 2 } else { x = 0 }
x
```

Examples: conditions; samples: control_flow, fizzbuzz

## Loops

`for` repeats for each number or item.

```wasp => 10
n = 0
for i in 1 to 4 { n += i }
n
```

`while` repeats while something is true.

```wasp => 8
n = 1
while n < 5 { n = n * 2 }
n
```

Examples: loops; samples: fizzbuzz, sieve, life, mandelbrot

## Functions

`def` makes a function.

```wasp => 9
def square(x) = x * x
square(3)
```

Parentheses are optional.

```wasp => 9
def square(x) = x * x
square 3
```

`:=` is a short form. The argument is called `it`.

```wasp => 16
square := it * it
square 4
```

A function can also be a value.

```wasp => 14
twice = x => x * 2
twice(7)
```

A function can call itself.

```wasp => 55
def fib(n) = if n < 2 then n else fib(n - 1) + fib(n - 2)
fib(10)
```

Examples: functions, arguments, "polyglot calls"; samples: functions, fibonacci, ackermann

## Whole lists at once

A function on one number also works on a list.

```wasp => [1 4 9]
square := it * it
square [1 2 3]
```

`map` does the same with any function.

```wasp => [2 4 6]
[1 2 3].map(x => x * 2)
```

Examples: broadcasting

## Closures

A function remembers the values around it.

```wasp => 7
def adder(k) = x => x + k
add3 = adder(3)
add3(4)
```

It can also change them.

```wasp => 2
def counter() { n = 0; () => { n += 1; n } }
next = counter()
next(); next()
```

Examples: closures

## Types and classes

`is` checks a type.

```wasp => 1
3 is int
```

`as` converts.

```wasp => 3
"3" as int
```

A class describes objects with fields and methods.

```wasp => 3
class Point { x: int y: int; def sum() = x + y }
Point{x: 1 y: 2}.sum()
```

Examples: classes, properties; samples: types, polymorphism

## Errors

An error is a value that says what went wrong.

```wasp => Error("divide by zero")
1 / 0
```

`raise` makes your own error.

```wasp => Error("boom")
raise "boom"
```

When a line could mean two things, warp asks which one you meant.

Examples: "welcoming errors", ambiguity

## Modules

`use` loads a module. With `use math`, roots become decimals.

```wasp => 1.4142135623730951
use math
sqrt 2
```

Examples: "C libraries"; samples: lib

## Values that follow

With `:=`, a value follows the values it reads.

```wasp => 8
x = 3
twice := x * 2
x = 4
twice
```

`whenever` runs code each time something becomes true.

Examples: "derived values", signals, listeners, "signals in lists"

## Events and timers

`on` handles an event. `emit` sends one. Timers run code later or again and again.

Examples: "emit and on", events, timers, animation, mouse, "game of life", "system values", "on exit"

## Tasks and channels

Tasks run at the same time. Channels carry messages between programs.

Examples: channels; samples: async, threads

## Web pages

Tags with braces make a page. Attributes look like HTML.

```wasp => p{class:"note" "hello"}
p{ class="note" "hello" }
```

`on click { … }` inside a tag handles clicks. A function that returns tags is a component.
`local["key"]` and `session["key"]` keep values in the browser. `clipboard.write(text)` copies text.

Examples: markup, "element events", "fine updates", components, cleanup, "keyed list", "form binding", styles, routes, transitions; samples: html, html_dsl

## WebAssembly

warp compiles to WebAssembly. Libraries in Rust or C can be called like wasp functions.

Examples: "wasm components"; samples: wasm_interop

## Laws

A law states something that must always hold. warp checks it.

```wasp => 9
square(x) := x * x
law square(-x) == square(x)
square(3)
```

Examples: "kitchen sink"; samples: laws, kitchensink
