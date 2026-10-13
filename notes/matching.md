# Matching by type name (D5)

User decision 2026-10-03 (notes/open_decisions.md, D5): "General rule". Any noun can name a type or a parameter
(wiki/matching.md, wiki/type.md, wiki/signature.md). This note sets the rule for the open details: unknown words
(`photo`) and multi-word names. Status: approved by warp-43 (2026-10-03) as an assumption for the user to review;
implemented in src/lowering/type_name_matching.rs (`parameter_slots`, used by the `to` phrase in warp_parser.rs and by the spaced
`f T x = …` form), tests/types/test_type_name_matching.rs.

## Forms covered

| written | parameters (name : type) |
|---|---|
| `fib int i = …` | `i : int` |
| `fibonacci number = …` | `number : number` (also `it`) |
| `foo of int = it + it` | `int : int`, used as `it` |
| `square of a number = it*it` | `number : number` |
| `to square a number: it*it` | `number : number` |
| `to add number a to number b: a+b` | `a : number`, `b : number` |
| `to add a to b: a+b` | `a`, `b` untyped |
| `to assign a photo to a contact: contact.image = photo` | `photo : photo`, `contact : contact` |
| `to integrate a function from a to b …` | `function : function`, `a`, `b` |

## The rule

1. A definition head splits into slots at prepositions (`to of from with in into at by for on`, never the first word,
   which is the verb or name); in the plain form `f T x = …` each word run without prepositions is one slot.
2. A slot is `[article] words…`. An article (`a an the`) counts only when another word follows it in the same slot
   and that word is a known type or the body never uses the article as a name: `a` in `to add a to b` and
   `to add a b: a+b` is a name, in `to square a number: a*a` an article (so `a` is undefined there).
   After a type word an article is a name when a preposition or the end follows: `number a to number b`.
3. The last word of a slot names the parameter. The words before it are its type, written type-first as in
   `number a`. Without a written type, the name itself is matched as a type (matching by type name):
   - a known type (builtin `int number text …`, a plural `numbers`, a declared `class`/`type`/`struct`) types it;
   - an unknown noun (`photo` with no `class photo`) is just a name, untyped (inferred as any other parameter).
     Never an error: declaring `class photo` later makes the same signature typed. Two definitions that differ
     only by unknown nouns (`to kill a person`, `to kill a dog` without classes) are a loud redefinition:
     "kill is defined twice; declare class person and class dog to dispatch on them".
4. Multi-word names: a slot whose words match a declared multi-word class (`class full name {…}`) as a whole is
   typed by that class; matching is longest-first over the declared classes in scope, so it never reaches into
   another module's names unless imported. Otherwise the head noun (last word) decides: `phone number` is typed
   number, `bar name` is typed name. A multi-word parameter is referred to in the body by its head noun when that is
   unique in the signature (`first name`, `last name` both need the full phrase, which needs multi-word identifiers:
   out of scope, an error "use one-word parameter names here" until then). Not implemented yet: matching declared
   multi-word classes, because `class full name {…}` cannot be declared yet (the parser takes one word after `class`).
5. A slot that is a single known type word and the only parameter is also `it` (`fibonacci number`, `foo of int`).
6. Spaced heads without any known type word (`f x = …`) keep their old meaning; the `to` phrase always uses the slots.
8. Calls with the phrase's prepositions (P52, user): `add 1 to 2`, `square of 4`, `move 1 from 2 to 3`
   (src/lowering/phrase_calls.rs, the first source pass). Elsewhere `to` stays a range and `of` a field lookup.
7. One parse for every spelling: `=` and `:=` heads both go through `type_name_matching::spaced_parameters`
   (declarations::spaced_definition delegates to it), the `to` phrase takes a `:` or a `{…}` body. A lone type-word
   parameter is `it` only when the body does not use the word itself (`fibonacci number := … number … it …` keeps
   `number`, and `it` means the same value). Tests: tests/types/test_type_name_definition_forms.rs.

