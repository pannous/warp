# Open discussion topics

Topics to talk through with the user; the outcome goes to notes/decisions.md, the topic then leaves this file.

## Free functions vs methods (user 2026-10-20, for 2026-10-21)

The user: "we initially had the concept of making functions with classes and classes with methods identical, but for
some random functions like `argument_result(word: text)` these don't really seem to belong to the text class. They're
just contextually useful methods working on text."

Example: lib/compiler/signatures.warp `argument_result(word: text)` reads the typed library lines for a word name.
It takes a text, but it is no operation *of* text.

### Supervisor's opinion
Separate two things the current concept merges: how a function can be **called**, and which type it **belongs to**.

1. Calling stays uniform: `f(x)` and `x.f()` remain the same call for every function (type-library-words). That part
   of the concept is good and costs nothing.
2. Belonging is declared, not inferred from the first parameter. A function belongs to a type's surface (dir(text),
   completion, docs, the playground's method list) only when declared there: inside `class Text { … }` or the type's
   own module (lib/text.warp). Everything else is a free function of its module: callable with dot syntax where its
   module is used, invisible elsewhere (as P171: other modules' words need their `use`). So `word.argument_result()`
   works inside the compiler, but `dir(text)` never lists it and no user program sees it.
3. Often the real fix is a more honest parameter type: `argument_result` works on a word *name*, not on any text.
   A domain type (`class Name` or a type alias `Name = text`) makes it a method of Name, where it does belong, and
   keeps `text` clean.

Open questions for the discussion:
- Should dot syntax on a free function outside its declaring type earn a hint, or stay silent? (Supervisor: silent;
  the alias rule says equivalent spellings are fine.)
- Type aliases as their own method namespace (`Name = text` with its own methods) or only real classes?
