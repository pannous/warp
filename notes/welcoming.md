# Welcoming syntax

Mantra:
- If the intent is clear, compile it.
- If there is an ambiguity, ask the user.
- If the preferred syntax is different, educate the user.

"Ask" means a warning or hint at compile time; "educate" means a hint naming the preferred wasp form, never a refusal.

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
