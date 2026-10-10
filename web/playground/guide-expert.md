# The warp guide for programmers

The same chapters in short: one line on what differs from the languages you know, one snippet. Press **try ▶** to run it.

## Hello, world

A program is a value: its last line is the result; `print` writes a line; `//` comments.

```warp => "Hello, 🌍"
print "Hi" // printed, not the value
"Hello, \:world"
```
```printed
Hi
```

## Numbers

Unbounded ints, exact fractions, `mod`, `^`; numbers carry units.

```warp => [1267650600228229401496703205376 0.5 1 2.5]
[2 ^ 100, 1/3 + 1/6, 7 mod 2, 10 / 4]
```

```warp => 150min
2h + 30min
```

## Text

`+` joins, `.length` counts, `\(…)` interpolates.

```warp => ["abcd" 4 "1 + 1 = 2"]
["ab" + "cd", "warp".length, "1 + 1 = \(1 + 1)"]
```

## Variables

`=` binds and rebinds, compound assignment works; constants like `pi` don't change.

```warp => 4
x = 3; x += 1; x
```

## Truth

Booleans are `yes` and `no`, with word operators `and`, `or`, `not`.

```warp => [yes no no no]
[3 > 2, 2 == 3, yes and no, not yes]
```

## Conditions

`if c : a else b` is an expression; braces hold several lines.

```warp => "medium"
x = 5; if x > 9 : "big" else if x > 3 : "medium" else "small"
```

## Lists

Space-separated, 1-based `#i`, `#xs` counts; comprehensions filter with `where`.

```warp => [4 3 10 [1 2 3 4]]
xs = [3 1 2]; xs += [4]; [#xs, xs#1, sum xs, sort xs]
```

```warp => [9 16]
[x² for x in [1 2 3 4] where x > 2]
```

### Whole lists at once

A function of a number broadcasts over a list; `map` takes a lambda `x => …`.

```warp => [[1 4 9] [2 4 6]]
square := it * it; [square [1 2 3], [1 2 3].map(x => x * 2)]
```

A heavy map of a linear float array runs on the GPU by itself from 10× its break-even (`GPU_AUTO_MIN_COUNT`), with a
one-time notice: f32, approximate by design, the error grows with the item count. `@cpu` (on the map, a block or a
function) keeps it exact in f64, `WARP_GPU=off` a whole run; `@gpu` opts in from the break-even.

## Objects

Literal objects with dot access and in-place updates.

```warp => {name:"Ada" age:37}
ada = {name: "Ada" age: 36}; ada.age += 1; ada
```

## Loops

`for … in` ranges and lists, `while`; a loop's value is its last body value.

```warp => 3
for i in 1 to 3 { print i }; n = 0; while n < 3 { n += 1 }; n
```
```printed
1
2
3
```

## Functions

`def f(x) = …`, calls without parentheses, `:=` with the implicit parameter `it`, recursion.

```warp => [55 42]
def fib(n) = if n < 2 then n else fib(n - 1) + fib(n - 2); twice := it * 2; [fib 10, twice 21]
```

## Closures

Lambdas capture their surroundings.

```warp => 7
def adder(k) = x => x + k; adder(3)(4)
```

## Types and classes

`type`, `is` tests, `as` converts; classes declare typed fields.

```warp => [int yes 3 1]
class Point { x: int y: int }; [type(3), "3" is text, "3" as int, Point{x: 1 y: 2}.x]
```

## Errors

Errors are values; `raise` makes one; ambiguous lines are questions, not guesses.

```warp => Error("divide by zero")
1 / 0
```

## Modules

`use` imports a module's words.

```warp => 1.4142135623730951
use math; sqrt 2
```

## Values that follow

`=` copies once, `:=` recomputes like a spreadsheet cell, `whenever` reacts.

```warp => [6 8]
x = 3; y = x * 2; z := x * 2; x = 4; [y, z]
```

## Events and timers

`on event : …` handles, `emit event` fires.

```warp
on ping : print "pong"; emit ping
```
```printed
pong
```

## Tasks and channels

`go { … }` runs a task concurrently; channels pass messages between tasks.

## Web pages

Tags are values: `tag{attributes children}`, `li all xs` repeats a tag over a list.

```warp => div{h1:"Fruit" ul{li:"apple" li:"pear"}}
fruits = ["apple" "pear"]; div{ h1{"Fruit"} ul{ li all fruits } }
```

## WebAssembly

Programs compile to WebAssembly GC; Rust and C code is called alike.

## Laws

`law` states a property that is proved with Lean when it is installed.

```warp => 9
square(x) := x * x; law square(-x) == square(x); square(3)
```

## Advanced

Whole programs combining the chapters: see the samples in the example menu.
