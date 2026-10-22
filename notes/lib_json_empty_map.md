# Empty maps in JSON

`to_json({})` and `to_json({a: {}})` currently produce `null` because the compiled value of `{}` reaches the JSON adapter as `Node::Empty`; `type({})` likewise reports `empty`.

The JSON adapter already serializes `Node::List(_, Bracket::Curly, _)` as an object, including an empty object. The general fix is therefore to preserve the empty map as a map through compilation. That representation is owned by the active `code-empty` branch, so `lib-json` should be retried after that branch lands instead of adding a JSON-only exception.
