# The wasp guide

Learn wasp one small step at a time. Press **try ▶** to run a snippet, then change it.

## Hello, world

A program tells the computer what to do. The simplest program is just a value.

```wasp => "Hello, \:world"
"Hello 🌍"
```

`print` shows a line of text.

```wasp
print "Hi"
```
```printed
Hi
```

A program can have many lines. The last one is the result.

```wasp => 3
print "Hi"
3
```
```printed
Hi
```

A comment is a note for people. The computer skips it.

```wasp => 3
3 // a note
```

Examples: hello

## Numbers

wasp works like a calculator.

```wasp => 3
1 + 2
```

Parentheses go first.

```wasp => 8
2 * (3 + 1)
```

`/` divides.

```wasp => 2.5
10 / 4
```

`^` raises to a power. Numbers never get too big.

```wasp => 1267650600228229401496703205376
2 ^ 100
```

Fractions stay exact.

```wasp => 0.5
1/3 + 1/6
```

`mod` gives the remainder.

```wasp => 1
7 mod 2
```

Numbers can have units.

```wasp => 150 min
2 h + 30 min
```

Examples: numbers; samples: factorial, power, gcd, collatz, quadratic

## Text

Text is anything in quotes: a word, a name, a sentence.

```wasp => "hello"
"hello"
```

`+` joins texts.

```wasp => "abcd"
"ab" + "cd"
```

`.length` counts the letters.

```wasp => 4
"wasp".length
```

`$(…)` puts a value inside a text.

```wasp => "1 + 1 = 2"
"1 + 1 = $(1 + 1)"
```

Examples: text; samples: levenshtein TODO: many more! sort!

## Variables

A variable is a name for a value, so you can use it again.

```wasp => 6
x = 2 * 3
x
```

Use the name in a calculation.

```wasp => 4
x = 3
x + 1
```

Give it a new value any time.

```wasp => 2
x = 1
x = 2
x
```

`+=` adds to it.

```wasp => 4
x = 3
x += 1
x
```

Some names, like `pi`, are constants. They never change.

Examples: constants

## Truth

Some questions have a yes-or-no answer.

```wasp => yes
3 > 2
```

`==` asks "equal?".

```wasp => no
2 == 3
```

### Logic

`and` is yes only if both are yes.

```wasp => no
yes and no
```

`not` turns yes into no.

```wasp => no
not yes
```

## Conditions

A condition lets the program choose what to do.

`if` runs code only when something is true.

```wasp
if 3 > 2 : print "yes"
```
```printed
yes
```

`else` says what to do otherwise.

```wasp => "no"
if 1 > 2 : "yes" else "no"
```

Check several things one after another.

```wasp => "medium"
x = 5
if x > 9 : "big" else if x > 3 : "medium" else "small"
```

Braces hold several lines.

```wasp
if 3 > 2 {
	print "yes"
	print "sure"
}
```
```printed
yes
sure
```

Examples: conditions; samples: control_flow, fizzbuzz

## Lists

A list keeps several values in order, like a shopping list.

```wasp => [1 2 3]
[1 2 3]
```

`#` counts the items.

```wasp => 2
friends = [Alf, Bob]
#friends
```

`#1` is the first item.

```wasp => "apple"
["apple" "pear"]#1
```

`sum` adds them up.

```wasp => 6
sum 1 2 3
```

`sort` puts them in order.

```wasp => [1 2 3]
sort [3 1 2]
```

`+=` adds items.

```wasp => [1 2 3]
xs = [1 2]
xs += [3]
xs
```

Make a new list from another one.

```wasp => [1 4 9 16]
[x² for x in [1 2 3 4]]
```

`where` keeps only some items.

```wasp => [3 4]
[1 2 3 4] where it > 2
```

Examples: lists, "lazy ranges", "linear arrays"; samples: sorting, quicksort, primes, sieve

## Objects

An object keeps values that belong together, each with a name, like a contact card.

```wasp => {name:"Ada" age:36}
{name: "Ada" age: 36}
```

A dot reads one value.

```wasp => "Ada"
ada = {name: "Ada"}
ada.name
```

