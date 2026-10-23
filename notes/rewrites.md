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
- `··` glues with the mark itself: `loop··step` is the hidden compiler name `loop·step` (notes/compiler_temporaries.md)
  no program can write, the way for_loops.warp marks a loop's step; `$i··end` of k is `k·end`, `$i·end··x` is
  `k_end·x`. The parser reads `a··b` as the declared-only operator call `operator_··(a, b)`, so outside a template it is
  an operator nothing declares, an error (lib/sugar/loops.warp: the C-style for's `loop··step: $step`).
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
| `$x is missing` | ø, also an operand not written (the end of `rest...`) |
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

The clause takes more than one hole (card f4, 2026-10-23):

```warp
rewrite numbered $xs... => [$x * $i for $x at $i in $xs...]              # numbered 5 5 5 → [5 10 15]: $i the position, from 1
rewrite paired ($as...) ($bs...) => [$a + $b for ($a, $b) in ($as..., $bs...)]   # zip: rest holes of as many items
rewrite pairs {$as...} => [$k * 10 + $v for ($k: $v) in $as...]          # a pattern takes each item apart
rewrite positions_of $e => [$i for $c at $i in cases of $e] where $e is enum     # the cases of an enum
```

An item a taking-apart pattern doesn't match stays in the result as it is (`($k + $v)` over `(1 + 2) 5` gives `[2 5]`);
zipped rest holes of different lengths don't splice (the rule doesn't fire). A zipped rest hole the pattern didn't
bind is as many fresh names (F3) as the others hold items, and stays bound for the rest of the firing (F11, card
rewrite-zip-runs):

```warp
rewrite kept($xs...) => (($t = $x) for ($t, $x) in ($ts..., $xs...); [$ts...])   # t·1 = 7; t·2 = 8; [t·1, t·2]
```

## Runs of statements (F6)
A pattern of several statements `($a; $b)` matches a run of them standing one after the other in a block, holes
binding across the run; the run is replaced by the template's statements:

```warp
rewrite ($a = $x; $b = $a) => ($a = $x; $b = $x + 1)
```

A run a rule made shorter is tried again where it stands, together with the statements after it, so a pairwise rule
folds a whole run: `rewrite (add_up($a); add_up($b)) => add_up($a + $b)` turns `add_up(1); add_up(2); add_up(3)` into
`add_up((1 + 2) + 3)`; a run as long or longer moves on, so this can't loop (F12). The sugar step (lib/sugar) matches
runs of statements the same way. Tests: tests/modules/test_rewrite_zip_runs.warp, probes/rewrites/zip_runs.

## Results rewritten again (F7)
A rule's result is rewritten again by the rules, its parts first, until none fires, so rules build on rules:

```warp
rewrite double $x => $x * 2
rewrite quadruple $x => double (double $x)
quadruple 3   # 12
```

A result rewritten 64 times in a row (`rewrite_depth_limit`) is the rules' loop: an error, not a hang.

A rule of a place (F1) rewrites its result again by the rules of that place, until none matches, so the declaration
words before a statement all go, in any order (card engine-placed-where-chain; rewrites.warp at_place):

```warp
rewrite loudly $s... => $s... where statement
rewrite gently $s... => $s... where statement
gently loudly gently f() := 3      # f() := 3
```

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

## Rest templates
A template that is a rest hole alone, `$s...`, is the words the hole bound, rebuilt as a phrase; a phrase template
that comes out one word long is that word. So one rule drops a declaration word before anything:

```warp
rewrite public $s... => $s...    # public static fun f() := 3 → static fun f() := 3, and so on by the next rules
rewrite marked $a $b... => $a $b...   # marked 5 → 5
```

The parser reads `...` at the end of a line as a range going on into the next line, so a rule ending in `$s...`
swallowed the line after it (the next rule, or the program's next statement); the engine ends such a rule line with
`;` before parsing (rewrites.warp rules_ended, for the program in passes.warp and for each module). Before (card
engine-rest-template) a bare `$s...` gave the tree `$s to ø` around the words, and a phrase of one word stayed a
phrase, so each word took two fixed-arity rules. A `where` after a template ending in `$s...`
on its line, which the parser reads as the range's end, ends the template (split_at_where); on the next line it is
still swallowed. Tests: tests/modules/test_rewrite_engine.warp.

## Called holes
A hole written right before `(`, `$t($a...)`, is the call it reads as, as the code's `P(1)` is; the parser itself reads
it as the hole and a group, `$t ($a...)`, which matched no call (card engine-guarded-call-phrase; rewrites.warp
with_called_holes, in patterns and templates). `$t ($a)`, with a space, stays the hole and a group.

```warp
rewrite build $t($a...) => $t($a...) where $t is type   # p = build P(4) → p = P(4); build f(2) stays
```

Tests: tests/modules/test_rewrite_engine.warp; probes: probes/rewrites/guarded_call.
