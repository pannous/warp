# Dispatch on return type (D10)

User decision (P17, notes/open_decisions.md): "Dispatch on return type" (un-parked). `render "hello" as pdf` and
`docx example = render "x"` pick the overload of `render` by the expected result type; when the expected type picks no
single overload, the first-declared one is taken with a got-it warning naming the explicit form.

The traits work (notes/traits.md) picks an operation by the type of its first argument; this picks a function by the type
its caller expects back. Both rename the variants to `name·T` (traits::witness_name) and resolve calls at compile time.

## Overloads
Two or more definitions of one name whose result types differ are overloads, in declaration order:
```wasp
class pdf{body}
class docx{body}
render(t):pdf  := pdf("%PDF " + t)      // the result type written after the head (also `render(t) as pdf := …`)
render(t):docx := docx("<w:t>" + t)
```
The result type is the written one, else the type of the instance the body constructs (`render(t) := pdf(…)`).
Definitions of one name whose result types are not all known stay what they were (the last one wins).

## The expected type of a call
| context                         | expected type |
|---------------------------------|---------------|
| `render "x" as pdf`, `(render "x") as pdf` | pdf  |
| `docx d = render "x"`, `d:docx = render "x"` | docx |
| `save(render "x")` with `save(d:docx)` | docx (the parameter's declared type) |
| anything else                   | none          |

A call with an expected type that one overload returns calls it (`render·pdf("x")`; the `as pdf` is then redundant and
dropped). Otherwise the first-declared overload is taken with the got-it warning (diagnostic::ask, Fallback::Warning,
topic `return-type`): `render has variants returning pdf, docx: which does render "x" mean? (taking pdf); fix: render "x" as pdf`.

## Lowering
src/overloads.rs, right after type_constructor (constructions are instances by then) and before the trait passes and
library_words, so `x = render "x" as pdf; x.body` reads a field: traits::InstanceTypes knows the result shape of a user
function from its body.
