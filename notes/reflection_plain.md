# Reflection and help as a plain library (card introspection-reflection)

Decision domains-plain-libraries: introspection.rs (~55 lines), word_help.rs (~220) and reflection.rs (~1020) in
src/lowering/data/ go; reflection, `dir` and `help` become warp functions over a few core values. Owner:
warp-self-analyzer. One small branch per step, each with its net src/ delta.

## Core values the library reads (the only compiler part)
- function value (decided, decisions.md reflection): `.name .params .signature .return_type .doc`; emitted today as
  field fallbacks over the closure targets (closures.rs FUNCTION_WORDS). Missing at run time: `.body .effects .dump`.
- class value (`type(o)`, `P`): `.name .fields .methods .parent`. Today `type(o).fields` works on an instance only
  through reflection.rs; `P.fields` of the class name fails ("no field fields").
- module value (`math` after `use math`): `.name .words .doc`, and a word by name (`sound[trick]`) for help of a name
  known only at run time. Today `dir(math)` is answered by introspection.rs, `f(m) := dir(m); f(math)` prints nothing.

## Library (lib/reflection.warp, implicit like other std words)
`dir`, `fields` (aliases attributes, members, properties), `methods` (alias functions), `help`, `keys` of instances,
`exports`, `dump`: each one function over the values above, typed overloads by kind (`dir(m: map) := m.keys`).

## Steps
1. introspection.rs → library `dir(m: module)` / `methods(m: module)` (needs module values)
2. word_help.rs → library `help` (needs the help-statement answer, module docs)
3. reflection.rs objects (fields/methods/dir of instances and classes) → library over class values
4. reflection.rs function words → library over function values (body/effects/dump fields)

## Answers (decision reflection-plain, 2026-10-18)
- Q1 hybrid: one core builtin `meta(x): map` (src/lowering/data/meta_values.rs) gives what the compiler knows of a
  class, module or function; lib/ builds the typed views on it (`fields(o) := meta(type(o)).fields`,
  `dir(m) := meta(m).words`). Only the library reads the map.
- Q2 (a): `help(x)` gives the text, a statement prints nothing; mid-program `print help(x)`. The printing pass is gone.

## How meta works
- `meta(NAME)` of a module or class named as written: a map literal (`{name words doc}`, `{name fields methods parent}`).
- Anything else is `meta_of(x)`, which the program defines (`with_table`, a program-only pass: a module source's
  table would lack the program's classes): `TABLE[string(x)]`, keyed by `math` / `class P` (a class value prints so).
  Not yet `if TABLE has … else error`: a field of a parenthesized if-expression fails (card field-parenthesized).
- `dir(math)`, `methods(math)`, `math.methods` of a module named as written are views of its meta in the same pass
  (introspection.rs is gone); `dir(m)` of a module known only at run time falls back to `meta_of(m).words`
  (reflection.rs dir_otherwise).

## Status
- step 2a (printing pass) and step 1 (meta core, introspection.rs gone): branch reflection-plain
- next: move class reflection (fields/methods/dir of instances and classes) onto `meta` in lib, then help

## Step 3 design (class reflection onto meta)
- First bite: reflection.rs `Objects::dispatched` (the run-time type test over every class for `f(o) := o.fields`,
  `make().class`) becomes `meta_of(type(o)).fields` / `.methods`: the table already holds every class under
  `class P`. The compile-time answers (known class, own member wins: `has_member`) stay until the library has typed
  overloads for instances.
- `meta(P).methods` holds method values; calling one read out of a map fails with an instance argument (card
  method-value), so `methods(o)#1(o)` through meta waits for that card.
- `help(P)` / `help(p)` (Objects::help) is text built from fields, methods and parent: a library
  `help(c) := "class " + meta(c).name + …` once help has typed overloads (function, module, class).
- Traps: a module symbol through two untyped functions reads as an int on main (card regression-module, one ignored
  test in test_meta_values); `(if … then map else …).field` fails (card field-parenthesized).
