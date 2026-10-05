# TODO

Open items live on the board https://github.com/users/pannous/projects/1 (columns Now / Next / Soon / Later / Done).
Use the `todo` command (~/dev/bin/todo): `todo add "title" [column] [-b details]`, `todo list [column]`,
`todo move <card> <column>`, `todo done <card> [commit]` (moves to Done and links the commit, default HEAD). New items go to Next; the user (or the Supervisor for the user)
moves cards. Without the board (offline, no gh), `todo add` appends below under "## Fallback"; `todo import` moves
those entries onto the board later.
