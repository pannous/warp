# Comments as meta information (card g-1tHQ, P114, 2026-10-06)

samples/comments.wasp says "Comments attach to the next element as metadata"; the playground user ran
`greeting.meta` and got `Error("no field meta@https")`.

- The parser already keeps a comment as a Meta layer `comment: "…"` on the first word of the next statement (lines of
  consecutive comments joined by newlines).
- meta_entries.rs (source pass): for a commented binding `x: …` / `x = …` / `x := …`, `x.@comment` is the comment
  text and `x.meta` the map `{comment: "…"}`, resolved at compile time (the value itself carries no meta at run time).
  A binding whose object has a field `meta` keeps `x.meta` the field. The last binding of a name wins.
- P114 (user): "the meta keyword or attribute should persist and be filled with the comments when we activate them
  but usually they should be deactivated": only under the pragma `use comments` (diagnostic.rs in_program_mode,
  comments_as_meta); without it a read of a commented binding's meta is the error naming the pragma. Keys in `.meta`
  are plain (`x.meta.comment`), `@` reads directly (`x.@comment`).
- The parser kept a comment only before a program's first statement: one between statements was dropped
  (lists.rs). It is now the next statement's (pending_comment, attached by parse_value). Not yet: annotations and line info in `x.meta`, comments on fields
  (`person.name.@comment`), comments of function definitions (`factorial.@comment`).
- `@https`: Firefox and Safari write stack frames as `name@url`; trap_error read the missing field's name allowing
  `@` anywhere (for `no_field_@source`), so it took `meta@https`. Now `@` counts only as the first character.
