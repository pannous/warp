# TODO

Open items live on the board https://github.com/users/pannous/projects/1 (columns Now / Next / Soon / Later / Done).
Use the `todo` command (~/dev/bin/todo): `todo add "title" [column] [-b details]`, `todo list [column]`,
`todo move <card> <column>`, `todo done <card> [commit]` (moves to Done and links the commit, default HEAD). New items go to Next; the user (or the Supervisor for the user)
moves cards. Without the board (offline, no gh), `todo add` appends below under "## Fallback"; `todo import` moves
those entries onto the board later.

## Fallback (not yet on the board: run `todo import`)
- [Now] pure-class-operators: instance operators dispatching to methods on the pure pipeline — Today a cast failure on the pure lane (found by Warp Lib Gap with Quantity, lib/units.warp): `a + b` where a's class has an operator method should dispatch to that method; builtins otherwise. Agent: Warp Signals. Branch: classops (boxtree).
- [Next] pure-units-quantity-sum: use units; quantity(5,"m") + quantity(3,"m") fails on pure — On the pure lane `use units; a = quantity(5, "m"); b = quantity(3, "m"); print((a + b).amount)` gives Error("outside the core: the unknown name for …:4:19"), also as a.plus(b).amount (Rust: 8). Not the operator dispatch (card pure-class-operators): something in lib/units.warp's Quantity path. Found by Warp Signals.
- [Now] pure-closure-cells: a closure that changes a captured variable prints 1 1 1 instead of 1 2 3 on the pure pipeline (Warp Idioms)
- [Now] pure-call-function-values: (x => x*2)(5) and adder(5)(1) print the lambda instead of calling it on the pure pipeline (Warp Idioms)
- [Now] pure-text-case-words: upper/lower/uppercase/.upper() on a text refused with 'a list where a text is needed' on the pure pipeline (Warp Idioms)
- [Now] pure-pipes: 3 | inc and f = inc | dbl give 'a function value in or' on the pure pipeline (Warp Idioms)
