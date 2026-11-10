# Abandoned branches

Branches deleted without landing that have no open card. The commits stay reachable by sha.
Abandoned branches that have a card are noted on the card (`abandoned: <branch> <tip>, …`).

- input-bind-name 2e3c9a6235: input{ bind: name } form bindings. The feature is on main in another form (events.warp bind rule, class Binding; card input-bind-made tracks the self-compile cost). The branch also carried a stray 1429-line lib/modules/words.tsv. Interviewer verdict: drop.
- wasm-module-uses 85fd7a6e18: only rebuilt cwasm fixture bytes. `use x.wasm` landed in f5da067d9c. Interviewer verdict: drop.
