// The tour: short programs from the is! tests and notes/welcoming.md; samples.js (made by build.sh) adds samples/*.wasp
const EXAMPLES = {
	welcome: `// edit me: the value of the last line is the result
greeting = "Hello " + "🌍"
print greeting
6*7`,
	math: `1+2*√3^2`,
	fibonacci: `fib := it < 2 ? it : fib(it - 1) + fib(it - 2)
fib(10)`,
	functions: `def square(x) = x*x
square(4) + square 3`,
	lists: `xs = [3 1 2]
xs#1 + #xs + max(xs)`,
	maps: `ages = {alice: 30, bob: 25}
ages.bob + ages.alice`,
	texts: `name = "warp"
"Hi " + name + ", " + #name + " letters"`,
	ask: `// an ambiguity is a question: click a reading below
x=0
for i in 1 upto 4 { x += i }
x`,
	"kotlin range": `// Kotlin's .. includes the end, wasp's excludes it: which did you mean?
n=4; c=0
for i in 0..n-1 { c += 1 }
c`,
	globals: `n = 0
def count(x){ n = x; x }
count(3)
n`,
	educate: `x = 7 // after code: is this a comment or floor division?
x`,
	"runtime error": `xs = [1 2 3]
xs[3]`,
	fetch: `fetch https://raw.githubusercontent.com/pannous/warp/main/README.md`,
};
