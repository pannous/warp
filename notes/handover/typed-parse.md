# typed-parse

State: the new outcome test is committed; implementation remains. No existing tests changed.

Default: no coercion; reject missing required and unknown fields, allow optional or defaulted fields (decision D4). A new outcome test covers valid input and the motivating type mismatch. Existing rewrite rule words can inspect `$program`; `class_view` and `class_declared` provide the declarations and field types.

`WARP_NO_WINDOW=1 /Users/me/dev/bin/warp test test/types/test_typed_parse.pairs.warp` reports 1 of 2 failed: valid input gives `outside the core: the unknown name parse of User at 1:42 (no owner)`; the invalid-input error case passes even without validation and therefore does not prove it. `rewrite parse<User>($a) => 42` matches, but `rewrite parse<$t>($a) => 42 where $t is type` and `rewrite $f($a) => 42` do not. `core/parsing/parser.warp` type_application_name makes `parse<User>` one symbol `parse of User`, so the hole is not a separate argument. `rewrite parse($a, $t) => 42 where $t is type` does match `parse("{}", User)`; this is a possible library spelling, not proof the requested generic form works.

`class User { id: number; name: string }; x = json("{\\"id\\":\\"7\\",\\"name\\":\\"Ada\\"}"); User{id: x["id"], name: x["name"]}.id` gives `'7'`: construction alone does not check a dynamically read field. Next try generating a validator and construction with a computed rule word for the two-argument spelling, with outcome tests for nested/list fields and exact diagnostics; investigate a general library rewrite of the generic symbol separately. Unsupported shapes must refuse rather than silently accept them. `lib/rule_words.warp` is baked, so smoke rebuilds it.

Reproduced with `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'class User { id: number; name: string }; parse<User>("{\"id\":\"7\",\"name\":\"Ada\"}")'`: `outside the core: the unknown name parse of User at 1:42 (no owner)`.

`WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'class User { id: number; name: string }; User("{\"id\":\"7\",\"name\":\"Ada\"}")'` returns `User{id: "{\"id\":\"7\",\"name\":\"Ada\"}", name: ""}`: ordinary construction does not validate JSON.

`lib/json.warp` parses JSON into data. `lib/sugar/reflection.warp` has rules using `$program`; class declarations and field types are collected in `core/passes/class_methods.warp` (Declared, fields_reaching). Next: investigate whether a library sugar rule and rule word can inspect the declaration from `$program` and generate a validator before concluding core support is required. Add an outcome test first, with a specification source, before implementation. Decide coercion and optional-field behavior only when needed; no default has been chosen yet.

Merged origin/main at 5d19b4d8c0; the original test still fails as above. Added a separate outcome test for the candidate `parse(text, User)` library spelling: valid input currently fails `outside the core: the call parse(…) at 1:48 (no owner)`; the error-only case is not yet proof of validation. A catch-all `rewrite $f($a) => 42 where $f is symbol` does not match `parse<User>(...)`, but the two-argument rewrite does match.

A direct runtime probe using `data = json(...)`, `if not (data["id"] is number) { raise "id must be number" }` and `User{id: data["id"], name: data["name"]}.id` produced `7` for numeric id and `id must be number at 1:122` for string id. An extra JSON field is silently ignored by construction, so validation must check keys. The library outcome test now additionally asserts a caught diagnostic `id must be number, received text`: it and the valid case currently fail at compile time with `outside the core: the call parse(…)`.
