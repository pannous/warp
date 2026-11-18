#!/usr/bin/env bash
# wap's protocols (wiki/trait.md; decision traits-are-protocols): `trait shape { area(); scaled(by) }` names what a
# value can do, a Record of function-typed fields and no values; a class fits it by having each word.
# Run it with bash test/wap/test_protocols.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"

check "a trait declared beside a program" "$(ran declared 'trait shape { scaled(by); moved(x: int, y: int) }
print "ok"')" "ok"
exit $((failures > 0))
