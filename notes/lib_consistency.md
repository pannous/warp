# lib/ consistency audit (card lib-consistency, report only, 2026-11-06)

56 names are defined at the top of more than one lib/*.warp module (a regex over definition heads, `name(…) :=`,
`name =`, `def name`). Sorted by what they are; nothing changed by this audit, each finding a candidate card.

## Fine: typed overloads of one meaning
- sin cos tan exp (math, reals, fast_math), atan2 pow (math, fast_math), max min (math, extrema), log (math, prelude),
  sqrt cbrt abs hypot pow (math and c): the same function on other types or as C spells it (lib/c.warp groups the C
  twins, card c-entries-sort).
- print (prelude, primitive), write (file, web, primitive), read (prelude, web), paint clear (draw, prelude),
  listen (prelude, web), slice (prelude, memory), filter (prelude, sound), task_context / task_context_back (prelude,
  sound), slept sleep (prelude, tasks, units), map_values (map, prelude): one word, an overload for the module's type.

## Duplicated declarations: one home would do
- C functions declared again in the module that calls them: fopen fclose setenv (c, os), fputs fflush getpid
  (c, sound), unlink (file, sound), mkdir (sound's C `mkdir(path, mode)` beside shell's `mkdir(folder)`), puts (c,
  prelude). Each module could `import … from "c"` the line of lib/c.warp instead of repeating it.
- `HEX_DIGITS = "0123456789abcdef"` in encoding and json; `big_endian(n, size)` in hash and sound (two spellings of the
  same bytes; decision pixels-bytes makes `n.bytes(big_endian)` the canonical form, so both could go to it).

## Real clashes: one name, two meanings
- hex_digit: hash's is a character's value (text → int), uuid's a value's character (int → text): opposite
  directions under one name.
- padded: hash's pads a message to a block (SHA padding), time's pads a number with zeros to a width.
- parse_pattern: time's reads a date pattern, sound's a drum-step pattern.
- sample: random's picks k items, sound's loads a sound file.
- transpose: matrix's swaps rows and columns, sound's shifts a pitch by semitones.
- scale: matrix's scales a matrix, sound's is a musical scale.
- unique: list's distinct items, shell's the distinct lines of a file (Path).
- lines: text's splits a text, file's reads a path's lines (`lines("~/x")` reads a file, `lines(t)` splits it: the
  type tells them apart only when the argument's type is known).
- display_width: prelude's the display's pixel width, tables' the printed width of a text (columns).
- milliseconds_of: units' takes a Quantity, time's anything; decision library-word-visibility replaces it by typed
  conversions (`q as Duration`).
- mix: math's linear interpolation (alias of lerp), sound's adds signals.
- beats (extrema, sound), save (draw, file, sound), show (color, draw, units), find (prelude, shell), sort (prelude,
  shell), run (os, wasmtime): overloads by type mostly; find and sort in shell take a Path and so are the shell's.

Under decision library-word-visibility (untyped library words private by default) most clashes between untyped
helpers stop colliding; the typed ones above (hex_digit, padded, transpose) still meet in a program using both
modules and are refused there, so a rename (hash: hex_value only; sound: transposed, pattern_steps, sample_file) is
the cleaner way.

## Naming style
Two names not in snake_case, both unit spellings: dB (sound), kHz (units). Fine as unit symbols.
