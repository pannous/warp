# Wiki feature survey: row index

A survey of `wiki/` against the binary numbered the features the spec plans (rows 1-40), the questions it raised
(D1-D16) and the ideas too vague or big to build (T1-T12). Notes, tests and cards cite these numbers; the full survey
with examples and the original findings is notes/OLD/wiki_features.md.

## Rows: all implemented except 38 and 40
| # | feature | limits that remain |
|---|---|---|
| 1 | `until`, `unless`, `N times`, trailing `if`/`while` | |
| 2 | `!` evaluates a block | |
| 3 | `∨`, `nand`, `¬&`, `⊼` | |
| 4 | mutating method words `x.upper`, `x.uppercase!` | |
| 5 | basic text and list words (`upper`, `reverse`, `first`, `sort`, `sum`, `join`, `split`) | |
| 6 | block comments `/# … #/`, doc comments `##` `/** */`, line continuation `\` | |
| 7 | property access `p.b.c`, `name of p`, `p's name` | |
| 8 | assignment to a property and nested paths | |
| 9 | safe navigation `?:` and `?.` | |
| 10 | `count bytes of "abc"`, `count chars of`, `size of text in bytes` | not `count footprint of`, `count codepoint1 in` |
| 11 | `hex 1010` literal keyword | |
| 12 | `function name(args){body}`, `to name args:` | typed phrases: notes/matching.md |
| 13 | `f = {it*2}; f 3` | |
| 14 | `final`, `constant`, `val` | |
| 15 | `do <block>`, `!!` | |
| 16 | `x is int`, `x is number 9`, `x as number = 9` | |
| 17 | superscript exponents and suffix words | |
| 18 | `switch` / `match` with default | |
| 19 | loop forms, slices `xs[a..b]`, `xs[a:b]` | |
| 20 | typed and fixed arrays `char[3]` | see notes/library_words.md |
| 21 | units: comparison, rates, products `3 m * 2 m`, powers | notes/units_runtime.md |
| 22 | first-class operators and functions `fold +` | |
| 23 | anonymous functions `x=>x*x`, `{it*it}` | |
| 24 | function broadcasting on lists and pairs | notes/broadcasting.md |
| 25 | type-name-as-parameter definitions `fibonacci number = …` | notes/matching.md |
| 26 | multiple dispatch by argument type | notes/dispatch.md |
| 27 | optional test words `empty`, `missing`, `failed`; unwrap `x!` | notes/unwrap.md |
| 28 | `try … else …`, `assert cond else X` | notes/wasm_exceptions.md |
| 29 | `x!` mutating calls, `add x to list` | |
| 30 | struct / record / class construction and fields, enums | |
| 31 | string interpolation `"${expr}"`, `"\(x)"` | |
| 32 | text concatenation and `+=` on text | |
| 33 | `import` / `include` / `require` / `use <module>` | notes/wasm_modules.md, notes/packages.md |
| 34 | data as scope: kebab names and parent variables | |
| 35 | prefix/suffix/infix operator declarations with precedence | |
| 36 | hyperreal numbers ε, ω | notes/hyperreals.md |
| 37 | `\alpha` entities | |
| 38 | broadcast-free pipe and composition | became decision D6 |
| 39 | records equality `{x:3.0} == {x:3}` | |
| 40 | multi-value returns | an emission strategy, not a language feature (notes/multi_value.md) |

## Questions D1-D16
All decided; notes/decisions.md records each by its number (its index line maps D1 → P6, D8 → approx-zero, …).

## Too vague or too big (T)
T5 (structural pattern matching) is built and T6 (charged data, wiki/charged.md) decided. Still ideas: T1 `with context
do … end`, T3 `result` keyword, T4 gap filling (refused: a missing argument is a loud error), T7 uniscript in the IDE
(the format itself works: notes/uniscript.md), T8 geometric algebra, T9 scene graphics, T10 package infrastructure,
T11 meta properties and property setters on numbers, T12 open structs and ABI marshalling. T2 (signals, tasks, `go`):
notes/signals.md, notes/threads.md.
