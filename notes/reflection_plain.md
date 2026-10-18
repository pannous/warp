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

## Open (asked warp-interviewer 2026-10-18)
- class and module values as core values with plain fields
- help-prints needs statement position, which a library can't see
