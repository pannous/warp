# Taste: what warp calls beautiful

The user's taste, by example (2026-10-27). Judge a design, a test or a sample by these, not by whether it merely works.

## Beautiful
- Plain warp over special syntax: `on escape { quit() }` is a call of a library function `on(key: Key, handler: block)`;
  `escape` is found as `Key.escape` because the parameter wants a Key (names from the expected type, wiki/inventions.md).
- Units are values: `sleep(3ms)`, `5km == 5000m`, `90km/h`: implicit multiplication `5 * km`, Quantity in lib/units.warp.
- Named fields and enums: `p.kind`, `enum Level { Error, Warning }`, `Bracket.Square`, `enum Coin { Penny = 1, Dime = 10 }`.
- English-like general forms the wiki shows: `square of number = it*it`, `for (dx, dy) in moves`, `xs from #2 to #4`,
  `if no x`, `x is empty`.
- One general rule that holds for its siblings: every type, every operator, not the one example at hand.

## Ugly
- One rewrite rule per name: `rewrite $k => "Escape" where $k in [ESC, ESCAPE, Escape, esc, escape]`; vocabulary is an
  enum in lib/.
- Magic words and glue names visible to programs or pinned by tests: `ran·without·error`, `(on··every··0)(t)`.
- Text or number codes where a type belongs: kinds as `"int"`, `value#1` / `value#2` of a pair instead of fields.
- A sugar rule that drops meaning: `300ms` → `0.3`, the unit lost.
- Rust-isms: `is!("3+3", 6)` call syntax in code, `::` paths, `;`-heavy one-liners where the wiki uses lines, explicit
  types everywhere.
- A special case kept alive by one test; behaviour only one odd spelling reaches.
- Tests of wording, exit codes, parse shapes or wasm bytes.

## Tests are spec only when backed
A test file's header names its source: a wiki page or a decision key. New cases only for forms a wiki page or a decision
shows; a new form needs the decision first, then the test.
