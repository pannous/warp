# Annotations `@name` / `@name(value)`

- **Leading** `@version(2) @draft tee{a:1}`: annotates the atom that follows it (`WaspParser::parse_attribute`).
- **Trailing** `ys = xs.map(f) @parallel` (wiki/Purpose.md, card parallel-map): an annotation with nothing after it on
  the statement annotates the expression before it, here the map call. "Nothing after it" means the end of input, a
  newline, `;`, `,` or a closing bracket. It annotates only a whole statement or a whole assigned value, never an
  operand inside it. Written by `WaspParser::try_parse_trailing_attribute`, from the expression loop.
- **Inside a literal** `point{x:1 @source:"gps"}`: the meta entry `@source`, never a field.

Read annotations with `node.attribute("parallel")`; `node["@name"]` sets them (open_decisions.md, P on metadata).
