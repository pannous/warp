#!/usr/bin/env bash
# wap's texts with holes (wiki/string.md "templates interpolation"): `"a ${x} b"` and `"a \(x) b"` show x's text in
# place, a hole is any code, texts inside it too; a hole's value is shown by text(x). Run it with bash
# test/wap/test_interpolation.sh
source "$(dirname "$0")/compile.sh"
source "$(dirname "$0")/format_checks.sh"

check "a hole shows a variable's value" "$(ran hole 'n = 3
print "n is ${n}"')" "n is 3"
check "a hole of code with texts inside" "$(ran nested 'c = yes
print "<${if c then "a" else ""}>"
print "<${if no then "a" else ""}>"')" "<a>
<>"
check "holes written the Swift way, several in a text" "$(ran swift 'a = 1
b = 2
print "\(a) and \(b) make \(a + b)"')" "1 and 2 make 3"
check "the texts of ints, characters, yes and no" "$(ran shown 'print "${-120} ${0} ${'"'"'é'"'"'} ${yes} ${no}"')" "-120 0 é yes no"
check "a dollar without braces and an escaped hole stay text" "$(ran dollars 'print "costs \$5, \${x} and $y"')" 'costs $5, ${x} and $y'
# wap format prints a text with holes as it was written
format_check 'print "n is ${n + 1}, \(m)"' 'print "n is ${n + 1}, \(m)"'
exit $((failures > 0))
