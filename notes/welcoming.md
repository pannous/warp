# Welcoming syntax

Mantra:
- If the intent is clear, compile it.
- If there is an ambiguity, ask the user.
- If the preferred syntax is different, educate the user.

Limit (user, 2026-10-02): implement what a newcomer expects only where it does not clash with a known footgun
(wiki/Footguns.md). Where it clashes, the footgun decision wins and the compiler educates instead. The one exception is
text + number concatenation. Examples: `a[-1]` stays an error with a hint to `#-1` (silent wrap), `[x]*n` is an Ask
(Python repeats, NumPy multiplies; `n times [x]` repeats), mutating a main-level variable from a function needs `global`.

"Educate" is a hint naming the preferred wasp form, never a refusal.

"Ask" is a diagnostic category of its own and a new programming paradigm: by default the compiler really
pops up a question to the user (which interpretation did you mean?) and compiles the answer. Only when the
question cannot be escalated to a user (tests, CI, piped/non-interactive runs, a special no-ask mode) does an
Ask degrade to its fallback, which each ambiguity declares: a warning (take the default reading, continue) or
an error (too dangerous to guess).

## Field test 2026-10-02
Agents wrote standard algorithms (sorting, life, sieve, levenshtein, queens/hanoi, dijkstra) in their natural
Python/JS style: samples/<name>.wasp next to samples/<name>_idiomatic.wasp, tests in tests/test_algo_<name>.rs.
Fix branches fix-<topic> carry the compiler changes.

User decisions from that round:
- text + number concatenates (`"F:" + 13` → "F:13")
- `//` with no space before it is floor division (`7//2`, `x//=2`); `x // note` stays a comment
- `#ident` with no space is count (`#s`); `# text` stays a comment; directives (#use, #include, #!) keep working
- `upto` excludes the end, with a warning naming the explicit forms (`..<`/`..` exclusive, `to`/`...` inclusive)
- missing map key `m["Z"]` stays an error (as in Python); `m.get(k)`, `m.get(k, default)`, `k in m`, `m.has(k)` are the soft forms
