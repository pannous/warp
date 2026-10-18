# Matching by type name (D5)

Any noun can name a type or a parameter (D5; wiki/matching.md, wiki/type.md, wiki/signature.md), including unknown
words (`photo`) and multi-word names. src/lowering/types/type_name_matching.rs (`parameter_slots`, used by the `to`
phrase in warp_parser.rs and by spaced heads), tests/types/test_type_name_matching.rs.
Spaced heads with a type word before the name (`fib int i = …`) define no typed functions: write `fib(i:int) := …`.

## Forms covered

| written | parameters (name : type) |
|---|---|

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
   Declared multi-word classes: `class full name {…}` declares the kebab name `full-name`, and the
   parser reads the spaced words `full name` as that one name wherever they stand in the same source (declaration_scan
   scan_multi_word_types, lists.rs joined_type_name), so `full name{given:"Ann"}` constructs one, `to greet a full name:`
   types its slot and names its parameter `full-name`, used in the body as `full name.given`. Only classes declared in
   the source join, so `full name` stays two words elsewhere (wiki/matching.md's "undesired matching from other contexts").
   Not yet: referring to the parameter by its head noun alone (`name.given`). Tests: tests/types/test_multi_word_class.rs.
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

A type word repeated alone in a head (`combine float with float`) is the error "two parameters are named float".

# Pattern matching (topic card `matching`)

Inventory: probes/matching/inventory.sh runs every matching form of the spec (wiki/matching.md, pattern-matching.md,
switch.md, call.md, for.md, constructor.md, indexing.md, Features.md) and of the C++ wasp (`patterns` there are `[…]`
selectors: `y[1]`, `{a:1 b:2}[a]`, plus signature variant matching).

Works: switch/match on literals, symbols and `default`/`_`/`other`/`else`; map selection `{a:1 b:2}(a)` and `[x]`;
list patterns with wildcards, nesting, pairs and rests; guards `n if n < 0`; relational arms `> 12`; class patterns
`Point{x, ..}`; enum cases with and without values (`Shape::Circle(r)`, P178) and enum parameter types; destructuring
`[a, b] =`, `(a, b) =`, `{x, y} = p`; matching by type name (D5, above); dispatch by argument type and by return type
(D10, notes/dispatch.md); `for friend in xs` filters (P46); `xs is [1, 2]` as equality; `matches` as an infix of
lib/regex.warp (the parser scans the operators of used std modules, declaration_scan.rs used_std_modules).
Parked: natural phrase calls fed by same-named locals (`to integrate a function from a to b …`, call.md).

Decisions:
- In a case key `1 | 2`, `1 or 2` and `1, 2` are alternatives; a computed comparison goes in a guard. Alternatives
  bind no names.
- `int | text` / `int or text` in a case key is a type union, as is `x is (int or text)`; mixed `0 or text` means
  the value 0 or any text.
- `other` (like `_`, `default`, `else`) is a catch-all case key; a variable named other stays a variable.
- Definitions by cases `fib(0) := 0; fib(n) := …` have no diagnostic of their own (cut, cleanup-cuts-2): the last one
  wins, as any redefinition; write the cases as one match.
- Exhaustiveness: a match over a known enum or sealed set that misses a case is the compile error "match on Coin
  misses Dime"; an unknown subject type keeps the run-time "no case".

All in src/lowering/control/switch.rs, one entry `pattern_test` shared with parameter patterns: alternatives, type arms (`int =>`, `i: int =>`, `Circle =>`), ranges (`13 to 19` inclusive,
`13..20` exclusive, open `65..`), `other`/`else` catch-alls, tuple arms `(1, t)` (a round comma list matches by
shape like `[1, t]`), `xs is [_, 2]` and `if xs is [a, b] then b` (switch::is_pattern, called from type_tests: a list
with `_`, a rest or a name defined nowhere is a pattern, else `is` compares), and bare-name arms `name => "hi " + name`
(bind when the body uses the name; unused, `red: "fire"` stays the symbol case; a variable compares).
The whole matched value: no `@`/`as` binding. In an arm the subject's name (`match xs`), the word `value` (any
subject, innermost match; a program variable named value wins) and `match name = expr { … }` (assigns, then matches;
the name stays bound) give it.
Parameter patterns (lowering/definitions/parameter_patterns.rs): `add_pair([a, b]) := a + b`,
`total(Point{x, y}) := x + y`. Each pattern parameter gets a numbered name (`list·1`, `point·1: Point`), and the body
becomes a match on it, so another shape is the match's "no case" error and a non-Point the call's type error.
Tests: tests/functions/test_parameter_patterns.rs. Open: a user `sum(p: Point)` called with 5 runs the library sum
(card sum-point; the general fix belongs in overload dispatch).
Pattern names are declared `let` (switch::bind), the arm's own locals: no local-or-global question.
Class arms `Point{x, y} =>` bind with `let` too (class_methods/destructuring.rs bound_parts; declarations.rs
haskell_definition: a declaration word is never a function name). Map parameters `greet({name}) :=`
take a `map·N` parameter like list ones.
`else =>` on its own line is the catch-all arm; `else:` on its own line in a switch still fails to
parse (it clashes with Python's block colon).
Item patterns: `[[n, s]...]` in a list pattern's rest position matches when every item
matches; each name binds the list of its values (switch::every_item_test, a while loop over the rest slice).
Named patterns: `pattern Adult = {age: >= 18}`; `x is Adult`, arms
`Adult =>`, `it is Adult and Admin`; names inside bind. A plain `Name = …` is a value; one whose right side only a
pattern can be (`_`, `..`, an item pattern) is an error with the fix `pattern Name = …` (src/lowering/types/named_patterns.rs,
substitutes the pattern before lower_where).
Tests: tests/control/test_match_*.rs.

## Tree patterns (beyond what Rust offers)
Structural matching of whole trees, data (JSON-like maps and lists) and code alike. Design examples:
probes/matching/tree_patterns.warp. Built:
1. Map patterns: deep partial map patterns `{status: 200, body: {user: {name}}}`, fields not named are ignored.
   In arms and `x is {…}` (with a name to bind). switch.rs pattern_tests: the value must be a map or a pair (a
   one-field map is a pair), each named field there (`collection_contains`), read by subscript `path["name"]`.
   A bare `{a, b} => …` is an arm now; the JS object-parameter lambda needs its parentheses, `({a, b}) => …`.
   Tests: tests/control/test_match_maps.rs.
2. Deep conditions: conditions at any depth: `{age: >= 18}`, `{age: 13 to 17}`, `{name: text}`, `{code: 301 | 302}`,
   in lists too (`[> 3, text]`). switch.rs `condition` is the one test that binds nothing (comparison, type word,
   range, alternatives), used by a whole arm and by every part (`Matched::pattern`); a comparison makes `x is {…}` a
   pattern. A shorthand field `{text}` binds, never tests the type. Tests: tests/control/test_match_deep_conditions.rs.
3. Item patterns over lists: `{items: [{sku}...]}` binds `sku` to the list of every item's sku.
4. Named patterns as values: `pattern Adult = {age: >= 18}`; `x is Adult`, `people where it is Adult and Admin`.
5. Descendants: `tree find pattern` matches at any depth (XPath `//`, jq `..`); `tree replace p with q` rewrites.
   find (src/lowering/collections/tree_walks.rs): each `subject find P` (P a map, list or named pattern) becomes a call of a
   generated recursive walker `find·N`, defined from a source template; result: the one name's values, a map of the
   names, or the matching subtrees. Tests tests/control/test_tree_find.rs.
   Open: `found = [[…]] find P` parses as an assignment first (card found-find; the fix belongs in assignment parsing).
   replace (same file): `tree replace P with Q` rebuilds lists (bracketed/separated keep the
   bracket), maps (`keys`, `r[k] = …`) and keys (`key(left, op, right)`), children first, then rewrites the node. Data
   and code alike: `code replace data($a * 1) with a`. In a function body the words come in nested pairs and `with Q`
   as a group: `flattened` splices them. Tests tests/control/test_tree_replace.rs.
6. Schemas: `json(text) is Config` with `pattern Config = {port: int, host: text, debug?: bool}`. Built:
   a map pattern whose field is a type word or optional is a pattern for `is` (switch.rs has_open_part; in list
   patterns type words still compare as values). `debug?: bool` parses as the elvis `debug ? debug : bool`, recognised
   by type_kinds.rs optional_field (shared with type bodies): missing, or there and matching. Fields not named are
   allowed, as in every map pattern. Note `{port: p, host: h}` compares host with the unit h (3600), no binding.
   Tests tests/control/test_schemas.rs.
7. Code patterns: quoted code with holes, `data($a + 0) => a`, matched against `parse("…")` or `data (…)` (card
   parse-word, notes/parse_word.md). Backticks stay template text.
   Code holes (switch.rs code_tests): `$a` binds the subtree, a repeated hole tests equality with the first, a key
   tests `is key` and its op, a list `is list` and its count; `$f($x)` is flattened like a quoted call `g x`. A match
   in a function copies its subject: indexing the copy guesses the parameter a list, and widen_parameters
   (user_functions.rs) drops that guess when only keys (code) are passed. Tests
   tests/control/test_code_holes.rs.
   Parsed code: code patterns match `parse(t)` of a text known only at run time as they match `data (…)`. Quoted
   code of several statements stays as written (lowering passes leave quoted data alone). Open: `data (j: 1⏎ k: 2)` keeps its `(` bracket,
   `parse("j: 1\nk: 2")` has none, so the two lists differ while their items are equal. Test
   tests/control/test_parsed_code_patterns.rs.
Beyond Rust: Rust patterns are flat per level, need every struct field or `..`, cannot match at depth, cannot be
stored or combined, and cannot match code (macros aside).

## Pattern clauses
Definitions of one name and arity, one of them with a pattern parameter, are one function trying them in order
(src/lowering/definitions/pattern_clauses.rs, a SUGAR pass before parameter_patterns): `fib(0) := 0; fib(n) := …` is
`fib(n·1) := match n·1 { 0: 0; _: … }`; several parameters match as one list `[x·1, y·2]`. A plain name is `_` in its
arm and its body reads the numbered parameter, so a global of that name never turns the arm into a comparison (a
bare `n` arm compares when n is a global). A clause of plain names only is the `_` arm; clauses after it are never
reached and dropped (match puts `_` last wherever it stands). Typed parameters (`f(x: int)`) keep overloading by type:
a group holding one is not joined. All-wildcard list arms `[_, _]` read as a partial application, hence the bare `_`.
Test tests/functions/test_pattern_clauses.rs.

## Roadmap: compiler-grade patterns (for the second, pattern-based pass over the self-hosted compiler)
Goal: parser, passes, analyzer and emitter written in warp read as rule sets over node shapes, more elegant than the
Rust compiler. Probe of the gaps: probes/self_hosting/code_patterns.warp. Steps in order; ★ = a language decision
(notes/decisions.md: clauses in order, binding alternatives, `as` inside patterns, `+` text patterns).

| step | card | what | today |
|---|---|---|---|
| 1 | code-patterns | rest holes `g($first, $rest...)`, `[$h, $t...]`; statement patterns `for $v in $it { $body }`, `if $c { $a } else { $b }`; operator holes `$l $op $r`; a `for` inside a `data(…)` pattern must stay sealed | no match; the for runs |
| 2 | typed-holes | conditions on holes: `$r: int`, `$n: > 0`, `$s: symbol`, `$b: block` | no match |
| 3 | patterns-as-values | `n matches p` → bindings or nothing; `p: pattern` parameters; `find`/`replace` as lib warp (notes/pattern_values.md route B) | parse error |
| 4 | rewrite-rules | a rule set as a value, `rules { data($a * 1) => a; data($a + 0) => a }`, applied bottom-up or to a fixpoint, keeping positions: the vocabulary of a lowering pass | single `replace … with` only |
| 5 ★ | pattern-clauses | a function defined by pattern clauses `simplify(data($a * 1)) := a` … tried in order (decided) | done (pattern_clauses.rs) |
| 6 ★ | binding-alternatives | alternatives that bind the same names `data($k: int + $x) \| data($x + $k: int) => x` | error "alternatives bind no names" |
| 7 ★ | whole-binding | `as` binds a (sub)pattern: `data($f($inner) as call)` | parse error |
| 8 ★ | text-patterns | text patterns for the lexer: `"0x" + rest`, prefix/suffix, character classes | done (Text patterns below) |
| 9 | node-exhaustive | exhaustiveness over the Node kinds and sealed key ops, so a pass's match misses no shape | runtime "no case" |
| 10 | pattern-dispatch-speed | many-arm matches compiled to one decision tree (switch on kind/op first); measured on the passes | arm by arm |
| 11 | patterns-second-pass | rewrite lib/compiler (passes first, then parser, analyzer, emitter) as rule sets with 1–10; judged by the warp-pipeline-suite count and line count | — |

## Text patterns (card text-patterns, step 8)
A sum of text pieces and holes is a pattern in `match`/`switch` arms and after `is`: `"0x" + rest`, `name + ".txt"`,
`"/users/" + u + "/posts/" + p`. A hole is a name defined nowhere or `_`; a name the program defines stays a value
(`p + "ff"` compares). Between two holes a piece must stand (loud error otherwise), so the cut is unambiguous: the
prefix and suffix first, then each middle piece at its first occurrence.
A hole's type: a reading converts the text (`id: int`, `integer`, `number`, `float`; no match when the text isn't
one), a class tests its characters (`digit`, `letter` one character; `digits`, `letters`, `hex`, `space`, `word` one
or more; interviewer's decision A). These words are hole types only in that position, never reserved type words.
Rust: src/lowering/control/text_patterns.rs builds `count(text_holes(s, pieces, types)) > 0` and binds
`id = int(text_holes(…)#1)`; the cutting and class tests are warp, lib/prelude.warp text_holes / fits_text_type, loaded
through needed_definitions IMPLIED_WORDS. A non-text subject simply doesn't match.

