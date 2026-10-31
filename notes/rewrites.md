# The rewrite engine (core/passes/rewrites.warp, card rewrite-engine)

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
| `$w is label` | a word labelling a parameter: a phrase preposition or `as` (words.warp label_words), F10 |
| `$x is text` (any hole kind) | code of that kind |
| `$n < 0` (`> <= >= == !=`) | a number literal, `-1` and `(-1)` too, so compared |
| `$f takes value`, `$f takes int` | a function of the program or a typed library word whose first parameter is typed (not yes/no), or typed int |

`not` negates any of them except the origin (`where not $f takes value`; `$t is not type` reads the same). Rules try
in order, so a later rule without conditions is the fallback. The `where` may stand after a template the parser
continued into it (`=> $c + $n where …`).

Three more ask about a word list and where the matched code stands (card engine-rule-conditions):

| condition | holds when |
|---|---|
| `$x in [add, push]` (also `$x is in […]`) | the hole bound one of the words listed, a name or a text naming it (`"push"`) |
| `method` | the matched code is the method part of `x.f(…)` or `x.f`: `f(…)`, `f`, the right of `.` |
| `shadowed` | a definition around the matched code binds the word it starts with: a parameter, or a name its body declares (`f(t) := { parse = x => 7; parse(t) }`, notes/dispatch.md: shadowing is scoped) |

Each is negated as the others: `where $f not in [add, push]`, `where not method and not shadowed`:

```warp
rewrite evaluate($t) => interpret(parse($t)) where not method and not shadowed
rewrite $x.$f($a) => $f($x, $a) where $f not in [add, append, push, insert, pop, remove, delete, clear]
```

The engine walks the program keeping where the node it tries stands (rewrites.warp bottom_up_placed): the method part
of the `.` it is in, the names the definitions around it bind. `shadowed` asks only the definitions around the code; a
word the program binds at its top level keeps the module and sugar rules away already (a program's own word keeps its
meaning), and `$x is unbound` asks it of a hole. A run of statements (F6) has no matched code of its own, so neither
holds there. Open: a lambda's parameter (`plus => plus(1)`) and a class's method bodies don't shadow yet, and a
result rewritten again (F7) is walked without them.

Tests: tests/modules/test_rule_conditions.warp; probes/rewrites/rule_conditions.

## Places (F1)
A place word among the `where` conditions lets a rule rewrite code only standing there:

| word | where |
|---|---|
| `statement` | a statement of a block or the program |
| `condition` | the condition of an `if` or `while` |
| `member` | an item of a class body |
| `key`, `value` | the key or the value of `k: v` |
| `element` | an item of a list literal `[…]` (the tuple `(1, 0)` of `[(1, 0), (2, 1)]`, core/extensions/tuples.warp) |

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
`add_up(1 + (2 + 3))` (right to left, F13); a run as long or longer moves on, so this can't loop (F12). The sugar step (lib/sugar) matches
runs of statements the same way. Tests: tests/modules/test_rewrite_zip_runs.warp, probes/rewrites/zip_runs.

Runs are rewritten right to left (F13, card engine-right-to-left-runs): the run starting at the last statement is
tried first, then the one before it, so a rule for the first statements of a run sees what follows them already
rewritten. Clauses join from the last pair, each pair rule seeing the joined rest:

```warp
rewrite (fib($a) := $x; fib($n) := $y) => (fib($n) := if $n == $a then $x else $y) where $a is literal
fib(0) := 0
fib(1) := 1
fib(n) := fib(n - 1) + fib(n - 2)   # one fib: if n == 0 then 0 else if n == 1 then 1 else fib(n - 1) + fib(n - 2)
```

A pairwise rule folds to the right: `rewrite (say($a); say($b)) => say($a - $b)` turns `say(10); say(5); say(2)`
into `say(10 - (5 - 2))`, and a rule for an end (`(item($a); done) => last_item($a)`) reaches every pair before it
(`(item($a); last_item($b)) => last_item($a + $b)`). Phrase runs (`keep 1 past 3`) stay left to right. Tests:
tests/modules/test_rewrite_right_to_left_runs.warp, probes/rewrites/right_to_left_runs.

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

