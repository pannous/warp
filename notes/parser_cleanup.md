# Parser cleanup (user 2026-11-03: "denser, if it doesn't compromise readability"; "implement all as you suggested")

Decision parser-cleanup in notes/decisions.md. One branch and one landing per step, in this order; each step changes
no outcome (every test and sample gives what main gives) and reports parser.warp's lines and stage-two CPU
(warp-self.record) before and after in its merge message.

## Steps
1. rust-twin-comments: the core's 493 comment lines naming the retired Rust compiler ("Rust twin: atoms.rs …",
   "Rust lane …", `*.rs` names) go. A comment that only names the twin goes whole; a sentence about the Rust lane
   inside a longer comment goes, the rest stays. Comments only: smoke + self-check. Lands first and fast, because it
   touches every core file (tell the other workers to merge right after).
2. library-module: library_texts, library_sources, same_but_comments, blanked_comments, LIBRARY_FILES,
   library_of_core, use_library, run_library … (parser.warp ~40–110) leave the parser for one module beside their
   users (baking in core/passes/uses.warp, compiler.warp): reading and bundling lib/ is not parsing.
3. one-operator-reader: one reader for every operator declaration, the pattern
   `(infix|prefix|suffix|postfix|circumfix)? operator $op := body` (the user's note in parser.warp), giving one record
   (fixity, spelling, body or alias target). operator_alias / circumfix_alias / *_aliases_in / math_* and
   extensions/declared_operators.warp consume it; parallel functions differing only in fixity go.
4. atoms-as-rows: the parser's own atoms become rows of the same atom_readers table the extensions fill (`(` `[` `{`
   → grouped, quotes → quoted, `$` → hole, `@` → attribute, digits → number …); bare_atom becomes the lookup plus
   the fallback. Core and extensions read atoms through one mechanism.

## Review checklist (Supervisor reviews each landing's diff against it)
- A dispatcher's arm is one line naming the form it reads; the body is a function named for that form.
- Repeated idioms become one helper with a name, at the third copy at the latest. Seen in parser.warp:
  `peek(p.code, p.at + k, p.n)` 110×, `is_at(p.code, …, p.n)` 87×, `p.at = p.at + 1` 50× (`p.at += 1`),
  saving and restoring Parser flags around a read 15× (one "read with these flags" helper).
- A helper lives beside its users, not in the file that needed it first.
- Comments say why, never history (no "was", "used to", twins of retired code, card archaeology).
- No new special case: a step that seems to need one stops and asks (warp-interviewer).
- Denser never means cryptic: names stay full English words; no one-letter names beyond loop indices and `p`.
- Steps 3 and 4 change how the parser is built: their shape goes to warp-interviewer as an FYI/OK first.
