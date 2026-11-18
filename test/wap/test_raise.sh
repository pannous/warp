#!/usr/bin/env bash
# wap's raise (wap/preload/prelude.wap; wiki/keyword.md raise): the program stops, its message on the standard error;
# raise fits where any value is wanted, since it never gives one. Run it with bash test/wap/test_raise.sh
source "$(dirname "$0")/compile.sh"

check "raise stops the program with its message" "$(ran stops 'print "before"
raise "stopped"
print "after"')" "before
stopped"
check "raise stands where a value is wanted" "$(ran as_value 'positive(x: int): int := if x > 0 then x else raise "not positive"
print "ok"
positive(-1)')" "ok
not positive"
check "the value when raise does not run" "$(ran not_raised 'positive(x: int): int := if x > 0 then x else raise "not positive"
positive(5)')" "5"
exit $((failures > 0))