## Function keywords: early sugar, applied holes, parenthesized templates (card sugar-definitions)
lib/sugar/functions.warp takes the function keywords (`def`, `fun`, `fn`, `define`, `function`, `func`) away before
any pass runs: it is an early sugar module (`early_sugar_module_names`), whose rules rewrite the program right after
interpolation, before definitions.warp and the passes that read definitions. Three notations make its rules readable:

- a hole applied reads as the name it stands for applied (parser.warp): `$h($p...)` is a call, `$h($p...) {$b...}` a
  call with its block side by side, and `$h($p...): $t {…}` a typed head, so a pattern has the shape of the code;
- parentheses around one template only group it, as around one pattern: `=> ($h($p...) := {$b...})` keeps the
  `:=`, which binds looser than `=>`, in the template;
- a block of a rest hole, `{$b...}`, gives the statements it bound back as statements.

```warp
rewrite (def $h($p...): $t {$b...}) => ($h($p...): $t := {$b...})
rewrite def $h($p...) {$b...} => ($h($p...) := {$b...})
rewrite (def $d) => $d
```

## Operator holes: `$op=` and `.$op` (cards elementwise-operator-forms, compound-assignment-desugaring)

`rewrite ($a $op= $b) => $a = ($a $op $b)` and `rewrite ($a .$op $b) => element_wise_any($a, $b, ($l, $r) -> ($l $op $r))`
(lib/sugar/arithmetic.warp) bind the operator a compound assignment or an element-wise form is written with; a
template's `$l $op $r` (bare or in parentheses) becomes the operation, a function word's call (`x max= 5`). Pitfalls met:
- the seed reads new rule syntax with its old parser (`$op=` as `$op =`) and applies the rules to the compiler's own
  code: guard such rules with a condition the old reading fails (`where $op is symbol`)
- `x = $a $op $b` parses as `(x = $a) $op $b` (a phrase takes the assignment): group the operation in the template
- a pattern whose operator hole binds weaker than `=>` loses its rule: parenthesize the pattern
- a hole marked as an operator only when the affix bound it, else every phrase template `match $x {…}` became a call
- rules_reaching only tries a rule whose operator the code holds: the operator inside `/!=` counts (operated_within)
- the sugar loader drops a rule after a comment with an odd apostrophe (`operator's`)

## Typed overloads of built-in operators (decision rules-to-typed-operators, card rewrite-rules)

`infix operator |(x: any, g: function) := g(x)` (in a lib/sugar file, a rule module or a program) is not a function:
rewrites.warp operator_rules makes it rules of the operator, `$x | $g => $g($x)`, its parameter types the conditions
(overload_conditions: function → `is function` or `takes one`, one rule each; type/text/list/map/int/number/symbol →
`is <kind>`; any → none). Overloads of one glyph are ordered most specific first (fewer `any`), an arm whose body is the
operator itself (`|(a: type, b: type) := a | b`) before the others as specific; it emits no rule but adds to every later
rule `not <its condition>` for one of its typed operands (one rule per operand), so the built-in stays where it fits.
Names the body's lambdas and assignments introduce are template holes, fresh at each firing. Only glyph spellings
(no letters: the parser names a declared operator as written, `|` not `or`), so prelude's `add(xs, x)` stays a function;
an operand typed by a class leaves it to class_methods.warp. Dispatch is the rule engine's, so it costs no run time;
operands whose kind only the run time knows get the built-in, as the hand rules did.
- the seed-built warp.wasm packs lib rules with the stage that has operator_rules, so a sugar file's overloads are baked
- a std module holding only overloads (no `rewrite` line) is neither packed nor in rule_needs: add a rewrite line or
  extend those scans when one appears
