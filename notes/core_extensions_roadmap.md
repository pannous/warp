# Core extensions roadmap (decision core-is-self, card core-extensions-roadmap)

The core is what the compiler needs to compile itself; every other feature becomes an extension, baked in and
registered in one static hook table.

## Hook table design

One static table in the core, filled at build time from the extensions' own files (no dynamic loading):
1. **operator rows**: lib/aliases.warp's entries for the core's operators, each extension's own rows in its file
   (symbol, aliases, power [left, right], fixity infix | prefix | suffix | circumfix with closing); the parser reads
   them as one table.
2. **name → handler**: keywords and statement heads (class, c-style for, times, named loops, try/assert/check),
   raw blocks (shader, wasm), text prefixes (sql, sh), commands (repl, build native, test), library words (gpu_zeros …):
   wasp's analyze() dispatch as a table.
3. **list kinds**: mark bit → {at, count, set, items} (ranges 1, GPU lists 2), KindUnit bits as columns.
4. **one registration per extension**: its rules file, operator rows, atom readers (glyph or suffix → reader of
   the literal), passes in the pass list, runtime sources and host exports for runtime.wasm.

Built: core/extension.warp `class Extension` (each field documented there), the rows in extensions/extensions.warp;
the emitter merges each row's library words and constant tables into its own and exports its host words. List kinds:
KindUnit.ListKind is a two-bit field (ListKind.Range 1, GPU lists 2); the core list words test only "an extension's
kind" and call runtime/bridge.warp's extended_list_* hooks, whose bodies extensions/runtime.warp writes (the list-kinds
table). Runtime sources: extensions/runtime.warp, used by core/runtime/extended.warp, joins each extension's runtime
file.

Row pitfalls (each cost a failed seed build):
- An extension's file never takes a std module's name (rewrites.warp std_module_names): `use "tasks.warp"` in
  extensions/extensions.warp loads lib/tasks.warp, so the tasks row lives in extensions/go_tasks.warp.
- A row's map keys are texts, `value_settlers: {"task_go": "task_awaited"}`: a bare key `{task_go: …}` is a free word
  to prelude_words, which brings the module defining it (lib/tasks.warp, and uncertain behind it) into the compiler.
  That is a compiler bug (a key read as a word, card key-map), not a style rule; the quotes hold until it is fixed.
- A function gathering a field is named apart from it: `extension_block_words()` for `block_words`.
- An extension part names no core path, so a top-level value it reads at load (`MARKED_NUMBER_KINDS = {int: Kind.Int}`)
  needs the core file defining that name used before the extension (definitions.warp uses kinds.warp first; card
  int-types-extraction: the seed failed "unknown name Kind").
