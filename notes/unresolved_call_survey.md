# Unresolved call in code position (decision #7, survey A15)

`print(3)`, `square(3.0)` or `min(1,2)` with nothing defined silently evaluates to the data `(print 3)`.
Experiment: export of 8f070776 in `probes/a15_wt` (own target dir; main untouched), one check in
`emit_list_node` (src/wasm_emitter/list_emitter.rs), placed after every resolver (user function, FFI, WASI, introspection, type
words, `return`…) and before the data fallthrough: an applied symbol nothing resolved → `emit_type_error("undefined function: <name>")`.
Switchable with the env var `A15_OFF` so baseline and experiment ran in the same binaries: `./test.sh`, all features.

## Key finding: call syntax is already distinguishable from paren data

The parser gives the same `List(items, Round)` for both, but the separator differs:

| source | items | bracket | separator |
|---|---|---|---|
| `print(3)`, `f(1,2)`, `f(a b)` | head symbol first | Round | **None** |
| `(print 3)`, `(a b c)` | | Round | Space |
| `(a,b)` | | Round | Colon |
| `g (a b)` (space before the paren) | | not Round | Space |

So "a symbol applied with parentheses" = `Round` + `Separator::None` + Symbol head. No parser change is needed.

## Test fallout

| variant | ./test.sh (760 tests) | changed |
|---|---|---|
| baseline (A15_OFF) | 754 passed, 6 failed (other agents' in-progress work) | – |
| naive: every Round list with a Symbol head and ≥2 items | 752 / 8 | `test_index`, `test_root_list_strings`: `(a b c)` is a *list of symbols* (data) and became `undefined function: a` |
| **Round + Separator::None** | 754 / 6, identical failure set | **none** |

## What "code position" means

Everything `eval`/compile emits is code, so the rule applies wherever the emitter reaches a list node:
top-level program, function bodies, operands (`1+print(3)`), assignment values (`x=print(3)`), arguments (`g(print(2))`),
elements and values inside `[..]` and `{..}` (`[print(3)]`, `{a:print(3)}`).
Not code: `parse`, `parse_data`, `warp data <file>` (nothing is emitted, `parse_data("print(3)")` stays `(print 3)`),
and paren data with a space or comma (`(print 3)`, `(a b c)`, `(a,b)`).

Three emitters must agree: node (`emit_list_node`), exact (`emit_numeric_value`), float (`emit_float_value`).
Only the node emitter had the experiment. In operand/assignment/function-body positions the numeric path reads the head as a
variable and, at this base, panics `Undefined variable: print` (a later commit turns that into the error value
`undefined variable: print`): loud but the message says variable, not function, so those two arms need the same check.

## What the experiment showed for other shapes

| input | before | with the rule |
|---|---|---|
| `print(3)`, `square(3.0)`, `f(1,2)` | `(print 3)` … | `undefined function: print` |
| `f(x):=x*2; g(3)` | `(g 3)` | `undefined function: g` |
| `[print(3)]`, `{a:print(3)}` | data | error |
| `min(1,2)`, `max(1,2)` | `(min 1 2)` | error: these are missing builtins, now visible |
| `type P{x:int}; P(1)` | `(P 1)` | error (a user type applied: needs its own resolver or message "construct with P{…}") |
| `x=2; x(3)` | `(2 3)` (x substituted) | `undefined function: x`; a variable is not callable (implicit multiplication would be a language decision) |
| `foo()` | `foo` | unchanged (zero-arg symbol form) |
| `if(1){2}`, `while(0){1}`, `sqrt(4)`, `puts("a")`, `type(3)`, `str(3)` | resolved | unchanged |

## Proposed rule

1. A call is `name(` written without a space: `Round` + `Separator::None` + Symbol head. It is code.
2. It resolves iff the name is a user function, FFI/WASI/host import, builtin (introspection, type words, keywords) or a user type
   (constructor). Otherwise compile-time error `undefined function: <name>` at the call's position (`Diagnostic::at`).
   A variable is not resolved as a callee.
3. Data stays data: parse/`parse_data`/`warp data`, and any paren list written with a space or comma. To build data in a program
   that looks like a call, write `(print 3)`, `[print 3]` or `print 3`.
4. Implementation: one helper `unresolved_call(items, bracket, separator) -> Option<&str>` used by the node, exact and float
   list arms, plus one `resolves_call(name)` that lists the resolvers (today they are spread over ~six `if`s in `emit_list_node`).
5. Decisions for the user: (a) user-type constructor `P(1)`: support it or error with a hint; (b) `x(3)` with a variable callee:
   error or multiplication; (c) whether `min`/`max`/`print` should exist as builtins (they now surface as errors).
