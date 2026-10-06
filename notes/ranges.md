# Ranges as values (card range-value, 2026-10-05)

A range is a loop header in `for i in a..b`, `a..b do {…}` and a slice in `xs#(a..b)`; anywhere else it is the list of its
numbers, as `x = 1..5` always was (wiki range.md: `print 1…5`):
- `print 1..5` prints [1 2 3 4], `str(1..5)` is "[1 2 3 4]", `type(1..5)` is `list of int`, `(1..5)#2` is 2;
  `1..5` excludes the end, `1…5` / `1 to 5` include it; an empty range (`5..1`) is ø.
- Literal bounds become the list literal; computed bounds (`print a..b`) the list a loop collects into the temporary
  `range_value·range` (analyzer lower_declarations_among, after for_loop has turned `for` headers into loops), the same
  as `xs = a..b` collects into `xs·range`. analyzer infer_type types a range List.
- `puts x` writes the text of any value without a line break (put_value), like `print` without it: before, anything
  but a text literal or a text variable wrote nothing (or the variable's name) and gave 0.