Change it like a variable.

```wasp => 37
ada = {age: 36}
ada.age += 1
ada.age
```

Examples: data; samples: data_structures, json_parser

## Loops

A loop repeats work, so you write it only once.

`for` runs once for each number.

```wasp
for i in 1 to 3 { print i }
```
```printed
1
2
3
```

Or once for each item of a list.

```wasp
for fruit in ["apple" "pear"] { print fruit }
```
```printed
apple
pear
```

`while` repeats as long as something is true.

```wasp => 3
n = 0
while n < 3 { n += 1 }
n
```

Examples: loops; samples: fizzbuzz, sieve, life, mandelbrot

## Functions

A function is a recipe with a name: write it once, use it often.

```wasp => 9
def square(x) = x * x
square(3)
```

Parentheses are optional

```wasp => 9
def square(x) = x * x
square 3
```

A function can take several inputs.

```wasp => 5
def add(a, b) = a + b
add(2, 3)
```

`:=` is a shortcut. The input is called `it`.

```wasp => 42
twice := it * 2
twice 21
```

A function can call itself.

```wasp => 55
def fib(n) = if n < 2 then n else fib(n - 1) + fib(n - 2)
fib(10)
```

Examples: functions, arguments, "polyglot calls"; samples: functions, fibonacci, ackermann

## Whole lists at once

A function for one number also works on a whole list.

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

A closure is a function that remembers the values around it.

```wasp => 7
def adder(k) = x => x + k
add3 = adder(3)
add3(4)
```

Examples: closures

## Types and classes

Every value has a type: a number, a text, a list...

```wasp => int
type(3)
```

`is` checks the type.

```wasp => yes
"3" is text
```

`as` converts to another type.

```wasp => 3
"3" as int
```

A class is a blueprint for objects.

```wasp => 1
class Point { x: int y: int }
Point{x: 1 y: 2}.x
```

Examples: classes, properties; samples: types, polymorphism

## Errors

Things go wrong sometimes. In wasp an error is a value that says what happened.

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

A module is a ready-made toolbox. `use` opens it.

```wasp => 1.4142135623730951
use math
sqrt 2
```

Examples: "C libraries"; samples: lib

## Values that follow

Like a spreadsheet cell, a value can follow other values.

`=` copies a value once.

```wasp => 6
x = 3
y = x * 2
x = 4
y
```

`:=` keeps following.

```wasp => 8
x = 3
y := x * 2
x = 4
y
```

`whenever` runs code each time something becomes true.

```wasp
x = 1
whenever x > 2 : print "big"
x = 3
```
```printed
big
```

Examples: "derived values", signals, listeners, "signals in lists"

## Events and timers

An event is something that happens: a click, a key, a tick of a clock.

`on` says what to do when it happens. `emit` makes it happen.

```wasp
on ping : print "pong"
emit ping
```
```printed
pong
```

Examples: "emit and on", events, timers, animation, mouse, "game of life", "system values", "on exit"

## Tasks and channels

Tasks do several things at the same time. 
```wasp
go {
	sleep(100 ms)
	print "I'm late;)"
}
print("first!")
```

Channels pass messages between programs.

Examples: channels; samples: async, threads

## Web pages

A web page is made of tags. In wasp, tags are values.

```wasp => p:"hello"
p{ "hello" }
```

Attributes look like HTML.

```wasp => b{class:"fat" "hello" }
b{class:"fat" "hello" }
```

`on click { … }` inside a button makes it do something. `local["key"]` remembers a value in the browser.

Examples: markup, "element events", "fine updates", components, cleanup, "keyed list", "form binding", styles, routes, transitions; samples: html, html_dsl

## WebAssembly

warp turns wasp into WebAssembly, which runs in every browser. Code from Rust or C can be called alike.

Examples: "wasm components"; samples: wasm_interop

## Laws

A law is a rule that must always hold. warp checks it for you.

```wasp => 9
square(x) := x * x
law square(-x) == square(x)
square(3)
```

This way, you can prove that your program always does the correct thing. 

Examples: "kitchen sink"; samples: laws, kitchensink


## Advanced
