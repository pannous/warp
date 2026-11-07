# A linked-module find-variant failure (solved, card typed-overload-replace, 60d16a5e36)

`use toml; use json_tools` (and `use text`, `use shell` with a text pattern `match`) failed with
`(find rest (str next))` fits no single variant of find(…) in the prelude's `fits_text_cut`. The cause was not the
linking: the dispatch judged a local variable `rest` a function because text's `Cursor.rest()` shares its name.
known_kind (core/passes/arity_overloads.warp) now takes an assigned name for a variable. Test:
test/modules/test_cross_module_overloads.pairs.warp.