Object arguments (tests/functions/test_object_arguments.rs): an untyped parameter reads fields of the object it gets
(`measure(p) := p.width; measure({width:3})`), and an argument known at compile time not to be an instance of a
class-typed parameter is an error (`keep 3`, `keep(page{…})` for `keep(p:photo)`: "keep needs a photo for parameter p,
got 3 (an Int)"). An argument of unknown type, or a written map, is judged by the fields its uses read ("no field width").

Repeated type words (samples/polymorphism.warp, tests/types/test_repeated_type_parameters.rs): a type word repeated
alone in a head numbers its parameters, `combine float with float = float#1 + float#2` is
`combine(float·1:float, float·2:float) := float·1 + float·2`; `$0`, `$1` read them by position as in any definition.
Other repeated names stay the error "two parameters are named …" (type_name_matching.rs numbered_repeated_types).

# Roadmap (card matching-create, warp-semantics 2026-10-13)

Topic card `matching` (Roadmaps). Inventory: probes/matching/inventory.sh runs every matching form of the spec
(wiki/matching.md, pattern-matching.md, switch.md, call.md, for.md, constructor.md, indexing.md, Features.md) and of
the C++ wasp (`patterns` there are `[…]` selectors: `y[1]`, `{a:1 b:2}[a]`, plus signature variant matching).
On main 07aa204b0: 21 of 41 forms work.

Works: switch/match on literals, symbols and `default`/`_`; map selection `{a:1 b:2}(a)` and `[x]`; list patterns
with wildcards, nesting, pairs and rests; guards `n if n < 0`; relational arms `> 12`; class patterns `Point{x, ..}`;
enum cases with and without values (`Shape::Circle(r)`, P178); destructuring `[a, b] =`, `(a, b) =`, `{x, y} = p`;
matching by type name (D5, above); dispatch by argument type and by return type (D10); `for friend in xs` filters
(P46); `xs is [1, 2]` as equality.

Gaps, most important first (silent wrong answers lead: they quietly compensate instead of failing loudly):

| card | written | now |
|---|---|---|
| match-alternatives | `match 2 { 1 \| 2 => …; 1 or 2 => …; 1, 2 => … }` | `\|`/`or` silently miss (the subject is compared with the logical value), `,` fails to parse |
| match-ranges | `match 15 { 13..19 => "teen" }`, `13 to 19` | silently misses |
| match-tuples | `match p { (1, t) => t }`, `(a, b) =>` | `(1, t)` silently misses, `(a, b)` "undefined variable: a" |
| is-pattern | `xs is [_, 2]`, `if xs is [a, b] then b` | `_` undefined; the binding form silently gives the else branch |
| definition-cases | `fib(0) := 0; fib(1) := 1; fib(n) := …` | the last definition replaces the others: stack exhausted |
| match-binding-arm | `match "bob" { "alice" => …; name => "hi " + name }`, `all @ [a, b] =>` | a bare name is compared with an undefined variable; `@` unexpected |
| match-type-arms | `match v { int => …; i: int => …; Circle => … }` | no case / undefined variable |
| parameter-patterns | `add_pair([a, b]) := a + b`, `sum(Point{x, y}) := x + y` | undefined variable / unknown type |
| match-exhaustive | `match c { Penny: 1 }` over `enum Coin { Penny, Dime }`, class arms over a known class set | only the run-time "no case"; aligned with tagged-unions (warp-dispatch, parser-features) |
| enum-parameter-type | `f(c: Coin) := …` for `enum Coin` | "unknown type Coin of parameter c" |
| map-patterns | `match {name: "Ann", age: 3} { {name: n} => n }` | done (step 1 of tree patterns, below) |
| match-negative-subject | `match -3 { … }` | parsed as `match - 3` |
| multi-word-class | `class full name { first; last }` (D5 rule 4) | the parser takes one word after class |
| matches-infix | `use regex; "abc" matches "a.c"` | "matches needs 2 arguments" (infix use of a library word) |
| render-as | `to render text as pdf: …; render "hello" as pdf` (Features.md, D10) | silently `int` |
| gap-feeding | `to integrate a function from a to b …` fed by same-named locals (call.md) | "needs a value for parameter a" |

Design defaults sent to warp-interviewer (undoable, implemented as written until answered):
- In a case key `1 | 2`, `1 or 2` and `1, 2` all mean alternatives (Rust's `|`); `|` stays logical elsewhere.
- Definitions by cases: definitions whose parameters are literals or patterns join one function; a call tries the
  most specific first (literal before pattern before plain name), like dispatch, whatever the written order.
- Exhaustiveness: a match over a subject of known enum or sealed class set without `_`/`default` that misses a case
  is a compile error naming the missing cases; an unknown subject type keeps the run-time "no case".

## Tree patterns (card tree-patterns, user 2026-10-13: "go beyond what Rust offers")
Structural matching of whole trees, data (JSON-like maps and lists) and code alike. Design examples:
probes/matching/tree_patterns.warp. Steps, each its own sub-card of matching:
1. map-patterns (done): deep partial map patterns `{status: 200, body: {user: {name}}}`, fields not named are ignored.
   In arms and `x is {…}` (with a name to bind). switch.rs pattern_tests: the value must be a map or a pair (a
   one-field map is a pair), each named field there (`collection_contains`), read by subscript `path["name"]`.
   A bare `{a, b} => …` is an arm now; the JS object-parameter lambda needs its parentheses, `({a, b}) => …`.
   Tests: tests/control/test_match_maps.rs; probe probes/matching/map_patterns.warp.
2. deep-conditions (done): conditions at any depth: `{age: >= 18}`, `{age: 13 to 17}`, `{name: text}`, `{code: 301 | 302}`,
   in lists too (`[> 3, text]`). switch.rs `condition` is the one test that binds nothing (comparison, type word,
   range, alternatives), used by a whole arm and by every part (`Matched::pattern`); a comparison makes `x is {…}` a
   pattern. A shorthand field `{text}` binds, never tests the type. Tests: tests/control/test_match_deep_conditions.rs;
   probe probes/matching/deep_conditions.warp.
3. Item patterns over lists: `{items: [{sku}...]}` binds `sku` to the list of every item's sku.
4. Named patterns as values: `Adult = {age: >= 18}`; `x is Adult`, `people where it is Adult and Admin`.
5. Descendants: `tree find pattern` matches at any depth (XPath `//`, jq `..`); `tree replace p with q` rewrites.
6. Schemas: `json(text) is Config` with `Config = {port: int, host: text, debug?: bool}`.
7. Code patterns: quoted code with holes, `` `$a + 0` => a ``; needs card parse-word (real Nodes of a source).
Beyond Rust: Rust patterns are flat per level, need every struct field or `..`, cannot match at depth, cannot be
stored or combined, and cannot match code (macros aside).
