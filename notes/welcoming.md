# Welcoming syntax

Mantra:
- If the intent is clear, compile it.
- If there is an ambiguity, warn with the default reading and the explicit forms (or refuse when a guess is dangerous).
- If the preferred syntax is different, educate the user.

Limit (user, 2026-10-02): implement what a newcomer expects only where it does not clash with a known footgun
(wiki/Footguns.md). Where it clashes, the footgun decision wins and the compiler educates instead. The one exception is
text + number concatenation. Examples: `a[-1]` stays an error with a hint to `#-1` (silent wrap), `[x]*n` is an
ambiguity error (Python repeats, NumPy multiplies; `n times [x]` repeats), mutating a main-level variable from a function needs `global`.

"Educate" is a hint naming the preferred wasp form, never a refusal.

"Ask" (the name stayed, the question went) is an ambiguity record. USER DECISION 2026-10-03: "I really love the got it
mechanism for the warnings, the Ask mechanism is not what I expected. I thought it would rewrite the code to whatever
the user picks; we don't want context-sensitive execution, instead turn all the Ask into a warning with the got it
feature, plus an extra feature for later: an intelligent intent to change the code." So the same source always compiles
the same way: nothing asks which reading was meant and no answer is remembered. Each ambiguity declares its fallback:
a warning (take the default reading, shown until the user says "got it") or an error naming the explicit forms (too
dangerous to guess; user, same day: "Stay loud errors").

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
- `Ask { topic, written, question, readings: [Reading{meaning, explicit_form}], default, fallback: Fallback::{Warning, Error}, line, column }`;
  `ask(&Ask) -> Result<usize, Node>` returns the default reading or the error. It never asks and never remembers a reading.
- Fallback Warning prints `warning: <question> (taking <default>) at l:c; fix: <explicit form>` and takes the default.
  The warning shows until the user says "got it" for its topic, then never again (an acknowledgement silences, it never
  changes the value). Under `--strict` or a `use strict` line it is an error, acknowledged or not.
- Fallback Error returns `Node::Error` naming every explicit form (`` `a` for first or `b` for second``), acknowledged or not.
- "Got it" is the educate_once machinery, shared by Ask warnings and educating notes (`educate_once(topic, written,
  preferred, reason)`): `trait Acknowledger` (`has_acknowledged` for a host's own list, `acknowledge` asked once per run
  after the warning or note), `set_acknowledger` / `with_acknowledger`; `TerminalAcknowledger` (`got it? [y …]`),
  `Acknowledging(topics)` for tests, the playground's page list (src/web.rs `PageAcknowledger`, report `notes`).
  Remembered as `ack:<topic> = acknowledged` in `.wasp-acknowledged` (CLI, working directory). Non-interactive runs
  just show the warning, never block; `--no-ask` never prompts.
- Users: `a upto b` (topic `upto`) and the for-header bound `0..n-1` (topic `kotlin-range`, Kotlin's `..` is inclusive),
  both default exclusive, fallback Warning (src/wasp_parser.rs `range_reading`); `local-or-global` (src/analyzer.rs
  `resolve_main_variable_assignments`, default a new local `let n = …`, explicit main's `global n = …`); `signed-operand`
  `1 -1` (default the list `[1 -1]`). Error fallback: `list-times` `[x]*n`, `insert-order`, `list-plus` `[1 2 3]+4`,
  `bare-list` `a=1 2 3`, `suffix-precedence` `1+2 squared`. Explicit forms (`..<`, `...`, `to`, `global n`, `[4]`, `.+`
  …) never warn. tests/test_got_it_warnings.rs, test_welcoming_ask.rs, test_welcoming_globals.rs.
- A wrong guess fails far away, so `eval` names it: a runtime error lists the defaults the warnings took
  (`assumed at 23:13, 24:15: …; fix: ..<`, diagnostic::take_assumptions), and an index out of range in a program with an
  exclusive range adds "hint: `..` excludes the end; `...` or `to` include it". The kotlin-range warning fires for every
  for-header `a..b-1` form, also `(0..n-1)`, `0..(n-1)`, `do` bodies and loops inside functions.
- New ambiguities: build an `Ask` where the ambiguity is still visible (often the parser, which knows the written form),
  give every reading its explicit form, pick Error only when a wrong guess would silently corrupt results.

## Later: change the code to the intended form (not built)
The user's "intelligent intent to change the code": instead of a question that changes what the source means, an
action that rewrites the source to the explicit form the user picks, so the file itself says it. Seed data is already
there: every `Ask` keeps `written` (the ambiguous text), `line`/`column` and each reading's `explicit_form`, and the
warning's `fix` carries the default's form. A host offers the readings as "change code" buttons (playground) or quick
fixes (IDE/LSP code actions); the edit replaces `written` at the position with the chosen `explicit_form`, after which
the warning is gone because the explicit form never warns. Open: positions of multi-line `written`, several readings
whose explicit form needs surrounding context (`global n` is a declaration elsewhere), and a CLI form (`warp fix`).
