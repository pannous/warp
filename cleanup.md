# Bend-style laws cleanup

- Normalize the quantified law alias in the law layer, leaving the general parser and ordinary `for` loops unchanged.
- Keep one canonical law representation so assertion, property testing, and Lean export reuse the existing pipeline.
- Add a checked `LAWS.warp` convention example and make `warp verify` end with an explicit proof-gate verdict.
- Cover the alias, file convention, success wording, and failure wording with focused tests.
- Update the laws and Bend notes, run the focused tests, then run `cargo fix --offline --allow-dirty --lib --bins`.
