# The warp guide

Learn Warp one small step at a time. Press **try ▶** to run a snippet, then change it.

## Hello, world

A program tells the computer what to do. The simplest program is just a value.

```warp => "Hello, 🌍"
"Hello, \:world"
```

`print` shows a line of text.

```warp
print "Hi"
```
```printed
Hi
```

A program can have many lines. The last one is the result.

```warp => 3
print "Hi"
3
```
```printed
Hi
```

A comment is a note for people. The computer skips it.

```warp => 3
3 // a note
```

Examples: hello; samples: comments, sample

## Numbers

warp works like a calculator.

```warp => 3
1 + 2
```

Parentheses go first.

```warp => 8
2 * (3 + 1)
```

`/` divides.

```warp => 2.5
10 / 4
```

`^` raises to a power. Numbers never get too big.

```warp => 1267650600228229401496703205376
2 ^ 100
```

Fractions stay exact.

```warp => 0.5
1/3 + 1/6
```

`mod` gives the remainder.

```warp => 1
7 mod 2
```

Numbers can have units.

```warp => 150min
2 h + 30 min
```

Examples: numbers; samples: factorial, power, gcd, collatz, quadratic, sum, sin, sine

## Text

Text is anything in quotes: a word, a name, a sentence.

```warp => "hello"
"hello"
```

`+` joins texts.

```warp => "abcd"
"ab" + "cd"
```

`.length` counts the letters.

```warp => 4
"warp".length
```

`$(…)` puts a value inside a text.

```warp => "1 + 1 = 2"
"1 + 1 = $(1 + 1)"
```

Examples: text; samples: levenshtein, palindrome, word_count, sort_words, word_lengths, replace_word, find_text, levenshtein_idiomatic, calculator

## Variables

A variable is a name for a value, so you can use it again.

```warp => 6
x = 2 * 3
x
```

Use the name in a calculation.

```warp => 4
x = 3
x + 1
```

Give it a new value any time.

```warp => 2
x = 1
x = 2
x
```

`+=` adds to it.

```warp => 4
x = 3
x += 1
x
```

Some names, like `pi`, are constants. They never change.

Examples: constants

## Truth

Some questions have a yes-or-no answer.

```warp => yes
3 > 2
```

`==` asks "equal?".

```warp => no
2 == 3
```

### Logic

`and` is yes only if both are yes.

```warp => no
yes and no
```

`not` turns yes into no.

```warp => no
not yes
```

## Conditions

A condition lets the program choose what to do.

`if` runs code only when something is true.

```warp
if 3 > 2 : print "yes"
```
```printed
yes
```

`else` says what to do otherwise.

```warp => "no"
if 1 > 2 : "yes" else "no"
```

Check several things one after another.

```warp => "medium"
x = 5
if x > 9 : "big" else if x > 3 : "medium" else "small"
```

`time` is the time of day, so the answer depends on when you run it.

```warp
if time < 12h : "morning" else "later"
```

Braces hold several lines.

```warp
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

```warp => [1 2 3]
[1 2 3]
```

`#` counts the items.

```warp => 2
friends = [Alf, Bob]
#friends
```

`#1` is the first item.

```warp => "apple"
["apple" "pear"]#1
```

`sum` adds them up.

```warp => 6
sum 1 2 3
```

`sort` puts them in order.

```warp => [1 2 3]
sort [3 1 2]
```

`+=` adds items.

```warp => [1 2 3]
xs = [1 2]
xs += [3]
xs
```

Make a new list from another one.

```warp => [1 4 9 16]
[x² for x in [1 2 3 4]]
```

With a function, `square [1 2 3 4]` does the same: see "Whole lists at once" below.

`where` keeps only some items.

```warp => [3 4]
[1 2 3 4] where it > 2
```

### Whole lists at once

A function for one number also works on a whole list.

```warp => [1 4 9]
square := it * it
square [1 2 3]
```

`map` spells the same out, handy for a function written in place.

```warp => [2 4 6]
[1 2 3].map(x => x * 2)
```

Examples: lists, broadcasting, "lazy ranges", "linear arrays"; samples: sorting, quicksort, primes, sieve, sorting_idiomatic, sieve_idiomatic

## Objects

An object keeps values that belong together, each with a name, like a contact card.

```warp => {name:"Ada" age:36}
{name: "Ada" age: 36}
```

A dot reads one value.

```warp => "Ada"
ada = {name: "Ada"}
ada.name
```

Change it like a variable.

```warp => 37
ada = {age: 36}
ada.age += 1
ada.age
```

Examples: data; samples: data_structures, json_parser, binary_tree, dijkstra, dijkstra_idiomatic

## Loops

A loop repeats work, so you write it only once.

`for` runs once for each number.

```warp
for i in 1 to 3 { print i }
```
```printed
1
2
3
```

