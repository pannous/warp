# Code patterns and templates (quasi-quote)

The two halves of quoted code with holes, for passes written in warp (notes/self_hosting.md):

```warp
swap(c) := match c { data($a - $b) => data($b - $a); _ => c }   // pattern binds a, b; template splices them
tail(c) := match c { data($f($first, $rest...)) => data($f($rest...)); _ => c }
k = data(width); data(m.pop($(str(k))))                          // m.pop("width")
```

- `$a` in a pattern binds the subtree there (switch.rs code_tests), in a template splices the value of a.
  `$(expr)` splices the value of expr; `$xs...` a list's items (pattern: the slice of the remaining items).
- Which one: a pattern where a pattern stands, an arm's key with its guard and the right side of `is` / `==`
  (find/replace walkers are match arms by then); a template everywhere else. `x == data($a + 1)` therefore tests
  the shape; to compare with a template, put it on the left or in a variable.
- lowering/control/quasi_quote.rs builds a template right after tree_walks, before any pass renames a variable or
  reads captures (`xs.map(x => data($f($x)))`), with `key`, `bracketed` and `separated` (reflection/node_api.rs);
  parts without holes stay quoted. `$f($x)` builds the call shape of a quoted `g(x)`.
- Quoted code is sealed before the sugar passes (blocks::seal_quoted), the call form `data(e)` and the word form
  `data(a of b)` too, so `data(3 times {…})` stays the phrase (card data-pattern-sealed). The parser takes a hole
  as the operand of `n times $b`.
- A hole named like a unit (`$m`) is a variable for units.rs, also in its arm (card unit-hole-name).

Open (cards): quoted-number (`n = data(3); n` fails), replace-quoted-block (tree replace into `{…}` code),
replace-level (`x = A replace P with R`), built-key-print (built keys print `:`).
