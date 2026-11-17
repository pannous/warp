# The kernel

The kernel is the small language at the bottom of wap, the new compiler: a dozen forms such as `If`, `While`, `Call`
and `Define`, plus the primitives. The emitter, which writes the wasm bytes, understands these forms and nothing else.
Everything a program may write (`for`, `unless`, `x if c`, `n times`, `match`, operators, units, ranges …) is
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

Each form is a narrowed type of `Node` whose body is a pattern with named holes (notes/backbone.md "Kernel forms are
narrowed types of Node with a pattern body"). This is the same mechanism as `type Meters < int`, with a pattern
instead of "any int":

```warp
type If < Node = (·if cond: Node then_part: Node else_part: Node)
```

One declaration gives four uses (built in extensions/match_patterns.warp, tested in test/types/test_pattern_types.warp):

```warp
If(c, a, b)                        // build: the node (·if c a b)
node is If                         // test
match node { If(c, a, b) => … }    // take apart, the holes bound by place
emit_if(n: If) := … n.cond …       // read a hole off a parameter of the type
```

and the emitter is one function over the closed union:

```warp
kernel = Literal | Name | Quote | Define | Get | Set | Fn | Call | Prim | If | Seq | While | Break | Continue | Return | Record | Case | Try
emit(n: kernel) := match n { If(c, a, b) => …; While(c, body) => …; … }
emit(n: Node) := refuse("not a kernel form: " + n)
```

Two details:
- **The heads start with `·`** (`·if`, `·call`): a reserved word no program can write, so a program's own `(if a b)`
  group is never mistaken for a kernel form. Warp's reader does not yet read `·if` as one word; until it does, the
  declarations use plain heads (open point, see the end).
- **Holes are named by what they hold, never by a keyword:** `yes`, `no`, `then`, `else`, `function` and type words
  are values or keywords in warp, not names, so `(·if cond yes no)` would have a single hole.

## The forms

| form | declaration | the program wrote |
|---|---|---|
| Literal | the Node cases Number, Text, Char, True, False, Empty, Symbol | `3`, `"a"`, `'a'`, `yes`, `ø`, `#red` |
| Name | a Symbol read as a variable | `x` |
| Quote | `(·quote code: Node)`, `$x` holes spliced | `data a + b`, `data f($x)` |
| Define | `(·define name: Node parameters: Node body: Node)` | `f(x) := …`, constants, `global n = 0` |
| Get | `(·get place: Node)`: a name, `x.f` or `xs#i` | `p.x`, `xs#1` |
| Set | `(·set place: Node value: Node)` | `x = 1`, `p.x = 2`, `xs#1 = 3`, `x += 1` |
| Fn | `(·fn parameters: Node body: Node)` | `x => x + 1`, `{ it * 2 }`, `\|x\| …` |
| Call | `(·call callee: Node arguments: Node)` | `f(1, 2)`, `x.f(y)`, `a + b`, `print x` |
| Prim | `(·prim operation: Node arguments: Node)` | nothing directly: library bodies (below) |
| If | `(·if cond: Node then_part: Node else_part: Node)` | `if`, `unless`, `a ? b : c`, `a if c`, `a or b` |
| Seq | `(·seq statements: Node)`, its own scope | `{…}`, `(a; b)`, indented lines |
| While | `(·while cond: Node body: Node)` | `while`, `until`, `for`, `n times`, `loop` |
| Break, Continue, Return | `(·break)`, `(·continue)`, `(·return value: Node)` | the same words |
| Record | `(·record name: Node fields: Node variants: Node)` | `class`, `enum`, `type X = A \| B` |
| Case | `(·case subject: Node arms: Node)` | `match`, `x is T` |
| Try | `(·try body: Node handler: Node)` | `try … catch`, `try x else y`, `except` |

How a few spellings reach the kernel:
- `for x in xs { b }` is `Seq(i = iterator(xs); While(has_next(i), Seq(x = next(i); b)))`.
- `loop { b }` is `While(yes, b)`; `until c { b }` is `While(not c, b)`.
- `a + b` is `Call(+, [a, b])`: an operator is a function (first principles). Whether `+` on two ints becomes a
  single wasm instruction is decided by `+`'s library definition, whose body is a primitive.
- `x is int` is a `Case` with one arm.

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
- **constructors of values:** `new_list items`, `new_map entries`, `new Name fields` (list, map and instance
  literals)
- **runtime words:** functions of the runtime written in warp (text, big numbers, equality …), notes/self_hosting.md
- **host words:** what the host answers (print, files, time, graphics)

The test of the list: whenever the emitter needs a branch that is none of the forms above, either a primitive is
missing, or the branch is really a spelling and belongs in a rule.

## How a program reaches the kernel

Rewrite rules turn the program into kernel forms, in three stages. The engine picks the stage from what a rule's
pattern asks:

| stage | runs | the pattern may ask | examples |
|---|---|---|---|
| 1 shape | right after reading | the shape only | `for` → `While`, `unless` → `If`, `a ?: b`, ranges |
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
- dependent types are not needed: typed holes, type variables and, for units, integer exponents in types cover every
  case found.

## Open points

1. **The `·` heads:** warp's reader reads `·if` as `·` followed by the keyword `if`. Either the reader accepts a word
   starting with `·` (a small reader change in warp), or the kernel heads are plain words. Plain words are safe as
   long as only the rewrite stages build kernel nodes, which is the design; `·` adds a guarantee no program can forge
   one.
2. **`While` or `Loop`:** this note uses `While(cond, body)`, which reads like the programs; `loop { if not c break; b }`
   is the same thing. Either way it is one form.
