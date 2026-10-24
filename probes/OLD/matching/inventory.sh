#!/bin/bash
# card matching-create: every matching form of the spec (wiki/matching.md, pattern-matching.md, switch.md, call.md,
# for.md, constructor.md, indexing.md, Features.md) and of wasp, run on a warp build; prints ok/DIFF per case
# usage: probes/matching/inventory.sh [warp binary]
WARP=${1:-warp}
pass=0
fail=0
check() { # name, expected last line (or error:<text> for an error containing text), code
	local got
	local out
	out=$(WARP_NO_WINDOW=1 WARP_HINTS=0 "$WARP" eval "$3" 2>&1)
	got=$(echo "$out" | tail -1 | sed "s/^» //")
	[[ "$got" == *"|"* ]] && got=$(echo "$out" | head -1)
	if [ "$got" == "$2" ] || [[ "$2" == error:* && "$got" == *Error\(*"${2#error:}"* ]]; then
		pass=$((pass + 1))
		echo "ok    $1"
	else
		fail=$((fail + 1))
		echo "DIFF  $1: want $2, got ${got:0:150}"
	fi
}

# switch / match: literal cases, default, map indexing (done, baseline)
check switch-literal '"fire"' 'color = "red"; switch color { "green": "woods"; "red": "fire" }'
check switch-symbol '"fire"' 'color = red; switch color { green: "woods"; red: "fire" }'
check switch-default '"other"' 'switch 7 { 1: "one"; default: "other" }'
check map-call-select 2 '{a:1+1 b:2}(a)'
check map-index-select 1 'x = a; {a:1 b:2}[x]'

# structural patterns (done, baseline)
check list-pattern 3 'match [1, 2] { [a, b] => a + b }'
check rest-pattern 6 'total(xs) := match xs { [] => 0; [first, ...rest] => first + total(rest) }; total([1, 2, 3])'
check guard '"negative"' 'match -3 { n if n < 0 => "negative"; _ => "other" }'
check class-pattern 3 'class Point{x:int; y:int}; p = Point(3, 4); match p { Point{x, ..} => x }'
check list-destructuring 3 '[a, b] = [1, 2]; a + b'
check tuple-destructuring 3 '(a, b) = (1, 2); a + b'
check rest-destructuring 5 '[first, ...rest] = [1, 2, 3]; first + #rest * 2'
check field-destructuring 7 'class Point{x:int; y:int}; {x, y} = Point(3, 4); x + y'

# alternatives, ranges, bindings inside arms
check or-pattern '"small"' 'match 2 { 1 | 2 => "small"; _ => "big" }'
check or-word-pattern '"small"' 'match 2 { 1 or 2 => "small"; _ => "big" }'
check comma-pattern '"small"' 'match 2 { 1, 2 => "small"; _ => "big" }'
check type-alternatives '"plain"' 'f(v) := match v { int | text => "plain"; _ => "other" }; f("a")'
check mixed-alternatives '"zero or text"' 'f(v) := match v { 0 or text => "zero or text"; _ => "other" }; f("a")'
check is-type-union 'yes' 'x = 3; x is (int or text)'
check other-catch-all 0 'match 3 { 1 => 1; other => 0 }'
check else-catch-all 0 'match 3 { 1 => 1; else => 0 }'
check range-pattern '"teen"' 'match 15 { 13..19 => "teen"; _ => "other" }'
# user decision: no @/as binding, the subject's name or the word value gives the whole value
check as-binding "error:'@'" 'match [1, 2] { all @ [a, b] => #all + 1 }'
check text-pattern '"hi bob"' 'match "bob" { "alice" => "hey"; name => "hi " + name }'
check map-pattern '"Ann"' 'match {name: "Ann", age: 3} { {name: n} => n }'
check nested-class-pattern 1 'class P{x:int; y:int}; class L{a:P; b:P}; match L(P(1,2), P(3,4)) { L{a: P{x, ..}, ..} => x }'

# type patterns: by type in an arm
check type-arm-int '"int 3"' 'describe(v) := match v { i: int => "int " + i; t: text => "text " + t }; describe(3)'
check type-arm-text '"text a"' 'describe(v) := match v { i: int => "int " + i; t: text => "text " + t }; describe("a")'
check type-arm-bare '"number"' 'kind_of(v) := match v { int => "number"; text => "words" }; kind_of(3)'
check type-arm-class '"circle"' 'class Circle{r:int}; class Square{s:int}; name(v) := match v { Circle => "circle"; Square => "square" }; name(Circle(1))'

# enum / variant cases (wiki/matching.md "not yet: match on enum cases")
check enum-cases 10 'enum Coin { Penny, Nickel, Dime, Quarter }; c = Dime; match c { Penny: 1; Nickel: 5; Dime: 10; Quarter: 25 }'
check enum-scoped-cases 10 'enum Coin { Penny, Nickel, Dime, Quarter }; c = Coin::Dime; match c { Coin::Penny => 1; Coin::Dime => 10; _ => 0 }'
check enum-exhaustive-missing 'error:match on Coin misses Dime' 'enum Coin { Penny, Dime }; c = Dime; match c { Penny: 1 }'

# test without arms: is / matches
check is-pattern 'yes' 'xs = [1, 2]; xs is [_, 2]'
check if-match 2 'xs = [1, 2]; if xs is [a, b] then b else 0'
check regex-match 'yes' '"abc" matches /a.c/'

# definitions by cases (equations): fib 0 = 0; fib 1 = 1; fib n = …
# user decision: no joining by cases, the error's fix shows the match
check equations-by-cases 'error:fib is defined twice' 'fib(0) := 0; fib(1) := 1; fib(n) := fib(n-1) + fib(n-2); fib(10)'
check parameter-pattern 3 'add_pair([a, b]) := a + b; add_pair([1, 2])'
check class-parameter-pattern 7 'class Point{x:int; y:int}; sum(Point{x, y}) := x + y; sum(Point(3, 4))'

# matching by type name (D5, done) and its open part: multi-word classes
check type-name-parameter 9 'square of a number = it*it; square of 3'
check multi-word-class '"Ada Lovelace"' 'class full name { first: text; last: text }; n = full name("Ada", "Lovelace"); n.first + " " + n.last'
check field-type-by-name 'yes' 'class Email{address:text}; class contact { name; email? }; c = contact(name: "a"); c.email is ø'

# dispatch: by argument type (done), by return type (done), to … as …
check type-dispatch '"int"' 'show(x:int) := "int"; show(x:text) := "text"; show(3)'
check render-as 'pdf' 'class pdf{body:text}; to render text as pdf: pdf(text); type of (render "hello" as pdf)'

# for TYPE in …: type-filtered iteration (wiki/for.md)
check for-type-filter 2 'class friend{n:int}; class foe{n:int}; count = 0; for friend in [foe(1), friend(2), foe(3), friend(4)] { count += 1 }; count'

# gap feeding: missing arguments from same-named locals (wiki/call.md)
check gap-feeding 2 'to integrate a function from a to b: b - a; from = 1; to = 3; function = sin; integrate function'

# constructor: given data matched to fields by name (wiki/constructor.md)
check constructor-by-name '"Bern"' 'class person{name:text; city:text}; p = person{city: "Bern", name: "Ann"}; p.city'
check constructor-by-type '"Bern"' 'class city{name:text}; class person{name:text; home:city}; p = person("Ann", city("Bern")); p.home.name'

echo "$pass ok, $fail differ"
