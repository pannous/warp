# Matching by type name (D5)

User decision 2026-10-03 (notes/open_decisions.md, D5): "General rule". Any noun can name a type or a parameter
(wiki/matching.md, wiki/type.md, wiki/signature.md). This note sets the rule for the open details: unknown words
(`photo`) and multi-word names. Status: approved by warp-43 (2026-10-03) as an assumption for the user to review;
implemented in src/lowering/types/type_name_matching.rs (`parameter_slots`, used by the `to` phrase in warp_parser.rs and by the spaced
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
   out of scope, an error "use one-word parameter names here" until then).
   Declared multi-word classes (card multi-word-class): `class full name {…}` declares the kebab name `full-name`, and the
   parser reads the spaced words `full name` as that one name wherever they stand in the same source (declaration_scan
   scan_multi_word_types, lists.rs joined_type_name), so `full name{given:"Ann"}` constructs one, `to greet a full name:`
   types its slot and names its parameter `full-name`, used in the body as `full name.given`. Only classes declared in
   the source join, so `full name` stays two words elsewhere (wiki/matching.md's "undesired matching from other contexts").
   Not yet: referring to the parameter by its head noun alone (`name.given`). Probe: probes/matching/multi_word_class.warp.
5. A slot that is a single known type word and the only parameter is also `it` (`fibonacci number`, `foo of int`).
6. Spaced heads without any known type word (`f x = …`) keep their old meaning; the `to` phrase always uses the slots.
8. Phrase-shaped definitions are ordinary functions and use explicit calls. Elsewhere `to` stays a range and `of`
   a field lookup.
7. One parse for every spelling: `=` and `:=` heads both go through `type_name_matching::spaced_parameters`
   (declarations::spaced_definition delegates to it), the `to` phrase takes a `:` or a `{…}` body. A lone type-word
   parameter is `it` only when the body does not use the word itself (`fibonacci number := … number … it …` keeps
   `number`, and `it` means the same value). Tests: tests/types/test_type_name_definition_forms.rs.

Object arguments (tests/functions/test_object_arguments.rs): an untyped parameter reads fields of the object it gets
(`measure(p) := p.width; measure({width:3})`), and an argument known at compile time not to be an instance of a
class-typed parameter is an error (`keep 3`, `keep(page{…})` for `keep(p:photo)`: "keep needs a photo for parameter p,
got 3 (an Int)"). An argument of unknown type, or a written map, is judged by the fields its uses read ("no field width").

Repeated type words (samples/types/polymorphism.warp, tests/types/test_repeated_type_parameters.rs): a type word repeated
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
| match-exhaustive | `match c { Penny: 1 }` over `enum Coin { Penny, Dime }`, class arms over a known class set | done: declared `c: Coin`, and `c = Dime` when every assignment of c is a case of one enum (exhaustive-assigned); other assigned values keep the run-time "no case" |
| enum-parameter-type | `f(c: Coin) := …` for `enum Coin` | "unknown type Coin of parameter c" |
| map-patterns | `match {name: "Ann", age: 3} { {name: n} => n }` | done (step 1 of tree patterns, below) |
| match-negative-subject | `match -3 { … }` | parsed as `match - 3` |
| multi-word-class | `class full name { first; last }` (D5 rule 4) | done: rule 4 above |
| matches-infix | `use regex; "abc" matches "a.c"` | done: lib/regex.warp declares `infix operator matches`; the parser scans the operators of used std modules too (declaration_scan.rs used_std_modules); gives 1/0 like `matches()` |
| multi-word-class | `class full name { first; last }` (D5 rule 4) | done: rule 4 above; `n.first` read the library word first until field-before-library-word (a known instance's field wins) |
| matches-infix | `use regex; "abc" matches "a.c"` | "matches needs 2 arguments" (infix use of a library word) |
| render-as | `to render text as pdf: …; render "hello" as pdf` (Features.md, D10) | done: notes/dispatch.md, probes/matching/render_as.warp |
| gap-feeding | `to integrate a function from a to b …` fed by same-named locals (call.md) | parked with natural phrase calls; explicit calls name every argument |

Decisions (user, via warp-interviewer):
- In a case key `1 | 2`, `1 or 2` and `1, 2` are alternatives; a computed comparison goes in a guard. Alternatives
  bind no names.
- `int | text` / `int or text` in a case key is a type union, as is `x is (int or text)`; mixed `0 or text` means
  the value 0 or any text.
- `other` (like `_`, `default`, `else`) is a catch-all case key; a variable named other stays a variable.
- Definitions by cases `fib(0) := 0; fib(n) := …` have no diagnostic of their own (cut, cleanup-cuts-2): the last one
  wins, as any redefinition; write the cases as one match.
- Exhaustiveness: a match over a known enum or sealed set that misses a case is the compile error "match on Coin
  misses Dime"; an unknown subject type keeps the run-time "no case".

Done on branch matching (warp-semantics), all in src/lowering/control/switch.rs, one entry `pattern_test` shared with
parameter patterns: alternatives, type arms (`int =>`, `i: int =>`, `Circle =>`), ranges (`13 to 19` inclusive,
`13..20` exclusive, open `65..`), `other`/`else` catch-alls, tuple arms `(1, t)` (a round comma list matches by
shape like `[1, t]`), `xs is [_, 2]` and `if xs is [a, b] then b` (switch::is_pattern, called from type_tests: a list
with `_`, a rest or a name defined nowhere is a pattern, else `is` compares), and bare-name arms `name => "hi " + name`
(bind when the body uses the name; unused, `red: "fire"` stays the symbol case; a variable compares).
The whole matched value (user): no `@`/`as` binding. In an arm the subject's name (`match xs`), the word `value` (any
subject, innermost match; a program variable named value wins) and `match name = expr { … }` (assigns, then matches;
the name stays bound) give it.
Parameter patterns (card parameter-patterns, lowering/definitions/parameter_patterns.rs): `add_pair([a, b]) := a + b`,
`total(Point{x, y}) := x + y`. Each pattern parameter gets a numbered name (`list·1`, `point·1: Point`), and the body
becomes a match on it, so another shape is the match's "no case" error and a non-Point the call's type error.
Probe: probes/matching/parameter_patterns.warp. Open: a user `sum(p: Point)` called with 5 runs the library sum
(card sum-point; its error special case was removed by cleanup-sum-point, a general fix belongs in overload dispatch).
Pattern names are declared `let` (switch::bind), the arm's own locals: no local-or-global question (card pattern-names).
Class arms `Point{x, y} =>` bind with `let` too (class_methods/destructuring.rs bound_parts, card
param-pattern-local-warning); that `let x = p.x; let y = …` once read as a Haskell definition named let
(declarations.rs haskell_definition: a declaration word is never a function name). Map parameters `greet({name}) :=`
take a `map·N` parameter like list ones (card param-map-pattern).
`else =>` on its own line is the catch-all arm (card match-arm); `else:` on its own line in a switch still fails to
parse (it clashes with Python's block colon).
Item patterns (step 3, card list-pattern): `[[n, s]...]` in a list pattern's rest position matches when every item
matches; each name binds the list of its values (switch::every_item_test, a while loop over the rest slice).
Named patterns (step 4, card patterns-values, user: the keyword): `pattern Adult = {age: >= 18}`; `x is Adult`, arms
`Adult =>`, `it is Adult and Admin`; names inside bind. A plain `Name = …` is a value; one whose right side only a
pattern can be (`_`, `..`, an item pattern) is an error with the fix `pattern Name = …` (src/lowering/types/named_patterns.rs,
substitutes the pattern before lower_where).
Probes: probes/matching/arms.warp, whole_value.warp; inventory with a fresh build: 35 of 48 before tuples, is and
bare names.

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
4. Named patterns as values: `pattern Adult = {age: >= 18}`; `x is Adult`, `people where it is Adult and Admin`.
5. Descendants: `tree find pattern` matches at any depth (XPath `//`, jq `..`); `tree replace p with q` rewrites.
   find done (src/lowering/collections/tree_walks.rs): each `subject find P` (P a map, list or named pattern) becomes a call of a
   generated recursive walker `find·N`, defined from a source template; result: the one name's values, a map of the
   names, or the matching subtrees. Tests tests/control/test_tree_find.rs, probe probes/matching/tree_find.warp.
   Open: `found = [[…]] find P` parses as an assignment first (card found-find; its parser special case was removed by cleanup-found-find, the general fix belongs in assignment parsing).
   replace done (same file, renamed tree_walks.rs): `tree replace P with Q` rebuilds lists (bracketed/separated keep the
   bracket), maps (`keys`, `r[k] = …`) and keys (`key(left, op, right)`), children first, then rewrites the node. Data
   and code alike: `code replace data($a * 1) with a`. In a function body the words come in nested pairs and `with Q`
   as a group: `flattened` splices them. Tests tests/control/test_tree_replace.rs, probe probes/matching/tree_replace.warp.
6. Schemas: `json(text) is Config` with `pattern Config = {port: int, host: text, debug?: bool}`. Done (card schemas):
   a map pattern whose field is a type word or optional is a pattern for `is` (switch.rs has_open_part; in list
   patterns type words still compare as values). `debug?: bool` parses as the elvis `debug ? debug : bool`, recognised
   by type_kinds.rs optional_field (shared with type bodies): missing, or there and matching. Fields not named are
   allowed, as in every map pattern. Note `{port: p, host: h}` compares host with the unit h (3600), no binding.
   Tests tests/control/test_schemas.rs, probe probes/matching/schemas.warp.
7. Code patterns: quoted code with holes, `data($a + 0) => a`, matched against `parse("…")` or `data (…)` (card
   parse-word, notes/parse_word.md). Backticks stay template text.
   code-holes done (switch.rs code_tests): `$a` binds the subtree, a repeated hole tests equality with the first, a key
   tests `is key` and its op, a list `is list` and its count; `$f($x)` is flattened like a quoted call `g x`. A match
   in a function copies its subject: indexing the copy guesses the parameter a list, and widen_parameters
   (user_functions.rs) drops that guess when only keys (code) are passed (card alias-indexed-param). Tests
   tests/control/test_code_holes.rs, probe probes/matching/code_holes.warp.
   Parsed code (2026-10-13): code patterns match `parse(t)` of a text known only at run time as they match `data (…)`.
   Quoted code of several statements stays as written: blocks.rs (lowering/control) bound `k: n * 1` of
   `parse("j: 1\nk: n * 1")`, a constant text parsed at compile time, as a block (warning, no field name); it now
   leaves quoted data alone, as the other passes do since data-quote-lowered. Open: `data (j: 1⏎ k: 2)` keeps its `(` bracket,
   `parse("j: 1\nk: 2")` has none, so the two lists differ while their items are equal. Test
   tests/control/test_parsed_code_patterns.rs.
Beyond Rust: Rust patterns are flat per level, need every struct field or `..`, cannot match at depth, cannot be
stored or combined, and cannot match code (macros aside).
