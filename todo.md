# TODO

Open items live on the board https://github.com/users/pannous/projects/1 (columns Now / Next / Soon / Later / Done).
Use the `todo` command (~/dev/bin/todo): `todo add "title" [column] [-b details]`, `todo list [column]`,
`todo move <card> <column>`, `todo done <card> [commit]` (moves to Done and links the commit, default HEAD). New items go to Next; the user (or the Supervisor for the user)
moves cards. Without the board (offline, no gh), `todo add` appends below under "## Fallback"; `todo import` moves
those entries onto the board later.

## Fallback (not yet on the board: run `todo import`)
- [Now] pure-class-operators: instance operators dispatching to methods on the pure pipeline — Today a cast failure on the pure lane (found by Warp Lib Gap with Quantity, lib/units.warp): `a + b` where a's class has an operator method should dispatch to that method; builtins otherwise. Agent: Warp Signals. Branch: classops (boxtree).
