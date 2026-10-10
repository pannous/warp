# Block-scoped handlers: when, versus try/catch, and one mechanism ahead

Mechanics: notes/effect_handlers.md (steps 1–3). Errors: wiki/Error.md, wiki/try.md, decisions P21, P60, P67, #34.

## When a handler belongs to a block
`on e {h} in {block}` answers `emit e` only while the block runs, also deep inside the functions it calls; when the block ends, the handler is gone.
Use it when an answer is valid for one piece of code only:

| scenario | example |
|---|---|
| context / configuration without passing parameters | `home = on tax_rate { 0.2 } in { price(100) }`, the same for locale, currency, user |
| tests: fix the clock, randomness, network | `on now { 2026-01-01 } in { check_expiry(card) }` |
| recovery that resumes with a substitute | `on bad_number { 0 } in { sum(lines) }` keeps summing |
| collecting what code yields or logs | `on yield x { found += [x] } in { walk(tree) }`, `on log { } in { noisy() }` |
| sandboxing | `on write_file { break "denied" } in { plugin.run() }` |
| transactions | buffer every `emit write` inside, commit or drop at the end |
| modal UI | a dialog takes `key`/`click` only while open, the outer handlers return by themselves |
| overrides | an inner handler shadows an outer one for its block only |

Program-wide `on e {h}` fits handlers that live as long as the program (app key bindings, main event loop).
Block scope can't leak or be forgotten, nests, and is per task where a global isn't.

## Compared with try/catch
| | `try X else Y` / `catch e` | `on error of f {h}` | `on e {h} in {X}` |
|---|---|---|---|
| who raises | `raise`, Error values, traps directly under try (#34) | any failing call of f | `emit e`, any depth |
| after the handler | X is abandoned, Y is the value | that call's value is h | **resumes**: emit gives h, X goes on; `break v` abandons X with v |
| selects by | everything (one tag `warp_error`) | function name | event name |
| scope | the expression X | every call of f, program-wide | the block, dynamically |
| effect rows | not tracked | not tracked | `e` in f's effects, removed by the block |
| lowering | try_guard.rs, wasm tag `warp_error` | its own lowering | scoped_handlers.rs, globals + tag `warp_abort` |

So try/catch is the aborting special case of a handler for one event (`error`) and cannot resume. A handler can do
both: resume (Common Lisp's restarts, the "substitute a value" case) or abort (`break`, try's case).

## Look ahead: one mechanism
Three lowerings do one thing. All could be scoped handlers:
- `raise x` = `emit error{x}` whose handler never resumes
- `try X else Y` = `on error { break Y } in { X }`; `catch e {…}` = `on error { break … event … }` (P67 binds e)
- `on error of f {h}` = each call of f wrapped in `on error { break h } in { f(…) }`
- `finally Z` = a leave action of the block (the leave already runs on both paths)
- a collecting generator call = `on yield v { g·yielded += [v] } in { g(…) }` (generators.rs today inlines this by hand)

Gains: one lowering and one wasm tag instead of `warp_error` + `warp_abort`; errors appear in effect rows (Error.md
wants "marks the containing function as potentially throwing"); typed catches for free (`on no_food {…}` is
`catch (no food)`); errors become resumable where it makes sense.

What has to be decided or built first:
1. **Default per event.** An unhandled emit does nothing (P163); an unhandled error must end the program. Needs a
   default handler per event: `error` → abort the program, others → ø with the got-it warning (P202).
2. **Traps can't resume.** Index, divide-by-zero etc. have no value to go on with; they can only take the abort path,
   and deep traps (#34) need the guarded-body host import first.
3. **Handler bodies can't see the locals** of the function they're written in yet (effect_handlers.md limit); try's
   `else` can. Needed before try can be rewritten as a handler.
4. **Unwinding.** An error escaping a block leaves the handler active today; the unified version needs leave-on-throw.
5. Cost: the handler path is globals plus calls; try is one wasm try block. Small `try` stays the cheaper lowering
   unless the handler lowering inlines a single literal handler.

Order if we do it: (1) default-per-event + leave-on-throw, (2) `try … else` as sugar for `on error { break … } in`,
same tests green, (3) `on error of f`, (4) delete try_guard's own tag. Generators last, they work and are fast.
