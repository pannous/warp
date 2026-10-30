# typed-parse

State: investigating on branch typed-parse. No implementation or existing tests changed.

Default: no coercion; reject missing required and unknown fields, allow optional or defaulted fields (decision D4). A new outcome test covers valid input and the motivating type mismatch. Existing rewrite rule words can inspect `$program`; `class_view` and `class_declared` provide the declarations and field types. Next generate validation and construction through a computed rule word, including nested and list fields; unsupported shapes must refuse rather than silently accept them.

Reproduced with `WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'class User { id: number; name: string }; parse<User>("{\"id\":\"7\",\"name\":\"Ada\"}")'`: `outside the core: the unknown name parse of User at 1:42 (no owner)`.

`WARP_NO_WINDOW=1 /Users/me/dev/bin/warp 'class User { id: number; name: string }; User("{\"id\":\"7\",\"name\":\"Ada\"}")'` returns `User{id: "{\"id\":\"7\",\"name\":\"Ada\"}", name: ""}`: ordinary construction does not validate JSON.

`lib/json.warp` parses JSON into data. `lib/sugar/reflection.warp` has rules using `$program`; class declarations and field types are collected in `core/passes/class_methods.warp` (Declared, fields_reaching). Next: investigate whether a library sugar rule and rule word can inspect the declaration from `$program` and generate a validator before concluding core support is required. Add an outcome test first, with a specification source, before implementation. Decide coercion and optional-field behavior only when needed; no default has been chosen yet.
