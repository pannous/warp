# Single-statement block calls

A braceless library-word call in curly braces is a statement when the word and its argument make the intent clear:

```warp
if yes {sum [1 2]}
```

`lowering::nodes::is_spaced_call` is the shared recognition rule for ordinary and curly braceless calls.
`in_block_as_written` keeps the curly wrapper after resolving the call, so control flow evaluates the statement while
the block retains its source structure.
