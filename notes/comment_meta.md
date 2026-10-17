# Comments as meta information (P114)

samples/hello/comments.warp: "Comments attach to the next element as metadata".

- The parser keeps a comment as a Meta layer `comment: "…"` on the first word of the next statement (lines of
  consecutive comments joined by newlines).
- meta_entries.rs (source pass): for a commented binding `x: …` / `x = …` / `x := …`, `x.@comment` is the comment
  text and `x.meta` the map `{comment: "…"}`, resolved at compile time (the value itself carries no meta at run time).
  A binding whose object has a field `meta` keeps `x.meta` the field. The last binding of a name wins.
- Only under the pragma `use comments` (diagnostic.rs in_program_mode,
  comments_as_meta); without it a read of a commented binding's meta is the error naming the pragma. Keys in `.meta`
  are plain (`x.meta.comment`), `@` reads directly (`x.@comment`).
- A comment between statements is the next statement's (lists.rs pending_comment, attached by parse_value).
- Not yet: annotations and line info in `x.meta`, comments on fields (`person.name.@comment`), comments of function
  definitions (`factorial.@comment`).
- Gotcha: Firefox and Safari write stack frames as `name@url`, so trap_error takes `@` in a missing field's name only
  as its first character (`no_field_@source`).
