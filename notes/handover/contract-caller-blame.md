# contract-caller-blame (parked)

Design (decision 1319f7ad8d): a function's leading asserts are its contract; each call checks them first, so a broken
contract names the call: `f(-1)` fails "assertion failed: x > 0 in the call f(-1)".

Tried as a program-level rule in extensions/sugar/calls.warp, `rewrite $p => $q where program and $q =
contracts_blamed($p)`: it never fires. Sugar modules go through `sugar_applied` (core/passes/rewrites.warp), which
applies no Place.Program rule; only the std modules' program rules reach `with_program_rules`. A per-call rule
(`$f($args...)` with a rule word) would look the definition up at every call of every program, the compiler's own too.

Ways on: `sugar_applied` applies the Place.Program rules of the sugar too (a few core lines, needs the OK), or the
contract rule lives in a std module taken when a program says `assert`.

The rule word as written (rule_words.warp, removed again): collect `name → [parameter names, leading assert·else
statements]` over level_statements; walk the program (a definition's head is no call); a round group headed by such
a name with as many positional arguments becomes `{x·contract = arg; (assert·else cond' "msg in the call f(arg)");
f(x·contract)}`, cond' the condition with each parameter renamed (renamed, core/passes/lambdas.warp). At rule time
`assert c` is already `assert·else(c, "assertion failed: c")`.
Outcome tests: `f(x) := { assert x > 0; x * 2 }; f(3)` gives 6; `try f(-1) catch e { e.message }` gives
'assertion failed: x > 0 in the call f(-1)'.