Or once for each item of a list.

```warp
for fruit in ["apple" "pear"] { print fruit }
```
```printed
apple
pear
```

`while` repeats as long as something is true.

```warp => 3
n = 0
while n < 3 { n += 1 }
n
```

Examples: loops; samples: fizzbuzz, sieve, life, mandelbrot, game_of_life, life_idiomatic

## Functions

A function is a recipe with a name: write it once, use it often.

```warp => 9
def square(x) = x * x
square(3)
```

Parentheses are optional

```warp => 9
def square(x) = x * x
square 3
```

A function can take several inputs.

```warp => 5
def add(a, b) = a + b
add(2, 3)
```

`:=` is a shortcut. The input is called `it`.

```warp => 42
twice := it * 2
twice 21
```

A function can call itself.

```warp => 55
def fib(n) = if n < 2 then n else fib(n - 1) + fib(n - 2)
fib(10)
```

Examples: functions, arguments, "polyglot calls"; samples: functions, fibonacci, ackermann, queens, queens_idiomatic, sudoku

## Closures

Closures are anonymous functions, tiny ad-hoc pieces of code without a name, very useful in the comparisons:
```warp
sort([8 3 1 5 2], {$0 < $1})
```

(Much shorter than `def compare_elements(a,b){return a<b}`)

A closure is a function that remembers the values around it.

```warp => 7
def adder(k) = x => x + k
add3 = adder(3)
add3(4)
```

Examples: closures

## Types and classes

Every value has a type: a number, a text, a list...

```warp => int
type(3)
```

`is` checks the type.

```warp => yes
"3" is text
```

`as` converts to another type.

```warp => 3
"3" as int
```

A class is a blueprint for objects.

```warp => 1
class Point { x: int y: int }
Point{x: 1 y: 2}.x
```

Examples: classes, properties; samples: types, polymorphism

## Errors

Things go wrong sometimes. In warp an error is a value that says what happened.

```warp => Error("divide by zero")
1 / 0
```

`raise` makes your own error.

```warp => Error("boom")
raise "boom"
```

When a line could mean two things, warp asks which one you meant.

Examples: "welcoming errors", ambiguity; samples: try_catch, try_else, parse_number, raise_error, assertions

## Modules

A module is a ready-made toolbox. `use` opens it.

```warp => 1.4142135623730951
use math
sqrt 2
```

Examples: "C libraries"; samples: lib, square, main, test_ffi, test_ffi_comprehensive, test_ffi_extended

## Values that follow

Like a spreadsheet cell, a value can follow other values.

`=` copies a value once.

```warp => 6
x = 3
y = x * 2
x = 4
y
```

`:=` keeps following.

```warp => 8
x = 3
y := x * 2
x = 4
y
```

`whenever` runs code each time something becomes true.

```warp
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

```warp
on ping : print "pong"
emit ping
```
```printed
pong
```

Examples: "emit and on", events, timers, animation, mouse, "game of life", "system values", "on exit"

## Tasks and channels

Tasks do several things at the same time. 
```warp
go {
	sleep(100 ms)
	print "I'm late;)"
}
print("first!")
```

Channels pass messages between programs.

Examples: tasks, channels; samples: async, threads

## Web pages

A web page is made of tags. In warp, tags are values.

```warp => p:"hello"
p{ "hello" }
```

Attributes look like HTML.

```warp => b{class:"fat" "hello"}
b{class:"fat" "hello" }
```

Tags hold other tags, like a page does.

```warp => div{h1:"Fruit" p{"Fresh " b:"pears" " today"}}
div{ h1{"Fruit"} p{ "Fresh " b{"pears"} " today" } }
```

A list of values makes a list of tags.

```warp => ul{li:"apple" li:"pear"}
fruits = ["apple" "pear"]
ul{ li all fruits }
```

`on click { … }` inside a button makes it do something. `local["key"]` remembers a value in the browser.

Examples: markup, "element events", "fine updates", components, cleanup, "keyed list", "form binding", styles, routes, transitions; samples: html, html_dsl

## WebAssembly

warp turns warp into WebAssembly, which runs in every browser. Code from Rust or C can be called alike.

Examples: "wasm components"; samples: wasm_interop

## Laws

A law is a rule that must always hold. warp checks it for you (only if [Lean](https://github.com/leanprover/lean4) is installed.)

```warp => 9
square(x) := x * x
law square(-x) == square(x)
square(3)
```

This way, you can prove that your program always does the correct thing. 

Examples: "kitchen sink"; samples: laws, kitchensink


## Advanced

Bigger programs that put the chapters together: a neural net, a ray tracer, a game, pictures painted on the canvas.

Examples: "kitchen sink"; samples: neural_net, raytracer, snake, particles, circle, filled_circle, mandelbrot_canvas, test, simple
