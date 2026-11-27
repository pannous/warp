# Language decisions: an inbox

A language decision lands here as one line (≤160 chars): date, key, the rule, and the card that absorbs it. When that
card closes, the line is deleted. Its home is a wiki passage plus an outcome test; the wiki wins where they differ.
Only language decisions: core OKs, night decisions and one-moment rulings are notes on their card, process rules live
in AGENTS.md or a role file. Under 40 lines, nothing older than 7 days (card decisions-prune, warp-interviewer).
Grep a key or topic, never read it whole. Open questions: notes/open_decisions.md.

- 11-16 each-maps: each used as a value maps (`each xs {it*10}` → [10, 20, 30]); as a statement it loops.
- 11-16 class-name-patterns: `Name{…}` with Name a declared class matches/builds that class (`Circle{r} =>`); other words stay tags.
- 11-16 spaced-hash-count: a blank before `#` and none after is the count (`#letters`); the index is glued `xs#1`.
- 11-16 trailing-operator-indent: a line ending in an operator continues only onto a deeper-indented line.
- 11-17 pause-overload: `pause` alone is a rest Note (play(pause, 1s)); `pause job` holds a task: told apart by types.
- 11-17 range-value: every range is an immutable Range value (`1..5` shows 1..5); for/map/sum/xs[r] walk it, list(r) makes a list.
- 11-17 pattern-holes: pattern bodies bind only $names; a hole is typed by a shape (Node, Symbol, a pattern) or a value type (`$xs: list<int>`).
- 11-17 function-type-arrow: a function type is `(int, int) -> int`, `float -> float` for one parameter; no `=>` in types.
- 11-17 spaced-call-precedence: spaced arguments bind looser than every operator, tighter than = := ->: `square 3 * 3` is square(9).
- 11-17 kernel-forms: the reader's nodes are the data forms (`[a, b]` list, `{a: 1}` map); one Record form declares class, enum and `A | B`.
- 11-17 use-cycles: modules may use each other; each file is declared once and a cycle is cut where it closes.
- 11-18 type-words-travel: a call on a value of type A also finds what A's class and module declare taking an A.
- 11-18 bare-container-types: a bare `list`/`map`/`function` type takes the given value's type; fields write the full type.
- 11-18 instance-truthy: an instance is truthy unless its class defines `empty` (wiki/falsey.md).
- 11-19 kernel-try: Try(body, otherwise) catches a raise; one refused function then ends only itself.
- 11-19 eq-not-is: `==` is equality only (`3 == int` is no); `is` is the broader word (isa, can-be, equal).
- 11-19 module-clash: two used modules defining one word with the same parameter types is a loud clash naming both places.
- 11-19 atom-into-node: an atom given where a Node is wanted becomes its case: `n: Node = 'a'` is Char('a').
- 11-19 enum-case-unambiguous: a bare case name means the case only if nothing else of that name is in scope or the enum is expected.
- 11-20 empty-bracket-keeps-bracket: `()` is ø; `[]` and `{}` are an empty list and map, falsy, typed by their place (wiki node.md).
- 11-20 library-words-as-rules: words typing uses by name are ordinary rewrite rules in wap/sugar (`m#k = v` → set), no role annotations.
- 11-20 set-of-to: wap's map setter is `set(key, of: m, to: v)`, read `set k of: m to: v`.
- 11-20 chained-comparisons (night): `3>2>1` is `3>2 and 2>1`; `3>2 == true` stays one plain comparison (Footguns.md).
- 11-21 cross-file-run-order: a top-level read of a global another file sets later is refused; statements run in written order.
- 11-21 bare-kind-binds-alike: `==(a: enum, b: enum)` binds one type; two different enums are refused, an own `==(a: Side, b: Grouping)` allows.
- 11-21 compile-inputs (priority, wap#42): compiling needs only signatures, type declarations and rules; all bodies are compiled once (wap notes).
- 11-21 generic-fallback (wap#42): one compiled body per typed overload; a generic word gets one general fallback with run-time dispatch.
- 11-21 packed-library: packed warp (`pack`/`unpack`, tag byte per Node variant, back-refs) is a core library accessible everywhere 
- 11-22 variable-retyping: `x = 1; x = "a"` makes x any with a warning; let/const and written types (`w: int = 1`) keep theirs (wiki/type.md:316).
- 11-22 any-holds-container: `xs: any = [5, 6, 7]; print xs` prints [5, 6, 7]; generic words dispatch over boxed list/map types (card any-values; uniform-lists may replace it).
- 11-22 enum-case-as-type: a parameter may be one enum case, `first_size(d: Dot) := d.size`; it narrows and reads fields by name (wiki variant.md).
- 11-22 spaced-is-two (supersedes 11-12 spaced-data-call): `style {…}` is two items [style, {…}] with a did-you-mean; glued `style{…}` is the call.
- 11-22 list-variance: a list<Pair> may be passed where list<Node> is wanted (as built); a write breaking the caller's item type is refused when it appears.
- 11-22 any-takes-concrete: where the value is known, an any name takes the concrete type it holds: `c: any = "z"; c is text` → yes, as without any.
- 11-22 map-holds-anything: an untyped `{}` is map<any, any>: `m['a'] = 7; m[3] = 4` both work; uniform key types are a later optimization.
- 11-22 typed-operator-heads: `infix operator ⇔(a: bool, b: bool): bool := …` declares; an undeclared glyph `⊕(a: int) := a` is a plain name.
- 11-27 turnstile-aliases: `⊢` spells a law's parameter form inline (`law w: a ≤ b ⊢ a ≤ b + 1`); `⊨` `⊣` come too; ASCII spellings open. After T, with the sugar.
- 11-27 wap-verify-only: law.md and cli.md lose Lean and `wap types`; laws are checked by wap's own kernel through `wap verify` (wiki 596e330).
- 11-27 compiler-ints-fast: the compiler's own ints stay exact int in its source and build with `wap fast` (i64 storage), 0 source lines; replaces "the source writes i64".
