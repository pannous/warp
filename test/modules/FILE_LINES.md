# file_lines(p) reads a file's lines; lines(t) splits a text (card lib-file-lines)

- `file_lines(path)` (lib/file, preferred): `split(read(path), "\n")`, so a final newline leaves a trailing `""`:
  `"a\nb\n"` gives `["a", "b", ""]` (test/modules/test_file_lines.pairs.warp).
- shell's `file_lines(path)` (lib/shell): drops one final newline, so `"a\nb\n"` and `"a\nb"` both give `["a", "b"]`
  (test/modules/test_file_path_edges.pairs.warp, test_shell_edges.pairs.warp).
- `lines(t)` (prelude/text): splits the text itself.

lib/file's `lines(path)` is a deprecated alias of `file_lines(path)`. For a text argument the prelude's `lines(t)` wins,
so under `use file`, `lines(p)` with a text `p` is `[p]`, the path and not its lines; call `file_lines(p)` instead.
