# The rewrite engine (lib/compiler/rewrites.warp, card rewrite-engine)

`rewrite PATTERN => TEMPLATE [where …]` rules carry the sugar layer (notes/layers.md). The engine gains the features
the migration needs one at a time, each landed on its own before warp-layers moves the modules it unblocks.

## Fresh names (F3)
A template's hole the pattern doesn't bind is a fresh name each firing: `$t` becomes `t·3`, a compiler temporary
(notes/compiler_temporaries.md) no program can write, and every use of `$t` in that firing is the same name. Evaluate
once is spelled with one:

```warp
rewrite $a otherwise $b => ($t = $a; if $t then $t else $b)
```

Nested firings (`keep 1 past (keep 2 past 3)`) each get their own, and the program's own `t` is untouched.
A template of several statements `($a; $b)` takes a statement's place; in a value's place (`z = keep 1 past 3`) it
is that value's block `(t·1 = 1; 3; t·1)`. A phrase rule matches the words as written, also where the parser nests
them (`keep 1 (past 3)`).

## Positions
Everything the template adds (its words, keys and groups, the fresh names) is placed where the rewritten code was
written; the holes keep the positions of the code they bound. An error inside a rewrite points at the user's line,
never at the rule.

Tests: tests/modules/test_rewrite_engine.warp (warp only, `warp test tests/modules`).
