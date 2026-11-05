# lines(p) is the text's lines; file_lines(p) reads the file (documented only; card lib-shell-edges)

Under `use file`, `lines(p)` with a text `p` resolves to the prelude's `lines(t)`, which splits the text itself:

```warp
use file
p = temp_folder() + "/notes.txt"
write(p, "a\nb\n")
lines(p)        // ["/tmp/notes.txt"]: the path, not its lines
```

lib/file's `lines(path)` is shadowed for a text argument. To read a file's lines use lib/shell's `file_lines(path)`,
which drops one final newline: `write(p, "a\nb\n")` and `write(p, "a\nb")` both give `["a", "b"]`, and `"a\n\n"` gives
`["a", ""]` (test/modules/test_file_path_edges.pairs.warp, test_shell_edges.pairs.warp).

No rename and no new overload here: the lines/file_lines API cleanup is parked until it is decided.
