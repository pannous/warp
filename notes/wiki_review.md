# The wiki as specification: a review (2026-11-18)

By warp-architect-review, from three read-only sweeps of /Users/me/dev/angles/warp/wiki (277 files, about 18000
lines) by helper agents; the lines quoted under "Fundamentals" were checked by hand. Not every page was read.

Verdict: the wiki is six years of notebook with a specification inside it. The directive "read wiki/<topic>.md
before working on a topic" would today often lead a worker to a wrong, stale or self-contradicting page.

## Fundamentals stated two ways (the user decides; each then gets one page that wins)

| question | one place | another place |
|---|---|---|
| is a list shared after `b = a`? | list.md:272 shared like instances | Footguns.md:433 value semantics, nothing mutated in place; reference.md:57 |
| does `x!` run a block? | charged.md:193 runs nothing, unwraps an optional | charged.md:75 `w! // 9`; SPECIFICATION.md:119; data.md:39 |
| are `()`, `[]`, `{}` equal to ø? | null.md:80 no | Footguns.md:592 `()` is ø and ø is the empty list; list.md:261 |
| what does `is` mean? | assignment.md:38 always a comparison | =.md:3 also assignment; equality.md:10 the subtype test |
| what is `{…}`? | charged.md:162 a block | list.md:125 a list; null.md:82 a map; list.md:60 a column vector |
| how far does `f x …` reach? | precedence.md:9 up to `=` | function.md:29 depends on the parameter's type; Footguns.md:291 |
| two kinds in one variable | type.md:319 a compile error | notes/first_principles.md: the answer is `any` |
| which values are falsy? | truthiness.md:13 (0 included) | truthiness.md:42 `is empty` without 0; falsey.md:11; Error.md:172 |
| what is a character? | string.md:17 a grapheme | Footguns.md:751 a code point; char.md:3 char32_t |
| `"a" + 3` | string.md:110 "a3" | Footguns.md:443 a type error |
| how big is an int? | int.md 32 or 60 bits | Footguns.md:314 unbounded |
| what `T?` allows | null.md ø only | Error.md:170 ø or an Error |
| `s[0]` rewritten by the fixer | Normalization.md:111 to `s#0` | Footguns.md:416 `x#0` is an error (`#` counts from 1) |
| `in` | list.md:277 yes or no | indexing.md:27 the value |
| an enum case as a number | enum.md:22 Dime is 3 | enum.md:78 Dime is 2 |

Silent: how arguments are passed; when a field's default is evaluated and whether instances share it; whether a
line continues after a comma, an operator or an open bracket; what happens when an `any` meets a typed parameter;
how overloads are ranked (one sentence, function.md:105). No page for `any`, text, if, loops, modules, scope,
generics.

One spelling, several meanings: `as` (cast, conversion, result type, variable type, choice of an overload, a
counting mode of enums); `is`; `&` (four); a space between two words (depends on what the words name: unit.md:17,
function.md:155, constructor.md:16).

## Structure

- wiki/ is its own repository and git-ignored in warp: it does not exist in a worktree, where compiler work happens.
- Nothing marks a page as specification, idea, history or todo. SPECIFICATION.md has no incoming link and holds
  TODOs; the sidebar lists Todo, inventions, Bad and ideas beside spec pages.
- No status (built, planned, dropped) on any page; 57 ad hoc `# not yet:` lines; cards referenced in 56 pages.
- Old implementations as current: Normalization.md (`src/normalize.rs`, `cargo test`), standard-library.md (18
  Rust paths), type.md:317, struct.md, int.md. 77 pages say Angle or Wasp and never Warp; 78 unchanged since 2025.
- Duplicates without a winner: int/integer/number/overflow, char/string/unicode, struct/class/object,
  function/lambda/closure/block, use/import/include, evaluation/charged.
- Names: the text type is in string.md, evaluation time in charged.md; 33 stubs of three lines or fewer.
- Noise: wasp_documentation_2024.txt (9029 lines, hit by every grep), an editor workspace file, a symlink into a
  package cache with 44 more markdown files and its own AGENTS.md.
- About a third of the code examples state a result. Footguns.md has entries under "NOT YET" that read as working.
- cheats.md holds a GitHub URL with a `?token=GHSAT…`: a credential in a public repository.

## Recommendation

1. Seven pages that win, one per fundamental: values and sharing, equality, emptiness and truthiness, types and
   `any`, indexing, brackets, spaces and calls. Each states the rule, its examples with results, and nothing else.
2. A status line at the top of every page: `spec`, `idea` or `history`; ideas and history move to folders. The
   directive then reads: only `spec` pages bind; a missing rule is a question, not a licence.
3. Every example of a `spec` page states its result and is run against wap by the docs check: the wiki becomes
   wap's acceptance test, and a contradiction between two pages becomes a red line instead of a surprise.
4. Decisions of Footguns.md are folded into the topic's page and deleted there; the page of the topic wins.
5. The wiki is linked into every worktree; the noise files and the token leave the repository.
