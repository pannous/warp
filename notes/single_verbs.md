# One verb per action: two-word words become single words

Roadmap card: single-verbs (board; the user said "don't start it yet")

## Why
Multi-dispatch (wiki/polymorphism.md) already chooses a word's meaning by the receiver's type, which is the first
argument. A noun inside a name repeats what that argument's type already says: `read_file(f)`, `file_lines(f)`,
`write_bytes(f, b)`, `save_picture(p, …)`, `play_sample(s)`, `format_date(t)`. So one verb per action is enough, and
the type of the first argument picks the meaning: `read f`, `lines f`, `write f, bytes`, `save p as: "x.png"`,
`play s`, `format t`. That means fewer words to learn, and `x.verb` reads naturally (`f.read`, `picture.save`).

## The rules for a new name
1. **An action is a verb, a property is a noun getter:** `read f`, `delete f`, `play s`; `lines f`, `size f`,
   `files folder`.
2. **A conversion or parse is `as`** (decision as-text-not-cast: `as` stays a cast): `t as json`, `t as date`,
   `t as duration`, `x as html`, `angle as degrees` (units, card length-pixels).
3. **The distinguishing type must come first.** A run-time choice tests only the receiver's type, never another
   argument's (wiki/polymorphism.md). If two meanings differ only in a later argument, that difference becomes a named
   parameter: `sort(xs, by: key)`, `split(t, at: 3)`.
4. **Type words stay reserved** (`list`, `text`, `file` as a conversion), so they can't be verbs: `list_files` becomes
   `files(folder)`, not `list(folder)`.
5. **Plumbing keeps its two words:** words `from host`, `from emitter`, `from runtime` or `from 'c'`, runtime words
   (`text_address`) and the parser's internal `read_*` helpers are no surface words. Only words a program calls change.

## The prerequisite: a file has a type
Today a path is plain text, so `lines "a.txt"` and `lines "a\nb"` collide: lib/file.warp already calls `lines(path)`
a deprecated alias, because the prelude's `lines(t)` wins. A verb can only dispatch on a file if files are values of
their own type:
- **Types:** `File` and `Folder` (lib/path.warp; shell.warp's `class Path` is folded in).
- **Literals:** a path literal (`./notes.txt`, `~/x.txt`, `/etc/hosts`, `file://…`; P183 already treats a file URL
  as one).
- **Conversions:** `"x.txt" as file`, `file "x.txt"`.
- **Words that return files:** `files(folder)`, `glob`, `program.file`.

## Steps (sub-cards of single-verbs)
| step | card | what |
|---|---|---|
| 1 | verbs-inventory | List the surface two-word words: lib/ has ~1457 top-level `a_b(` words without plumbing, most of them helpers. Mark each one public or helper and propose its single word (table below). One generated list goes in data/, the decisions go here. |
| 2 | file-type | `File`/`Folder` types, path literal, `as file`, words returning files; the text forms keep working |
| 3 | verbs-files | The files family (table), with old names kept as aliases |
| 4 | verbs-as-conversions | `parse_*`, `to_*`, `format_*` become `as` and `format(x)` by type; needs the time types (now() is time, done) |
| 5 | verbs-media | `save_picture` → `save p as:`, `play_sample` → `play`, the draw words → `draw shape` |
| 6 | verbs-text-lists | sort/find/replace/split/remove_* with named parameters |
| 7 | verbs-docs-samples | wiki, samples and playground use only the new words |
| 8 | verbs-aliases-gone | Remove the old names once nothing in lib/, samples/ or wiki/ uses them (tests: see the open questions) |

Each rename has three parts: the new word, the old name as an alias that warns once ("read_file is read(file) now"),
and one outcome test of the new spelling. Existing tests stay as they are and pass through the alias.

## The rename table (first pass, step 1 completes it)
| now | becomes |
|---|---|
| read_file(path), read_bytes | read(file), read(file) as bytes |
| write_file(path, t), write_bytes(path, b) | write(file, t), write(file, b) (by content type) |
| append_file(path, t) | append(file, t) |
| file_lines(path), text_lines(t) | lines(file), lines(t) |
| file_size(path) | size(file) (a folder: the sum) |
| list_files(folder), find_files | files(folder), find(folder, pattern) |
| delete_file(path) | delete(file) |
| path_join(a, b), path_parts | folder / name, parts(path) |
| save_picture(p, w, h, f) | save p as: f (the save pattern, decision save-sound) |
| play_sample(s), play_blocks | play(s) |
| draw_shape, draw_shader (surface forms) | draw(shape), draw(shader) |
| format_date, format_time, format_duration, format_number, format_list, format_table | format(x) by type, a pattern as a named parameter |
| parse_json, parse_date, parse_duration, parse_iso_duration | t as json / date / duration |
| to_json, to_html, to_degrees, to_radians | x as json / html / degrees / radians |
| to_camel, to_snake, to_kebab, to_pascal | t as camel / snake / kebab / pascal (case styles as `as` targets) |
| sort_by(xs, k), sort_lines, sort_object | sort(xs, by: k), sort(lines t), sort(map) |
| replace_all, replace_matches | replace(t, a, b), where a regex `a` replaces its matches |
| find_all, find_matches | find(t, pattern) (all matches) |
| split_at(t, i) | split(t, at: i) |
| remove_prefix, remove_suffix | open: see question 4 |
| copy_text, new_list | copy(t), list() |

## Open questions: ANSWERED by the user (notes/decisions.md single-verbs questions): 1 yes as recommended, 2 as recommended, 3 surface now and helpers later as an optional card, 4 remove_prefix/remove_suffix stay, 5 size
1. **Does text still name a file?** Recommendation: yes for `read`, `write`, `exists` and `delete`, as today, where
   text has no other meaning. Words where text already means something else (`lines`, `size`, `append`, `find`,
   `sort`) need a `File`.
2. **Alias lifetime:** recommendation: the old name warns until the next release, then goes. The few existing tests
   that use an old name are upgraded at that point with the user's OK (tests are append-only).
3. **Scope:** recommendation: surface words only. Helpers and plumbing (rule 5) keep their names; renaming those is
   cleanup, not language.
4. **`remove_prefix` / `remove_suffix`:** `trim(t, start: "x")` / `trim(t, end: "x")`, or keep them as they are?
   Recommendation: trim with a named parameter.
5. **`size` or `count` for a file?** Recommendation: `size` (bytes); `count` stays the item count of lines and lists.
