# The kernel

The kernel is the small language at the bottom of wap, the new compiler: a dozen forms such as `If`, `While`, `Call`
and `Define`, plus the primitives. The emitter, which writes the wasm bytes, understands these forms and nothing else.
Everything a program may write (`for`, `unless`, `a if c`, `n times`, `match`, operators, units, ranges …) is
rewritten into kernel forms before the emitter sees it.

Think of it as a better Lisp with maps and types: a handful of special forms, and everything else is a function
call, a library definition or a rewrite rule.

Why have one:
- **The emitter stays small and total.** It has one case per kernel form instead of one branch per surface spelling.
  The old emitter has 130 branches over surface shapes; it reads `if` in three shapes and `as` once per target type.
- **A new spelling never touches the emitter.** `unless c { b }` is a rule that writes `If(not c, b, ø)`.
- **Its coverage is checked.** The emitter takes `kernel`, a closed union of types, so a form it does not handle is
  a compile error in the compiler, and a non-kernel node at run time is refused loudly, never skipped.

Read first: notes/first_principles.md. The plan this belongs to: notes/backbone.md.

## How a kernel form is declared

Each form is a named pattern of `Node`, code with holes wap/kernel/forms.wap :

```warp
pattern If = data((If $cond $then_part $else_part))
```

Think of a [[pattern]] as lightweight ad hoc class where each 'instance' arises only through structure.

One declaration gives four uses (extensions/match_patterns.warp, test/types/test_pattern_types.warp):

```warp
If(c, a, b)                        // build: the node (If c a b)
node is If                         // test
match node { If(c, a, b) => … }    // take apart, the holes bound by place
emit_if(n: If) := … n.cond …       // read a hole off a parameter of the type
```

and the emitter is one function over the forms, refusing any other node:

```warp
emit(n: Node) := match n { If(c, a, b) => …; While(c, body) => …; …; _ => refused(n, "not a kernel form") }
```

Three details:
- **Positions ride in the nodes.** A form is a Node, so it carries the line and column the program wrote, in its
  metadata like any parsed node. The stage walker (wap/core/stage.wap) gives every node a rewrite builds the position
  of the node it rewrote; the parts it reused, a pattern's holes, keep their own. Matching looks through the
  metadata. No form declares a position field, and no rule handles positions.
