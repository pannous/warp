// The tour, in the order of the guide (guide.md), from the basics to the web: each example's first line says what it shows; `value` is what the page shows for it,
// `printed` what it prints (test_in_browser.py --examples checks both in the page, so a broken example fails CI;
// `wait`: milliseconds of timers and listeners before checking; `clicks`: the shown buttons, by text, clicked after
// that, `clicked` the value then and `kept` that every element shown stays: only its text and attributes change;
// `keyed` that every element with a key keeps it: a list item's element moves with its item; `typed`: the text typed
// into the first input before the clicks; `animated` that the clicks started an animation; `clickedPrinted`: all
// printed text after the clicks; `address` the address bar's path after
// them).
// samples.js (made by build.sh) adds samples/*.warp.
const EXAMPLES = {
	hello: { value: '"Hello, world"', code: `// your first program: the value of the last line shows below
"Hello, world"` },
	numbers: { value: "1267650600228229401496703205376", code: `// whole numbers never overflow
2 ^ 100` },
	text: { value: '"Hello, Ada!"', code: `// put a value into a text with \\(…)
name = "Ada"
"Hello, \\(name)!"` },
	constants: { value: 'Error("pi is a constant at 2:1; fix: another name")', code: `// constants such as pi cannot change
pi = 4
2 * pi` },
	lists: { value: '[3 "apple"]', code: `// a list, how many items it has, and its first item
fruits = ["apple" "pear" "plum"]
[#fruits, fruits#1]` },
	"lazy ranges": { value: "[999999999999 499999999999500000000000 123456789]", code: `// a range of a trillion numbers, counted and summed at once
n = 10^12
r = 1..n
[count r, sum r, r#123456789]` },
	"linear arrays": { value: "[1 4 9 16 25]", code: `// an array of numbers in one block of memory
linear xs = int[5]
for i in 1 to 5 { xs#i = i * i }
xs` },
	data: { value: "31", code: `// an object holds named values
alice = {name: "Alice" age: 30}
alice.age + 1` },
	conditions: { value: '"warm"', code: `// if … then … else gives a value
temperature = 25
if temperature > 20 then "warm" else "cold"` },
	loops: { value: "55", code: `// add up the numbers 1 to 10
total = 0
for i in 1 to 10 { total += i }
total` },
	functions: { value: "25", code: `// a function, called with or without parentheses
def square(x) = x*x
square(4) + square 3` },
	arguments: { value: '["Hello, Ada" "Hi, Bob" 6 12 6]', code: `// default values, named arguments, any number of arguments
def greet(name, greeting = "Hello") = greeting + ", " + name
def total(xs...) = sum xs
def area(r) = r * r * 3
def area(w, h) = w * h
[greet("Ada"), greet(greeting: "Hi", name: "Bob"), total(1, 2, 3), area(2), area(2, 3)]` },
	"polyglot calls": { value: '[3 42 [10 20 30] [11 12 13] "Hello Swift"]', code: `// write functions the way you know them from Rust, Swift, Kotlin…
add = |a, b| a + b
twice = { |x| x * 2 }
func greet(person name: String) -> String { "Hello " + name }
fun Int.plus(n: Int) = this + n
[add(1, 2), twice(21), [1 2 3].map { $0 * 10 }, plus.([1 2 3], 10), greet(person: "Swift")]` },
	broadcasting: { value: "[[1 4 9] [1 4 9 16]]", code: `// a function of one number works on a whole list
square := it * it
xs = [1 2 3 4]
[square [1 2 3], square all xs]` },
	closures: { value: "[3 [1 2 3 5 8]]", code: `// a function remembers the variables around it
n = 0
tick = () => { n += 1; n }
tick(); tick()
[tick(), sorted([8 3 1 5 2], (a, b) => a < b)]` },
	classes: { value: '"Rex makes a sound: woof"', code: `// classes with methods, inheritance and super
class animal { name; speak := name + " makes a sound" }
class dog extends animal { speak := super.speak + ": woof" }
dog("Rex").speak` },
	properties: { value: "[100 51]", code: `// computed properties, generic classes, extension methods
class Temperature { celsius = 0.0; fahrenheit:{celsius * 9 / 5 + 32} set{celsius = (it - 32) * 5 / 9} }
water = Temperature()
water.fahrenheit = 212
class Box<T>{item:T}
extension Int { func squared() -> Int { self * self } }
[water.celsius, Box<int>(42).item + 3.squared()]` },
	"welcoming errors": { value: 'Error("say 3 == 3 is ambiguous; write (say 3) == 3 or say(3 == 3) at 3:5")', code: `// an unclear line is an error that offers its readings
say(x) := x
say 3 == 3` },
	ambiguity: { value: "6", code: `// when code could mean two things, warp says which one it took
x=0
for i in 1 upto 4 { x += i }
x` },
	"derived values": { value: "15", code: `// := keeps a value up to date: total follows count
price = 3
count = 2
total := price * count
count = 5
total` },
	signals: { value: "15", printed: "big order: 15\ntotal is now 15\n", code: `// whenever and on change react to a value changing
price = 3
count = 2
total := price * count
whenever total > 10 { print "big order: " + total }
on change total { print "total is now " + value }
count = 5
total` },
	listeners: { value: "1", printed: "hot: 35\nt = 35\nt = 40\n", code: `// listeners are values: keep one, remove it later
t = 20
alarm = whenever t > 30 { print "hot: " + value }
on change t { print "t = " + value }
t = 35
remove alarm from listeners of t
t = 40
count listeners of t` },
	"signals in lists": { value: "30", printed: "changed to 10\nchanged to 20\n", code: `// a function can watch every value of a list
watch_all(xs) := for s in xs { on change s { print "changed to " + value } }
a = 1; b = 2
watch_all([a, b])
a = 10; b = 20
a + b` },
	"emit and on": { value: '"still running"', printed: "cooling down from 97\n", code: `// send your own event with emit, handle it with on
on overheat { print "cooling down from " + event.degrees }
emit overheat{degrees: 97}
"still running"` },
	events: { value: "0", code: `// click or type in the result area
count = 0
price = 3
total := price * count
on click { count += 1; print "click at " + event.x + "," + event.y }
on key { price = 10 }
total` },
	timers: { value: "3", wait: 3500, printed: "tick 1\ntick 2\ntick 3\n", code: `// a timer ticks every second
ticks = 0
on every 1 second { ticks += 1; if ticks <= 3 { print "tick " + ticks } }
at 9:00 { print "good morning" }
ticks` },
	animation: { value: '"done"', canvases: 1, code: `// an animation: each show() draws the next frame
use draw
canvas(32, 16)
for x in 0..32 {
  clear(paper)
  circle(x, 8, 4, orange)
  show()
  sleep(30)
}
"done"` },
	frames: { value: '"done"', canvases: 1, code: `// without a sleep too: each show() replaces the picture of the same size
use draw
canvas(32, 16)
for x in 0..32 {
  clear(paper)
  circle(x, 8, 4, purple)
  show()
}
"done"` },
	mouse: { value: '"done"', canvases: 1, code: `// move the mouse over the canvas: the dot follows
use draw
canvas(48, 24)
for frame in 0..200 {
  clear(paper)
  circle(mouse_x, mouse_y, 3, if mouse_down then red else blue)
  show()
  sleep(30)
}
"done"` },
	"game of life": { value: "15", wait: 1200, code: `// Conway's Game of Life, a step every half second
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
	"system values": { value: '"online"', code: `// system values like dark mode and online are signals too
on change dark mode { if value { print "dark mode on" } else { print "light mode on" } }
if online then "online" else "offline"` },
	"on exit": { value: "42", printed: "working\nbye\n", code: `// on exit runs when the program is done
on exit { print "bye" }
print "working"
42` },
	tasks: { value: '"done"', printed: "first!\nlate\n", code: `// a task runs beside the program, which waits for it at the end
go {
	sleep(100 ms)
	print "late"
}
print "first!"
"done"` },
	channels: { value: '"sent"', wait: 500, printed: "got hi\n", code: `// send a message to every tab listening on a channel
on message from "chat" { print "got " + event.text }
send {text: "hi"} to "chat"
"sent"` },
	markup: { value: 'div{h2:"Alice" p:"30 years, likes:" ul{li:"reading" li:"hiking"}}', code: `// markup is data too: a value made of HTML tags shows as a page
div{ h2{ "Alice" } p{ "30 years, likes:" } ul{ li{ "reading" } li{ "hiking" } } }` },
	"element events": { value: 'div{button{"+1" data-warp-click:"1"} button{"reset" data-warp-click:"2"} p:"count: 0"}', code: `// a button with a click handler: click it
count = 0
div{ button{ "+1" on click { count += 1 } } button{ "reset" on click { count = 0 } } p{ "count: " + count } }` },
	"fine updates": { value: 'div{style:.done:color:"green" button{data-warp-click:"1" "toggle"} p{class:"open" "state"} ul:li:"toggled 0 times"}', clicks: ["toggle", "toggle", "toggle"], clicked: 'div{style:.done:color:"green" button{data-warp-click:"1" "toggle"} p{class:"done" "state"} ul:li:"toggled 3 times"}', kept: true, code: `// after a click only what changed is redrawn
done = false
toggles = 0
div{ style{ .done = "green" } button{ on click { done = not done; toggles += 1 } "toggle" } p{ class: (if done then "done" else "open") "state" } ul{ li{ "toggled " + toggles + " times" } } }` },
	components: { value: 'div{div{button{data-warp-instance:1 data-warp-click:"1" \'+\'} p:"Apples: 1"} div{button{data-warp-instance:2 data-warp-click:"1" \'+\'} p:"Pears: 5"}}', clicks: ["+"], clicked: 'div{div{button{data-warp-instance:1 data-warp-click:"1" \'+\'} p:"Apples: 2"} div{button{data-warp-instance:2 data-warp-click:"1" \'+\'} p:"Pears: 5"}}', kept: true, code: `// a component is a function that returns markup; each keeps its own count
def Counter(label, start) {
	count = start
	div{ button{ on click { count += 1 } "+" } p{ label + ": " + count } }
}
div{ Counter("Apples", 1) Counter("Pears", 5) }` },
	cleanup: { value: 'div{button{data-warp-click:"1" "hide pears"} span:"apples " span:"pears "}', printed: "hello apples\nhello pears\n", clicks: ["hide pears"], clicked: 'div{button{data-warp-click:"1" "hide pears"} span:"apples "}', clickedPrinted: "hello apples\nhello pears\nbye pears\n", code: `// on mount and on cleanup run when a component appears and leaves
def Fruit(label) {
	name = label
	on mount { print "hello " + name }
	on cleanup { print "bye " + name }
	span{ name + " " }
}
pears = true
div{ button{ on click { pears = false } "hide pears" } Fruit("apples") (if pears then Fruit("pears") else []) }` },
	"keyed list": { value: 'div{button{data-warp-click:"1" "rotate"} ul{li{key:1 "milk"} li{key:2 "eggs"} li{key:3 "tea"}}}', clicks: ["rotate"], clicked: 'div{button{data-warp-click:"1" "rotate"} ul{li{key:2 "eggs"} li{key:3 "tea"} li{key:1 "milk"}}}', kept: true, keyed: true, code: `// with a key, a list item keeps its element when the list reorders
todos = [{id:1 text:"milk"} {id:2 text:"eggs"} {id:3 text:"tea"}]
div{ button{ on click { todos = todos[1..] + [todos#1] } "rotate" } ul{ [li{ key: todo.id todo.text } for todo in todos] } }` },
	"form binding": { value: 'div{label{"Name " input{value:"Ann" data-warp-input:"1"}} p:"Hello Ann"}', typed: "Bob", clicked: 'div{label{"Name " input{value:"Bob" data-warp-input:"1"}} p:"Hello Bob"}', kept: true, code: `// bind ties an input to a variable: type a name
name = "Ann"
div{ label{ "Name " input{ bind: name } } p{ "Hello " + name } }` },
	styles: { value: 'div{style:.card{padding:8 border:"1px solid gray"} button{data-warp-click:"1" "dark"} p{class:"card" style{color:"black" background:"white"} "a themed card"}}', clicks: ["dark"], clicked: 'div{style:.card{padding:8 border:"1px solid gray"} button{data-warp-click:"1" "dark"} p{class:"card" style{color:"white" background:"black"} "a themed card"}}', kept: true, code: `// styles are data too: a style sheet and inline styles
dark = false
div{
	style{ ".card": { padding: 8 border: "1px solid gray" } }
	button{ on click { dark = not dark } "dark" }
	p{ class: "card" style: { color: (if dark then "white" else "black") background: (if dark then "black" else "white") } "a themed card" }
}` },
	routes: { value: 'div{h1:"Users" a{href:"/users/2" "Bo"}}', clicks: ["Bo"], clicked: 'div{h1:"Bo" a{href:\'/\' "back"}}', address: "/users/2", code: `// each address of the page shows its own block: click the link
users = ["Ann", "Bo"]
route "/" { div{ h1{ "Users" } a{ href: "/users/2" "Bo" } } }
route "/users/:id:int" { div{ h1{ users#id } a{ href: "/" "back" } } }
route "*" { p{ "no such page" } }` },
	forms: { value: 'div{form{method:"post" action:"/items" input:name:"item" button:"add"} ul:li:"milk"}', typed: "tea", clicks: ["add"], clicked: 'div{form{method:"post" action:"/items" input:name:"item" button:"add"} ul{li:"milk" li:"tea"}}', address: "/", code: `// a form posts to the program's own route: here the page answers it, \`warp serve\` serves it to every browser
items = ["milk"]
post "/items" { items.add(request.body.item) }
route "/" {
	div{
		form{ method:"post" action:"/items" input{ name:"item" } button{ "add" } }
		ul{ for item in items { li{ item } } }
	}
}` },
	transitions: { value: 'div{button{data-warp-click:"1" "rotate"} button{data-warp-click:"2" "remove"} ul{li{key:1 data-warp-starting-style:opacity:0 style:transition:"opacity 150ms, transform 150ms" "milk"} li{key:2 data-warp-starting-style:opacity:0 style:transition:"opacity 150ms, transform 150ms" "eggs"} li{key:3 data-warp-starting-style:opacity:0 style:transition:"opacity 150ms, transform 150ms" "tea"}}}', clicks: ["rotate", "remove"], clicked: 'div{button{data-warp-click:"1" "rotate"} button{data-warp-click:"2" "remove"} ul{li{key:3 data-warp-starting-style:opacity:0 style:transition:"opacity 150ms, transform 150ms" "tea"} li{key:1 data-warp-starting-style:opacity:0 style:transition:"opacity 150ms, transform 150ms" "milk"}}}', keyed: true, animated: true, code: `// CSS transitions: removed items fade out, the others glide
todos = [{id:1 text:"milk"} {id:2 text:"eggs"} {id:3 text:"tea"}]
div{
	button{ on click { todos = todos[1..] + [todos#1] } "rotate" }
	button{ on click { todos = todos[1..] } "remove" }
	ul{ [li{ key: todo.id starting-style: { opacity: 0 } transition: "opacity 150ms, transform 150ms" todo.text } for todo in todos] }
}` },
	"wasm components": { value: '[12586269025 ["a" "bc" "d"] 7]', code: `// a Rust library compiled to a WebAssembly component
use rust_demo.wasm
c = rust_demo.counter(5)
c.increment(2)
[rust_demo.fib(50), rust_demo.words("a bc d"), c.value()]` },
	"C libraries": { value: '["stack" "/b/c" 3400449319 "1.3.2"]', code: `// C functions and zlib, compiled to WebAssembly
use c
import tests/fixtures/wasm/zlib
[strstr("haystack", "st"), strchr("a/b/c", 47), crc32(0, "wasp", 4), zlibVersion()]` },
	"kitchen sink": { value: '"✓ 34 tests passed"', get code() { return SAMPLES.kitchensink; } },
};
