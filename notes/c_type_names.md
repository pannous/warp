# C's type names of several words (card unsigned-int)

`x: unsigned int = 3` used to misparse quietly (`x: unsigned` and `int = 3`). Now a C type name of several words is
one type word wherever a type stands (`x: …`, `unsigned int y = 3`, parameters), the fixed width C means
(decision D16's FFI types), with a got-it note naming warp's word:

| C | warp |
|---|---|
| unsigned int | uint32 |
| unsigned long, unsigned long long | uint64 |
| long long, long int, long long int | int64 (wraps at 2^63, decision i64-wraps) |
| signed int | int32 |
| unsigned short / short int | uint16 / int16 |
| unsigned char / signed char | byte / int8 |

Single words keep warp's meaning: `int`, `long` are the unbounded int. The table is COMPOUND_TYPES
(src/warp_parser/types/expressions.rs, twin in core/parsing/parser.warp), beside warp's own `fast int` / `fast float`.
User decision c-int-names: C's fixed widths with a got-it note, not plain unbounded `int`.
