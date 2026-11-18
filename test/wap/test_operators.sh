#!/usr/bin/env bash
# wap's operator stage (notes/backbone.md "Operators", notes/operators.md): a program printed back by `format` in
# canonical warp, every operator read by the table of wap/preload/operators.wap.
# Run it with bash test/wap/test_operators.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/format_checks.sh"

format_check '1+2 * 3' '1 + 2 * 3'
format_check '(1 + 2) * 3' '(1 + 2) * 3'
format_check '2^3^2' '2 ^ 3 ^ 2'
format_check '8 - 4 - 2' '8 - 4 - 2'
format_check 'x ≤ y' 'x <= y'
format_check 'a plus b' 'a + b'
format_check 'yes and true' 'yes and yes'
format_check 'x += 1' 'x += 1'
format_check 'f = x => x' 'f = x => x'
format_check 'f: (int)->int' 'f: (int) -> int'
format_check 'x  +=1' 'x += 1'
format_check 'x = c?1 : 2' 'x = c ? 1 : 2'
format_check 'print if c then 1 else 2' 'print (if c then 1 else 2)'
format_check 'not a and b' 'not a and b'
# the minus rule: spacing decides prefix or infix, and format spells the reading out
format_check '[a -1]' '[a, -1]'
format_check '[a - 1]' '[a - 1]'
format_check 'a-b' 'a - b'
format_check 'print -x' 'print (-x)'
format_check 'print 1 + 2' 'print 1 + 2'
# a spaced call takes the whole expression after it (decision spaced-call-precedence): brackets keep the other reading
format_check '(square 3) * 3' '(square 3) * 3'
format_check 'square(3) * 3' 'square(3) * 3'
# brackets that precedence makes needless are dropped, the others kept
format_check '1 + (2 * 3)' '1 + 2 * 3'
# a program's own operators (wiki/operator.md): a declared infix binds like +, a placed one by its relation
format_check $'infix operator ⊕(a, b) := a * b\n1 ⊕ (2 * 3)\n(1 ⊕ 2) * 3' $'infix operator ⊕(a, b) := a * b\n1 ⊕ 2 * 3\n(1 ⊕ 2) * 3'
format_check $'infix operator ⊗ := left * right\noperator ⊗ has precedence above *\n2 * (3 ⊗ 4)' $'infix operator ⊗ := left * right\noperator ⊗ has precedence above *\n2 * 3 ⊗ 4'
format_check $'infix operator divides(d, n) := n % d == 0\n3 divides (2 * 6)' $'infix operator divides(d, n) := n % d == 0\n3 divides 2 * 6'
# an operator with no operand after it is a name
format_check 'total := fold +' 'total := fold +'
# a function's result type prints after `as`, the canonical word (wiki/function.md); a typed name keeps its `:`
format_check 'twice(x): int := x * 2' 'twice(x) as int := x * 2'
format_check 'x: int := 3' 'x: int := 3'
format_check 'x: int = 3' 'x: int = 3'
# a case named by its enum, either way (wiki/enum.md)
format_check 'Coin::Dime' 'Coin.Dime'
format_check 'x = p . x+1' 'x = p.x + 1'
format_check 'rule show { ($k: $v)... } => 1' 'rule show {($k: $v)...} => 1'
# an optional's value, `x!` (wiki/optional.md), beside the prefix `!x` for not
format_check 'y = x!' 'y = x!'
format_check 'z = !x' 'z = not x'
# a word leading a rule's phrase after an infix takes the rest of the phrase, as at a phrase's start; other words stay operands
format_check 'x = f(a) ?? return ø' 'x = f(a) ?? (return ø)'
format_check 'n = if s < 0 then 0 - s else s' 'n = if s < 0 then 0 - s else s'
format_check 'z = a ?? if c {1} else {2}' 'z = a ?? (if c {1} else {2})'
check "a match after an assignment keeps its arms" "$(ran assigned_match 'x = 2
y = match x { 1 => "a"; _ => "b" }
print y')" "b"
check "an if after ?? takes its blocks" "$(ran default_if 'f(n: int) as int? := if n > 0 then n else ø
z = f(0) ?? if 2 > 1 {7} else {8}
print text(z)')" "7"
format_check 'x = if c then 1
	else 2' 'x = if c then 1 else 2'
exit $((failures > 0))
