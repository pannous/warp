# Bend-style laws cleanup

- Normalize the quantified law alias in the law layer, leaving the general parser and ordinary `for` loops unchanged.
- Keep one canonical law representation so assertion, property testing, and Lean export reuse the existing pipeline.
- Add a checked `LAWS.warp` convention example and make `warp verify` end with an explicit proof-gate verdict.
- Cover the alias, file convention, success wording, and failure wording with focused tests.
- Update the laws and Bend notes, run the focused tests, then run `cargo fix --offline --allow-dirty --lib --bins`.

# REPL variable persistence cleanup

- Keep the console's entered state in one small abstraction instead of spreading history handling through the readline loop.
- Re-evaluate only state-changing lines before the current expression, so ordinary queries and prints are not replayed.
- Add a headless console probe that exercises assignment followed by a variable read through the real CLI.
- Record the replay-based persistence boundary in the console notes, then run the focused CLI test, clippy, and wasm32 check.
