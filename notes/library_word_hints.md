# Library-word hints

When a program defines an untyped function with the same name as a typed library word, Warp keeps the library word for its declared types and uses the program's function for other values.

The `library-words-typed` hint must name both sides of that dispatch and give the actionable preference: choose a different function name when the program's definition should handle the library type too.

`tests/welcoming/test_library_word_hint.rs` pins the complete explanation through diagnostic capture, while `tests/types/test_one_dispatch.rs` keeps the dispatch result covered.
