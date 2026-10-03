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

## Ask: how it works (src/diagnostic.rs)
- `Ask { topic, question, readings: [Reading{meaning, explicit_form}], default, fallback: Fallback::{Warning, Error}, line, column }`;
  `ask(&Ask) -> Result<usize, Node>` returns the chosen reading. The is-a-warning/error relation is the `fallback` field.
- Order: a remembered answer (per topic) → the asker → the fallback. Fallback Warning prints
  `warning: <question> (taking <default>) at l:c; fix: <explicit form>` and takes the default (an error under `--strict` or
  a `use strict` line); fallback Error returns `Node::Error`.
- Askers are pluggable (`trait Asker`, `set_asker` / `with_asker`): `TerminalAsker` (numbered readings, default marked,
  Enter = default, end of input = no answer), `ScriptedAnswers` for tests; later web/IDE. The library default is no asker,
  so tests and embedders never block on a prompt.
- CLI: asks on the terminal when stdin and stderr are terminals, `CI` is unset and `--no-ask` is absent. Answers persist in
  `.wasp-answers` in the working directory (`topic = explicit form` per line); an answered Ask hints the explicit form
  (educate channel `normalize::hint`), so writing it removes the question for good.
- Users so far (src/wasp_parser.rs `range_reading`): `a upto b` (topic `upto`) and the for-header bound `0..n-1`
  (topic `kotlin-range`, Kotlin's `..` is inclusive), both default exclusive (wasp's documented reading), fallback Warning.
  Explicit `..<`, `...`, `to` never ask. tests/test_welcoming_ask.rs.
- `local-or-global` (src/analyzer.rs `resolve_main_variable_assignments`): `n = …` fresh inside a function while main has an
  n: a new local (default, `let n = …`) or main's n (`global n`), fallback Warning. tests/test_welcoming_globals.rs.
- An unanswered Error-fallback Ask names every explicit form in its fix (`` `a` for first or `b` for second``).
- Educate with acknowledge-once: `educate_once(topic, written, preferred, reason)` shows the hint once per run until
  the user acknowledges it (`Asker::acknowledge`; terminal: `y`; scripted: answer `ACKNOWLEDGED`), then never again:
  remembered as `ack:<topic> = acknowledged` next to the answers. Non-interactive runs just show it, never block.
- A wrong guess fails far away, so `eval` names it: a runtime error lists the defaults unanswered Asks took
  (`assumed at 23:13, 24:15: …; fix: ..<`, diagnostic::take_assumptions), and an index out of range in a program with an
  exclusive range adds "hint: `..` excludes the end; `...` or `to` include it". The kotlin-range Ask fires for every
  for-header `a..b-1` form, also `(0..n-1)`, `0..(n-1)`, `do` bodies and loops inside functions.
- New Asks: build an `Ask` where the ambiguity is still visible (often the parser, which knows the written form), map the
  chosen index to the reading, pick Error as fallback only when a wrong guess would silently corrupt results.
