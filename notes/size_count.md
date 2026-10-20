# `size` as the global count alias

`lib/prelude.warp` declares `alias size = count`. The analyzer reads counting aliases from the prelude, so `size(xs)` uses the same implementation as `count(xs)` for lists, text, and maps.

The alias is a fallback. A program-defined `size` function remains available for values that do not match the built-in collection overloads. Focused outcome coverage lives in `tests/functions/test_size_count_alias.rs`.
