# Matching by type name (D5)

User decision 2026-10-03 (notes/open_decisions.md, D5): "General rule". Any noun can name a type or a parameter
(wiki/matching.md, wiki/type.md, wiki/signature.md). This note sets the rule for the open details: unknown words
(`photo`) and multi-word names. Status: proposal sent to warp-43; only the known-type-word cases are implemented.

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
2. A slot is `[article] words…`. An article (`a an the`) counts only when another word follows it in the same slot,
   so `a` in `to add a to b` is a name.
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
5. A slot that is a single known type word and the only parameter is also `it` (`fibonacci number`, `foo of int`).
