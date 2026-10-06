#!/bin/bash
# Class cases ported from other languages (notes/classes.md): eval each and show the result next to the expectation.
#   probes/class_cases.sh [warp binary]   default: scratch/warp from scripts/own-warp.sh
WARP="${1:-$(dirname "$0")/../scratch/warp}"

cases=(
	# Python __init__ defaults / Kotlin default params
	'class point{x:int=0; y:int=0}; point().x + point(3).x|3'
	'class point{x:int=1; y:int=2}; p = point(); p.y|2'
	# Python/Ruby methods with self
	'class rect{w:int; h:int; area() := w * h}; rect(3, 4).area()|12'
	'class rect{w:int; h:int; area() := self.w * self.h}; rect(3, 4).area()|12'
	# Kotlin/C# property getter without parens
	'class square{side:int; area := side * side}; square(5).area|25'
	# method calling another method of the same class
	'class rect{w:int; h:int; area() := w * h; double() := 2 * area()}; rect(3, 4).double()|24'
	'class rect{w:int; h:int; area() := w * h; double() := 2 * self.area()}; rect(3, 4).double()|24'
	# method with arguments
	'class account{balance:int; after(amount:int) := balance + amount}; account(10).after(5)|15'
	# Java super: call the parent method from the override
	'class animal{name; speak() := name + " makes a sound"}; class dog extends animal{speak() := super.speak() + " (woof)"}; dog("Rex").speak()|Rex makes a sound (woof)'
	# three level inheritance
	'class a{x:int; f() := x}; class b extends a{g() := f() + 1}; class c extends b{h() := g() + 1}; c(1).h()|3'
	# polymorphism over a list of shapes (Java/C# abstract)
	'class shape{area() := 0}; class sq extends shape{s:int; area() := s * s}; class ci extends shape{r:int; area() := 3 * r * r}; sq(2).area() + ci(1).area()|7'
	# equality (Python dataclass __eq__, Kotlin data class)
	'class point{x:int; y:int}; point(1, 2) == point(1, 2)|1'
	'class point{x:int; y:int}; point(1, 2) == point(2, 1)|0'
	# printing (toString / __repr__)
	'class point{x:int; y:int}; "p=" + point(1, 2)|p=point{x:1 y:2}'
	# mutating method (Ruby/Swift mutating func)
	'class counter{n:int; inc() := n += 1}; c = counter(0); c.inc(); c.inc(); c.n|2'
	# field assignment from outside
	'class person{name; age:int}; p = person("Ann", 40); p.age = 41; p.age|41'
	# object in object (composition)
	'class engine{hp:int}; class car{e:engine}; car(engine(90)).e.hp|90'
	# static / class-level constant (Java static, Python class attribute)
	'class circle{r:int; static pi = 3}; circle.pi|3'
	# method returning a new instance (immutable with-style, Kotlin copy)
	'class point{x:int; y:int; moved(dx:int) := point(x + dx, y)}; point(1, 2).moved(3).x|4'
	# constructor from wiki/constructor.md: value block
	'class person{name; id:int; value{id = 7}}; person("Joe").id|7'
	# trait/interface in body
	'trait shape{area}; class sq{s:int; area() := s * s} is shape; sq(3).area()|9'
	# isinstance / is
	'class animal{name}; class dog extends animal{}; dog("Rex") is animal|1'
	# named construction
	'class person{name; age:int}; person{name:"Ann" age:3}.age|3'
)

for case in "${cases[@]}"; do
	code="${case%|*}"; expected="${case##*|}"
	actual=$("$WARP" --no-ask eval "$code" 2>&1 | tail -1)
	if [ "$actual" = "$expected" ]; then mark=ok; else mark=FAIL; fi
	printf '%-4s %s\n     → %s (want %s)\n' "$mark" "$code" "$actual" "$expected"
done
