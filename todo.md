# TODO

Open items live on the board https://github.com/users/pannous/projects/1 (columns Now / Next / Soon / Later / Done).
Use the `todo` command (~/dev/bin/todo): `todo add "title" [column] [-b details]`, `todo list [column]`,
`todo move <card> <column>`, `todo done <card> [commit]` (moves to Done and links the commit, default HEAD). New items go to Next; the user (or the Supervisor for the user)
moves cards. Without the board (offline, no gh), `todo add` appends below under "## Fallback"; `todo import` moves
those entries onto the board later.

## Fallback (not yet on the board: run `todo import`)
- [Next] a map parameter accepts any value: f(m:map) := count(m); f(3) is 1, no type error (single-entry)
- [Next] f(m: map of int) := … reads 'of' as a second parameter: 'f needs a value for parameter of' (single-entry)
- [Now] fast-try-binary: /Users/me/dev/bin/warp (what the user's plain warp runs) is main's DEBUG build, so finger paint feels much slower than the release build the user tried earlier (show 250-365 ms debug at load 21 vs 14 ms release; strokes unchanged at 0.3 ms). push_tested.sh should also build/copy a release binary for the user (debug stays for agents), then re-measure show in a real window (user 2026-10-12: 'still a regression')
- [Next] fade_sound in the browser: the playground plays each sound without a GainNode ramp; fade_sound fails there as no such word (native-sound)
- [Next] visualizer test flake: the_visualizer_paints_while_it_plays exited non-zero silently at load 45 on main d9820af0e (stderr: sound + painted frames, no error); the assert message should print run.status (signal vs code) to tell
