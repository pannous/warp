# Taste: Warp is beautiful

Judge a design, a test or a sample by these examples, not by whether it merely works.

## Beautiful
- Warp
- Units are values: `sleep(3ms)`, `5km == 5000m`, `90km/h`: 
- implicit multiplication `5km == 5 * km`
- English-like yet precise general forms: 
`square of number = it*it`
`for (dx, dy) in moves`
`for key in map` ≠ `for k,v in map`
`for char in text` ≠ `for byte in text`
`xs from #2 to #4`
`if no x`, `x is empty`.
- One general rule that holds for its siblings: every type, every operator, not the one example at hand.

## Ugly
- Rust
- Rust made verbose ugliness a first class citizen.
```pub fn horror()->Result{Ok(Result(mut &self))}```

## Bad practices in compiler
- One rewrite rule per name: `rewrite $k => "Escape" where $k in [ESC, ESCAPE, Escape, esc, escape]`; vocabulary is an enum in lib/.
- Magic words and glue names visible to programs or pinned by tests: `ran·without·error`, `(on··every··0)(t)`.
- `person#1` instead of person.name 
- A sugar rule that drops meaning: `300ms` → `0.3`, the unit lost.
- Rust-isms: `is!("3+3", 6)` call syntax in code, `::` paths, `;`-heavy one-liners where the wiki uses lines, explicit types everywhere.
- A special case kept alive by one test; behaviour only one odd spelling reaches.

## General
We are should sometimes be allowed to sacrifice maximum beauty if it makes the parser much cleaner and if we can use existing warp mechanisms instead of creating special keywords and so on.

- Plain warp over special syntax: `on escape { quit() }` is a call of a library function `on(key: Key, handler: block)`;
-  `escape` is found as enum `Key.escape` because the parameter wants a Key (names from the expected type, wiki/inventions.md ).

## Tests are spec only when backed
A test file's header names its source: a wiki page or a decision key. New cases only for forms a wiki page or a decision
shows; a new form needs the decision first, then the test.

## Familiar conventions first
Where warp has no stated reason to differ, the meaning people know from shells and mainstream languages wins over a convenience for samples
-relative paths resolve against the current folder