- **The head is the form's own name** (`(If c a b)`, `(Call f args)`): readable when printed, and never a keyword
  (`(if …)` inside a pattern would read as warp's own `if`). A program's own `(If a b)` group cannot pass for a
  kernel form, because only the rewrite stages build kernel nodes and the emitter sees nothing else.
- **Holes are named by what they hold, never by a keyword or a type word:** `yes`, `no`, `then`, `else`,
  `function` read as values, keywords or types, so `If` has `then_part` and `else_part`, and `Call` has `called`
  (the function called; a callee would be the one doing the calling). Typed holes (`$called: Symbol`) come with
  card typed-holes-check.

## The forms

| form | fields | the program wrote |
|---|---|---|
| Literal | the Node cases Number, Text, Char, True, False, Empty, Symbol | `3`, `"a"`, `'a'`, `yes`, `ø`, `#red` |
| Data | the reader's own nodes: a List (`[…]` a list, `{k: v}` a map) or a glued Key `Point{x: 1, y: 2}` (an instance), its parts forms (user 2026-11-17: the List node is the literal, no constructor primitive) | `[a, b]`, `{a: 1}`, `Point{x: 1, y: 2}` |
| Name | a Symbol read as a variable | `x` |
| Quote | `code`, `$x` holes spliced | `data a + b`, `data f($x)` |
| Define | `name parameters body` | `f(x) := …`, constants, `global n = 0` |
| Get | `place`: a name, `x.f` or `xs#i` | `p.x`, `xs#1` |
| Item | `x[k]`, an item counted from 0 or a map's entry, by the subject's type; typing turns it into Get or item(x, k) | `xs[0]`, `m["a"]` |
| Set | `place value` | `x = 1`, `p.x = 2`, `xs#1 = 3`, `x += 1` |
| Fn | `parameters body` | `x => x + 1`, `{ it * 2 }`, `\|x\| …` |
| Call | `called arguments` | `f(1, 2)`, `x.f(y)`, `a + b`, `print x`, `make(1)(2)` (called: a name or code giving a function value) |
| Prim | `operation arguments` | nothing directly: library bodies (below) |
| If | `cond then_part else_part` | `if/unless/?:/then-else`, `a if c` |
| Block | `statements`, its own scope | `{…}`, `(a; b)`, indented lines |
| While | `cond body` | `while`, `until`, `for`, `n times`, `loop` |
| Break, Continue, Return | none, none, `value` | the same words |
| Record | `name fields variants`, the one form that declares a type (user-approved 2026-11-17) | `class`, `enum`, `type X = A \| B` |
| Case | `subject arms` | `match`, `x is T` |
| Try | `body handler` | `try … catch`, `try x else y`, `except` |

How a few spellings reach the kernel:
- `for x in xs { b }` is `Block([i = iterator(xs), While(has_next(i), Block([x = next(i), b]))])`.
- `loop { b }` is `While(yes, b)`; `until c { b }` is `While(not c, b)`.
- `a + b` is `Call(+, [a, b])`: an operator is a function (first principles). Whether `+` on two ints becomes a
  single wasm instruction is decided by `+`'s library definition, whose body is a primitive.
- `x is int` is a `Case` with one arm.
- `class Point { x: int; y: int }` is `Record(Point, [x: int, y: int], [])`, `enum Color { red green }` is
  `Record(Color, [], [red, green])` (variants without fields are tags), `type Shape = Circle | Square` is
  `Record(Shape, [], [Circle, Square])` (variants that are records are subtypes). Its constructor is a Define
  whose body is the instance's data form, `Point(x, y) := Point{x: x, y: y}`, and `p.x` is a Get.

Not in the kernel, because they compute nothing and run before it: `use module`, `rule …`, `operator` rows. They
build the language the program is written in.

## Primitives

A primitive is an operation the emitter performs directly, because there is no warp code below it. It is the leaf
where warp ends and wasm begins. The program never writes `Prim`; the library does, as the body of a built-in:

```warp
+(a: int, b: int): int := prim i64.add(a, b)       // a wasm instruction
print(t: text) := prim host print_text(t)           // a host word, answered by the native driver or the browser
```

So `a + b` is an ordinary `Call` of the library's `+`, and the overload that fits two ints happens to be one
instruction. The compiler knows no built-in by name (first principles: "built-ins are declared like any function").

The kinds of primitive:
- **wasm instructions:** `i64.add`, `f64.sqrt`, `i64.rem_s` …
- **conversions:** `convert x T`, every `as` (today the old emitter has a branch per target type)
- **runtime words:** functions of the runtime written in warp (text, big numbers, equality …), notes/self_hosting.md
- **host words:** what the host answers (print, files, time, graphics)

A prim the kernel lacks means the emitter keeps a branch: that is the test of the list. ??

## How a program reaches the kernel

Rewrite rules turn the program into kernel forms, in three stages. The engine picks the stage from what a rule's
pattern asks:

| stage | runs | the pattern may ask | examples |
|---|---|---|---|
| 1 shape | right after reading | the shape only | `for` → `While`, `unless` → `If`, `try x else y`, `a ?: b`, `Color::Red`, comprehensions, ranges |
| 2 signature | after the declarations are collected | declared signatures and types (`takes value`, `is function`, `$x: Meters` of a declared parameter) | `print -1`, `x is T`, overloads by arity |
| 3 kind | after typing, on kernel code | the inferred type of a hole | `Meters + Meters` of variables, `d[1]` on a map that started `{}` |

A rule's output is warp code and must type-check; a type error in it is the rule's bug, reported at the user's line
with the rule named. A phrase no rule claims (`f x y` where nothing decides the reading) is refused with its
readings, in one place: the end of stage 2.

## Growing the kernel

A form or primitive joins the kernel only with a written case (what the emitter cannot do without it) and the
user's OK. Every addition shows up in scripts/vitals.py the day it lands.

## What the old compiler taught us

Measured on warp (the old compiler, 2026-11-17; the full census is in this file's history, commit 1851cab102):
- about 5,800 lines of its core know surface spellings (passes, parser branches, analysis). As rules they are
  estimated at 600 to 1,200 lines.
- the emitter infers types while it emits; separating analysis from emission is the single biggest saving
  (estimated 2,000 to 3,000 of its 6,300 lines).
- at least 7 passes walk variable bindings by themselves; wap has one scope analysis.
- of 410 rewrite rules, 82 ask about declared names or signatures and none about inferred types: stage 3 is new
  ground.
- `warp kernel <code|file>` (extensions/commands/kernel.warp) prints how many nodes of the old compiler's lowered
  program are kernel forms and each form outside the kernel with its count and first place.
- dependent types are not needed: typed holes, type variables and, for units, integer exponents in types cover every
  case found.

## Open points

1. **`While` or `Loop`:** this note uses `While(cond, body)`, which reads like the programs; `loop { if not c break; b }`
   is the same thing. Either way it is one form.
