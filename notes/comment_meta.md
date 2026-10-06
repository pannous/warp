# Comments as meta information (card g-1tHQ, 2026-10-06)

samples/comments.wasp says "Comments attach to the next element as metadata"; the playground user ran
`greeting.meta` and got `Error("no field meta@https")`.

- The parser already keeps a comment as a Meta layer `comment: "…"` on the first word of the next statement (lines of
  consecutive comments joined by newlines).
- meta_entries.rs (source pass): for a commented binding `x: …` / `x = …` / `x := …`, `x.@comment` is the comment
  text and `x.meta` the map `{comment: "…"}`, resolved at compile time (the value itself carries no meta at run time).
  A binding whose object has a field `meta` keeps `x.meta` the field. The last binding of a name wins.
- Assumption (queued with the Interviewer): `x.meta` is a map with plain keys (`x.meta.comment`), while the meta key
  form is `x.@comment` (wiki/meta.md). Not yet: annotations and line info in `x.meta`, comments on fields
  (`person.name.@comment`), comments of function definitions (`factorial.@comment`).
- `@https`: Firefox and Safari write stack frames as `name@url`; trap_error read the missing field's name allowing
  `@` anywhere (for `no_field_@source`), so it took `meta@https`. Now `@` counts only as the first character.
