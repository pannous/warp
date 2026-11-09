# The path type (decision path-type, card path-literal-type)

A path written without quotes, `./notes.txt`, `~/x`, `/etc/hosts`, `../x`, `^/x`, is a value of type `path`: a text
marked in its kind field the way a yes/no is a marked Int (Marked.Path = TextMark.Path 1 above Kind.Text).

- The parser reads `./a` as `"./a" as path`; `t as path` marks any text, shell's `Path(t)` too.
- Every text word takes a path (`#(./abc)`, `"dir: " + ./docs`); `x is text` holds, `x is path` only for the mark;
  `type(./a)` is path.
- Dispatch: path is narrower than text, so `f(p: path)` beside `f(t: text)` takes the path (shell's sort, unique,
  find, size of a file beside the prelude's text words).
- Unboxed the mark rides statically on TypedCode.mark (decision marks-unboxed-first); boxed it is the node's kind,
  tested at run time through HeldBit.Path.

Next (notes/single_verbs.md step 2, card file-type): File and Folder build on this mark, so the single verbs
(`lines f`, `size f`, `read f`) dispatch on the receiver's type. The runtime spells the mark by an older seed's rows
(core/runtime/types.warp) until the next scripts/refresh-seed.sh.
