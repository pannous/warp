# Parenthesized tuples

An explicit round comma list is a tuple value even when an element contains statements or control flow:

```warp
xs = ((try 1/0 else 0), 2)
xs # (0 2)
```

The parser and lowering passes already retain the outer `Bracket::Round` and `Separator::Comma`. Both type inference and list emission must use that pair as the value boundary: inference gives it `Kind::List`, and emission constructs every element instead of running the outer list as a statement sequence. Unbracketed comma-separated assignments remain statements.
