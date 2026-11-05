# definition-head-phrases (parked: phrase-params, 2026-11-05)

Branch `definition-head-phrases` (tip 2cfd2e22c5, pushed), based on square-definition. Card definition-head-phrases (Later).
The user parked the parameter reading ((a) "named by the body or typed" vs (b) `give (v) to (c) := …`) and phrase heads
on keyword words.

## What the branch does
- One reader for both spellings, extensions/phrase_definitions.warp: `to give v to c: …` and `give v to c := …`
  (read_spaced_definition, a new core hook `statement_readers` in core/extension.warp, called from parser.warp at a
  statement's start before keyword readers). The core parser loses spaced_definition (it moves into the extension).
- Reading: first word = name; a preposition labels the slot after it; an article opens a noun slot; a typed word
  `(body: block)` is a slot; any other word the body never names (and no `it`) is a fixed word, joined into the name:
  `on_time every n (body: block)` → on_time_every(n, body).
- Function keeps today's labels: `give v to c := …` → give(v, to c); `give(1, 2)`, `give 3 to: 4` work.
- Row: a head with fixed words or prepositions also yields `rewrite give $v to $c => give($v, to: $c)` (a block slot as
  `() => $body`), parsed after the program's own statements (Parser.later), so `give 2 to 40` calls it.
  `to add a to b: a + b; add 1 to 2` then works (fails on main).
- A head starting with a keyword reader's word (`send`, an emit word) raises naming it.
- Module scanners see phrase heads (prelude_words phrase_heads → rule_patterns/rule_heads, rewrites rule_needs), so
  library rules (channels, timers, tasks) can become declarations.
- Reading note: hints.data `phrase_reading` printing `give (v) to (c) := …` (rewrites note_phrase_reading).

## Lessons
- A phrase-shaped Key surviving past the parser breaks early passes (c_imports read a parameter `c` as the C module,
  implicit uses pulled channels for `send`): emit the plain definition and a plain rule statement from the parser.
- text `find` is 1-based while slices are 0-based: split instead.
- `send` is an emit word (extensions/event_phrases.warp emit_words): the decision's example needs another word.

## Open
- The tip 2cfd2e22c5 does not build: `label_of(Node) is declared in both core/analysis/signatures.warp and
  extensions/phrase_definitions.warp` — rename the extension's (e.g. phrase_label) first.
- Verify the last build (to-definition tests: test_to_definition, test_type_name_matching, test_phrase_class_parameter,
  test_statement_blocks, test_kept_wiki_forms) against main; then self-check, gate.
- Library migration (~15–20 plain rules → declarations) after the reading is decided.
