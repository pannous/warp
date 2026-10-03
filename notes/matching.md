# Matching by type name (D5)

User decision 2026-10-03 (notes/open_decisions.md, D5): "General rule". Any noun can name a type or a parameter
(wiki/matching.md, wiki/type.md, wiki/signature.md). This note sets the rule for the open details: unknown words
(`photo`) and multi-word names. Status: approved by warp-43 (2026-10-03) as an assumption for the user to review;
implemented in src/type_name_matching.rs (`parameter_slots`, used by the `to` phrase in wasp_parser.rs and by the spaced
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

Object arguments (tests/functions/test_object_arguments.rs): an untyped parameter reads fields of the object it gets
(`measure(p) := p.width; measure({width:3})`), and an argument known at compile time not to be an instance of a
class-typed parameter is an error (`keep 3`, `keep(page{…})` for `keep(p:photo)`: "keep needs a photo for parameter p,
got 3 (an Int)"). An argument of unknown type, or a written map, is judged by the fields its uses read ("no field width").
