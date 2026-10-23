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

## Name building (F5)
`·` in a template glues words and holes into one name (decision name-glue), the words joined by `_`:

```warp
rewrite count_up $i till $b => ($i = 1; $i·end = $b; while $i <= $i·end { $i += 1 })   # k_end
rewrite made $f $ws... => ($f·$ws = 4)          # made square a number → square_a_number = 4
rewrite doubled $f => (twice·$f(x) := 2 * $f(x)) # doubled sq → twice_sq(x) := 2 * sq(x)
rewrite speed_unit => km·per·hour                # km_per_hour
```

- Glue is recognised only in a rule's template, between words and holes (also the head of a
  call or definition `twice·$f(x)`): the parser reads `·` as `*` (lib/math.warp), so the engine reads the rule's
  source text before the right operand to tell `·` from `*`. `*` in a template stays the product, and outside templates
  `x·y` multiplies as before.
- A hole contributes the word it bound, or each word of a phrase it bound (a rest hole `$ws...` written `$ws`: `a number` → `a_number`); a fresh hole's
  temporary stays one (`t·3_end`).
- Numbers multiply: a number written beside `·` (`$x·2`) leaves the product, and so does a hole that bound a number
  (`$n·ten` of 3 → `3 * ten`). A hole bound to other code (`x + 1`) is an error.
- A template's definition with a hole or a built name as its head (`$f(x) := …`, `twice·$f(x) := …`, which the parser
  reads as a phrase) defines that function, and a template of one parenthesized definition is that definition.
- Open: generated names don't name their rule in help and errors yet; a built name clashing with the program's own
  isn't refused yet; glue in quoted code (`data(…)`) outside rules.

## Positions
Everything the template adds (its words, keys and groups, the fresh names) is placed where the rewritten code was
written; the holes keep the positions of the code they bound. An error inside a rewrite points at the user's line,
never at the rule.

Tests: tests/modules/test_rewrite_engine.warp (warp only, `warp test tests/modules`).

## Where-conditions (F2)
After `where`, conditions joined by `and` ask about what the holes bound:

| condition | holds when the hole bound |
|---|---|
| `$c = channel()` | a variable assigned code of that shape (the origin; one per rule) |
| `$t is type` | a type word no value of the program is named, or a type it declares |
| `$x is unbound` | a word the program binds nothing to |
| `$x is value`, `$f is function` | a word the program binds as a value, as a function |
| `$x is literal` | a number or text written as such |
| `$w is label` | a word labelling a parameter: a phrase preposition or `as` (words.warp label_words), F10 |
| `$x is text` (any hole kind) | code of that kind |
| `$n < 0` (`> <= >= == !=`) | a number literal, `-1` and `(-1)` too, so compared |
| `$f takes value`, `$f takes int` | a function of the program or a typed library word whose first parameter is typed (not yes/no), or typed int |

`not` negates any of them except the origin (`where not $f takes value`; `$t is not type` reads the same). Rules try
in order, so a later rule without conditions is the fallback. The `where` may stand after a template the parser
continued into it (`=> $c + $n where …`).

## Places (F1)
A place word among the `where` conditions lets a rule rewrite code only standing there:

| word | where |
|---|---|
| `statement` | a statement of a block or the program |
| `condition` | the condition of an `if` or `while` |
| `member` | an item of a class body |
| `key`, `value` | the key or the value of `k: v` |

```warp
rewrite $s except_if $c => if not $c then $s where statement
rewrite field $n => $n: int where member
```

The engine applies a place's rules to a node's parts standing there, right after their own rules ran (children
first, as everywhere); a rule without a place word rewrites anywhere. Phrase runs (`$n half`) stay for rules without a
place.

## For each item (F4)
`T for $x in $xs...` in a template splices T once for each item a rest hole bound, `$x` bound to that item (fresh
names are fresh for each). It works in a list, a map and a run of statements:

```warp
rewrite squares $xs... => [$x * $x for $x in $xs...]
rewrite bump_all $vs... => ($v += 1 for $v in $vs...)
rewrite flags $ns... => {($n: yes) for $n in $ns...}
```

Zip over two rest holes (`$a = $b for ($a, $b) in …`) isn't built yet.

## Runs of statements (F6)
A pattern of several statements `($a; $b)` matches a run of them standing one after the other in a block, holes
binding across the run; the run is replaced by the template's statements:

```warp
rewrite ($a = $x; $b = $a) => ($a = $x; $b = $x + 1)
```

## Results rewritten again (F7)
A rule's result is rewritten again by the rules, its parts first, until none fires, so rules build on rules:

```warp
rewrite double $x => $x * 2
rewrite quadruple $x => double (double $x)
quadruple 3   # 12
```

A result rewritten 64 times in a row (`rewrite_depth_limit`) is the rules' loop: an error, not a hang.

## Enum cases (F9)
Three more `where` conditions ask about the enums the program declares (`enum Coin { Penny, Dime }`, Swift's
`{ case north, south }`), read before plain_enums.warp lowers the declarations:

| condition | holds when the hole bound |
|---|---|
| `$e is enum` | a name of a declared enum, or that name as a text (`is_type(c, "Coin")`) |
| `$c is case of $e` | a case of the enum `$e` bound; with `$e` unbound, a case of the one enum declaring it, `$e` then bound to that enum |
| `$c is case` | a case of the one enum declaring it |

In a template, `cases of $e` stands among a list's or a call's items for the cases of the enum `$e` names, each a
symbol placed where the rewritten code was written:

```warp
rewrite $e.$c => $c where $c is case of $e
rewrite is_type($v, $e) => collection_contains([cases of $e], $v) where $e is enum
```

The rules of lib/sugar/enums.warp use them. Not built: a case's index in a template (`Coin.Dime as int` → 2) and
`cases of $e` as the source of `T for $x in …` (F4); the conversions stay in plain_enums.warp.

## Parameters (F10)
The place word `parameter` lets a rule rewrite only a parameter of a definition's head, `f(a, with b) := …` (also
`f(a): T := …`), in the program's own rules and in the sugar step alike; `$w is label` asks for a word that labels a
parameter. lib/sugar/parameters.warp drops the label, as Swift passes argument labels:

```warp
rewrite $word $p => $p where $word is label and parameter   # replace(t, old, with new) := … takes new
```

Write the place word last: with it first, `where parameter and $w is label`, the rule rewrote the head wrongly
(`scaled(x, by factor) := x * factor` lost `x`, so the body's `x` was unknown). The sugar step applies a rule of a
place only there (before F10 it ignored places). Tests:
tests/modules/test_rewrite_parameters.warp.
