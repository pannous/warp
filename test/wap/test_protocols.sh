#!/usr/bin/env bash
# wap's protocols (wiki/trait.md; decision traits-are-protocols): `trait shape { area(); scaled(by) }` names what a
# value can do, a Record of function-typed fields and no values; a class fits it by having each word.
# Run it with bash test/wap/test_protocols.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a trait declared beside a program" "$(ran declared 'trait shape { scaled(by); moved(x: int, y: int) }
print "ok"')" "ok"
check "a trait word without parameters" "$(ran no_parameters 'trait shape { area(); scaled(by) }
print "ok"')" "ok"
check "a class fits a protocol by having its words, each call the given class's own" "$(ran fits 'trait shape { area() }
class square { side: int }
area(s: square) := s.side * s.side
class rect { w: int
h: int
}
area(r: rect) := r.w * r.h
total(a: shape, b: shape) := area(a) + area(b)
print text(total(square(1), rect(2, 3)))')" "7"
exit $((failures > 0))
