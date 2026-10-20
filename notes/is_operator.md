# `is` is its own operator (card is-operator)

Only `is` tests a type (user decision #30): `3 is int` is yes, `3 == int` compares 3 with the type value int (no).

- Both parsers read `is` as `Op::Is` (Rust spellings.rs "is", parser.warp WORD_OPERATORS), never as `==`.
- Rust: `Op::Is` lives only while parsing. parse() (warp_parser/sugar.rs read_equality) makes `x is v` the unmarked
  `x == v`, which the passes read as `is` (a type test where v is a type word), and marks a type word on either side
  of `==`/`!=` as the type value compared (type_tests.rs equality_operand). The later passes see the trees they always
  saw. The front lane applies the same reading to parser.warp's tree (self_hosted.rs parse), and the bridge hands the
  mark on to the warp twin passes (node_views.rs BRIDGED_MARKS).
- Pure lane: `Op.Is` stays until type_tests.warp, which makes `x is T` of a type word T the program binds nowhere
  is_type(x, "T") and every other `x is v` `x == v`; `==` there is never a type test.
- Declared types (card enum-is-test): scope.warp gives `enum Coin {…}` and a type view (`class P {…}`) the meaning
  "type", so `x is Coin` is is_type(x, "Coin") too; plain_enums.warp turns it into `x == Penny or x == Dime …`.
  emitter.warp answers is_type of a class by instance_of (the instance's own class only), so a type with cases or
  subclasses (`enum Shape { Circle(r), Dot }`, `type Result = Ok(T) | Err(E)`, `class dog extends animal`) is refused
  until card pure-lane-shape, not answered no.
- `x is not 4`, `x isn't 4` are `not (x is 4)` in both parsers.
- Leftover (card operator-leftover): Rust marks only built-in type words, so `p == P` of a declared class or enum is
  still read as a type test.
