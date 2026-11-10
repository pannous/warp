# Language layers: notation, kernel, sugar (card language-layers)

1. **notation**: the reader only, the same one warp data uses: atoms, keys (pairs, tags), groups `() [] {}`,
   comments, operators from one precedence table (extensible by `operator` declarations), juxtaposition; no keywords.
2. **kernel**: the forms the analyzer and emitter must know. Test: a form is kernel only if it cannot be a rewrite
   into other kernel forms.
3. **sugar**: everything expressible as `rewrite` rules (core/passes/rewrites.warp) into the kernel, one module per
   topic in extensions/sugar/*.warp loaded by the prelude. New syntax extends the notation generally (a table row, an
   operator or alias declaration), never a parser keyword branch.

## The kernel

| # | form | why it can't be a rewrite |
|---|------|---------------------------|
| 1 | `name(params) := body`: definition (functions, constants), parameters with type and default | binds names |
| 2 | `place = value`: assignment to a name, `x.f`, `x#i` | the effect itself |
| 3 | `f(args)` call, `x.f` member, `x#i` element | the primitive operations |
| 4 | `params => body`: function literal | closure conversion needs free-variable capture (lambdas.warp) |
| 5 | `if c then a else b` (also the value form) | the one branch |
| 6 | `while c do body`, `break`, `continue`, `return` | the one loop and its jumps |
| 7 | `class Name {fields}` with variants: the one type declaration | declares a type |
| 8 | `use module` | loads code |
| 9 | `try x else y`: the error guard | wasm try_table, emitted specially |
| 10 | `global` | scope, not a value |
| 11 | `data x`: quote | stops evaluation; the code-as-data door |
| 12 | `rewrite p => t`, `operator …` declarations | the meta forms that build layer 3 and extend layer 1 |
| 13 | `match subject { pattern => value … }` | user decision; its lowering (node_matches, node_cases) is emitter work, never sugar |

Not kernel (evidence: a pass already lowers them away before the emitter): `for` (extensions/sugar/iteration.warp → while), `enum` (plain_enums.warp → class with variants), `unless`/`until`, `?:`, `??`,
`x is T` (type_tests.warp → is_type call), `as` (conversion calls), comprehensions, ranges, `n times`. Operators are
notation; their meaning is a call (arithmetic ones are emitter intrinsics, as any builtin).

Open within the decision: `global` could be an attribute instead (`@global`); `try` could be an effect handler
(wiki/effects.md) later; `|x|`/`x =>`/`fun(x)` are three spellings of #4 (sugar).

## Rewrite engine: what it has, what sugar still needs

Has: holes `$a`, rest holes `$xs...`, typed holes `$x: text|symbol|list|block|int|number`, phrase runs (`$n beat`
inside a longer phrase), statement templates `($a; $b)`, origin conditions `where $c = channel()`, module rules that
skip words the program binds, bottom-up, each node once.

Features named by the sugar (F1–F7 built, spec and examples in notes/rewrites.md; F8 open, no card):
- **F1 positions/context** (built): match only as a statement head, in a condition, in a class body, as a key or a value.
- **F2 where-predicates** (built) beyond `= shape`: "$t is a type word", "$x unbound", "$n < 0 literal", "f's first
  parameter is typed".
- **F3 fresh names + evaluate once** (built): hygienic temporaries (`$·tmp`, `·` names, core/passes/rewrites.warp), so
  `a ?: b` computes a once.
- **F4 per-item templates**: apply a sub-template to each item a rest hole bound (`$xs...` mapped). Built as `T for $x in $xs...`; zip over two rest holes
  (`for ($a, $b) in …`, multiple_assignments) is not.
- **F5 name building** (built; decision name-glue): `·`
  glues a hole or symbol into a name inside templates and quoted code (`$i·end`, `square·$a` → square_a_number);
  numbers beside `·` multiply, plain `x·y` between values stays multiplication.
- **F6 several statements** (built, `rewrite ($a; $b) => …`): match a run of statements (`fib(0) := 0; fib(1) := 1` → one definition).
- **F7 re-rewrite** (built, loop limit 64): a result rewritten again until fixpoint (`a of b of c`, for variants onto the one for).
- **F8 text holes** (open, not started; idea: a text pattern `"…$x…"` binding holes from its
  interpolation parts, the template's text holes spliced the same way): rules over a template's holes (interpolation, `sql "…$x"`); stays a pass otherwise.

Rule of thumb for new syntax: a table row or a declaration (notation), else a `rewrite` line in
extensions/sugar/ (sugar); a new parser branch or pass only with the missing engine feature named in the commit.

Related: notes/rewrites.md (the engine's spec), wiki/pattern-matching.md (patterns and templates).
