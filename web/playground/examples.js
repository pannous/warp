// The tour, from basics to wow: each example's first line says what it shows; `value` is what the page shows for it,
// `printed` what it prints (test_in_browser.py --examples checks both in the page, so a broken example fails CI;
// `wait`: milliseconds of timers and listeners before checking). samples.js (made by build.sh) adds samples/*.wasp.
const EXAMPLES = {
	welcome: { value: "42", printed: "Hello 🌍\n", code: `// edit me: the value of the last line is the result
greeting = "Hello " + "🌍"
print greeting
6*7` },
	data: { value: '"hiking"', code: `// wasp is a data notation first: this object is a value, and code reads it
alice = { name: "Alice" age: 30 hobbies: ["reading" "hiking" "coding"] }
alice.hobbies#2` },
	functions: { value: "25", code: `// a function, called with or without parentheses
def square(x) = x*x
square(4) + square 3` },
	lists: { value: "9", code: `// # counts or indexes from 1
xs = [3 1 2]
xs#1 + #xs + max(xs)` },
	broadcasting: { value: "[[1 4 9] [1 4 9 16]]", code: `// a function of one number applies to each item of a list: no map needed
square := it * it
xs = [1 2 3 4]
[square [1 2 3], square all xs]` },
	arguments: { value: '["Hello, Ada" "Hi, Bob" 6 12 6]', code: `// defaults, keyword arguments, variadic parameters and overloads by arity
def greet(name, greeting = "Hello") = greeting + ", " + name
def total(xs...) = sum xs
def area(r) = r * r * 3
def area(w, h) = w * h
[greet("Ada"), greet(greeting: "Hi", name: "Bob"), total(1, 2, 3), area(2), area(2, 3)]` },
	"polyglot calls": { value: '[3 42 [10 20 30] [11 12 13] "Hello Swift"]', code: `// write functions the way you know them: Rust, Ruby, Swift, Julia, Kotlin
add = |a, b| a + b
twice = { |x| x * 2 }
func greet(person name: String) -> String { "Hello " + name }
fun Int.plus(n: Int) = this + n
[add(1, 2), twice(21), [1 2 3].map { $0 * 10 }, plus.([1 2 3], 10), greet(person: "Swift")]` },
	closures: { value: "[3 [1 2 3 5 8]]", code: `// a closure shares the variables it changes; sorted takes a key or a comparator
n = 0
tick = () => { n += 1; n }
tick(); tick()
[tick(), sorted([8 3 1 5 2], (a, b) => a < b)]` },
	classes: { value: '"Rex makes a sound: woof"', code: `// classes are fast GC structs: methods, inheritance, super
class animal { name; speak() := name + " makes a sound" }
class dog extends animal { speak() := super.speak() + ": woof" }
dog("Rex").speak()` },
	properties: { value: "[100 51]", code: `// properties with setters, generic classes and extension methods
class Temperature { celsius = 0.0; fahrenheit:{celsius * 9 / 5 + 32} set{celsius = (it - 32) * 5 / 9} }
water = Temperature()
water.fahrenheit = 212
class Box<T>{item:T}
extension Int { func squared() -> Int { self * self } }
[water.celsius, Box<int>(42).item + 3.squared()]` },
	"lazy ranges": { value: "[999999999999 499999999999500000000000 123456789]", code: `// a range is two numbers, not a list: a trillion items, counted and summed at once
n = 10^12
r = 1..n
[count r, sum r, r#123456789]` },
	"linear arrays": { value: "[1 4 9 16 25]", code: `// numbers in linear memory, one block for the host or a GPU (the hint: usually the compiler picks)
linear xs = int[5]
for i in 1 to 5 { xs#i = i * i }
xs` },
	signals: { value: "15", printed: "big order: 15\ntotal is now 15\n", code: `// every variable is a signal once something listens to it; := derives one
price = 3
count = 2
total := price * count
whenever total > 10 { print "big order: " + total }
on change total { print "total is now " + value }
count = 5
total` },
	"emit and on": { value: '"still running"', printed: "cooling down from 97\n", code: `// events carry data: emit one (or send it), handle it with on; raise is for errors
on overheat { print "cooling down from " + event.degrees }
emit overheat{degrees: 97}
"still running"` },
	listeners: { value: "1", printed: "hot: 35\nt = 35\nt = 40\n", code: `// listeners are values: name one, list them, remove it
t = 20
alarm = whenever t > 30 { print "hot: " + value }
on change t { print "t = " + value }
t = 35
remove alarm from listeners of t
t = 40
count listeners of t` },
	"signals in lists": { value: "30", printed: "changed to 10\nchanged to 20\n", code: `// a function can subscribe to each signal of a list
watch_all(xs) := for s in xs { on change s { print "changed to " + value } }
a = 1; b = 2
watch_all([a, b])
a = 10; b = 20
a + b` },
	timers: { value: "3", wait: 3500, printed: "tick 1\ntick 2\ntick 3\n", code: `// timers keep the program alive: the value below updates with every tick
ticks = 0
on every 1 second { ticks += 1; if ticks <= 3 { print "tick " + ticks } }
at 9:00 { print "good morning" }
ticks` },
	"game of life": { value: "15", wait: 1200, code: `// Conway's Game of Life, live: a step every half second on the canvas; three gliders keep 15 cells alive
width = 24
height = 12
global cells = []
for i in 1 to width * height { cells.push(0) }
for (x, y) in [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)] {
  for corner in [0, 8, 16] { cells#((y + corner % 12) * width + x + corner + 1) = 1 }
}
alive(x, y) := cells#(((y + height) % height) * width + (x + width) % width + 1)
step() := {
  next = []
  for y in 0 to height - 1 {
    for x in 0 to width - 1 {
      n = alive(x-1, y-1) + alive(x, y-1) + alive(x+1, y-1) + alive(x-1, y) + alive(x+1, y) + alive(x-1, y+1) + alive(x, y+1) + alive(x+1, y+1)
      next.push(if n == 3 or (n == 2 and alive(x, y) == 1) then 1 else 0)
    }
  }
  cells = next
}
population = 15
paint(cells, width, height)
on every 500 ms { step(); population = sum(cells); paint(cells, width, height) }
population` },
	"system values": { value: '"online"', code: `// system values are signals too: switch your system to dark mode and back
on change dark mode { if value { print "dark mode on" } else { print "light mode on" } }
if online then "online" else "offline"` },
	channels: { value: '"sent"', wait: 500, printed: "got hi\n", code: `// send to a channel, every program listening there receives it: open a second tab
on message from "chat" { print "got " + event.text }
send {text: "hi"} to "chat"
"sent"` },
	"on exit": { value: "42", printed: "working\nbye\n", code: `// on exit runs once the program is done
on exit { print "bye" }
print "working"
42` },
	events: { value: "0", code: `// click the result (or type there): on click / on key run in the page, the last line shows the new total
count = 0
price = 3
total := price * count
on click { count += 1; print "click at " + event.x + "," + event.y }
on key { price = 10 }
total` },
	"wasm components": { value: '[12586269025 ["a" "bc" "d"] 7]', code: `// a Rust crate compiled to a WebAssembly component, called like wasp: numbers, lists, objects
use wasm "tests/fixtures/components/rust_demo.wasm" as rust
c = rust.counter(5)
c.increment(2)
[rust.fib(50), rust.words("a bc d"), c.value()]` },
	"C libraries": { value: '["stack" "/b/c" 3400449319 "1.3.2"]', code: `// C runs here too: wasi-libc's string functions, and zlib compiled to WebAssembly from its own sources
use c
import tests/fixtures/wasm/zlib
[strstr("haystack", "st"), strchr("a/b/c", 47), crc32(0, "wasp", 4), zlibVersion()]` },
	ambiguity: { value: "6", code: `// an ambiguity is a warning naming the explicit form; "got it" silences it, the value stays
x=0
for i in 1 upto 4 { x += i }
x` },
	"welcoming errors": { value: 'Error("say 3 == 3 is ambiguous; write (say 3) == 3 or say(3 == 3) at 3:5")', code: `// errors explain and offer the fix: click "I meant" (square 3 == 9 is clear: square needs a number)
say(x) := x
say 3 == 3` },
	constants: { value: 'Error("pi is a constant at 2:1; fix: another name")', code: `// constants stay constant
pi = 4
2 * pi` },
	// samples/kitchensink.wasp itself (samples.js loads after this file), tested natively by test_kitchensink
	"kitchen sink": { value: '"all 31 checks pass"', get code() { return SAMPLES.kitchensink; } },
};
