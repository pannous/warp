# Inspiration: strings and data formats

Verified 2026-09-26 with `probes/footguns/inspiration/scratch/strings_dataformats.py`, `strings.swift`, `s.go`, `s.rs`
and `node` one-liners (python 3 + PyYAML 6.0.1, Ruby Psych, Swift, Node 26, Go, rustc).
Extra finding for Footguns.md: YAML 1.1 (PyYAML `safe_load` and Ruby Psych both) reads `zip: 01234` as **668** (octal), not 1234.
pandas `read_csv` without `dtype` gives 1234.

### Bytes vs characters vs graphemes
Solved elsewhere: **Swift**: `"👍🏽".count` → `1` (and `.utf8.count` 8, `.utf16.count` 4, `.unicodeScalars.count` 2). The default `Character` is an extended grapheme cluster, the other units are separate named *views*, and `String.Index` is opaque, so `s[2]` does not compile.
**Rust**: `"👍🏽".len()` → `8` bytes, `.chars().count()` → `2`. Slicing at a non-char boundary panics (`is_char_boundary(1)` → false), it never returns half a code point. Graphemes need a crate (`unicode-segmentation`).
**JS (Intl.Segmenter)**: `[...new Intl.Segmenter().segment("👍🏽")].length` → `1`. It came late and is opt-in, while `.length` stays 4.
Adopt in Warp: use Swift's model. Text has named views `bytes`, `codepoints`, `graphemes`, and the default `#`, `for c in text` and `count` work on graphemes. Byte access is only possible through the `bytes` view (or `[]` if Warp keeps that split). An index that is not on a boundary is an error value, never a partial character like `'Ã'`.

### The Norway problem
Solved elsewhere: **TOML 1.0**: `country = NO` → *parse error*, so it has to be written `"NO"`. The only booleans are lowercase `true`/`false`, bare words are not values, and strings must be quoted.
**YAML 1.2 core schema / StrictYAML**: 1.2 cut the boolean set down to `true|false` (in any case); StrictYAML treats every scalar as a string until a schema says otherwise. (PyYAML and Ruby Psych still use 1.1: `NO` → `false`, verified.)
**JSON**: `true`/`false` only; a bare `NO` is a syntax error.
Adopt in Warp: data literals should accept only `true`/`false` (plus `✔`/`✖` if they are wanted) as booleans. `yes`/`no`/`on`/`off` should be symbols in data. In code, a symbol reaches `Bool` only through an expected-type coercion that the elaborator records explicitly. The value then depends on the schema, not on the spelling.

### Numbers that are not numbers
Solved elsewhere: **TOML 1.0**: `zip = 01234` → *parse error*, because leading zeros are forbidden. That makes `"01234"` the only way to write it, so a leading zero can never be silently dropped.
**Python json**: `json.loads('{"v":1.10}', parse_float=Decimal)` → `Decimal('1.10')`. The decimal keeps its trailing zero because the parser takes a hook for number construction.
**JS (Node 26, JSON.parse source text access)**: `JSON.parse('{"v":1.10}', (k,v,ctx)=>ctx.source ?? v)` → `{v:'1.10'}`. The reviver sees the original lexeme; `JSON.rawJSON` does the same when serializing.
Adopt in Warp: every numeric literal in `Meta` keeps its source lexeme, so serialization round-trips `1.10` and `01234` byte for byte. When no numeric type is expected, a lexeme with a leading zero is a symbol/text with a diagnostic, as in TOML, not an octal or truncated number. This matches DESIGN.md's rule that `Node` preserves "data literals exactly enough for round-trip serialization".

### Data that executes
Solved elsewhere: **PyYAML**: `yaml.safe_load("!!python/object/apply:os.system ['echo pwned']")` → `ConstructorError`. The safe loader has no constructor for language-object tags, so a tag cannot produce code (plain `yaml.load` without `SafeLoader` used to run it).
**JSON / Rust serde**: `serde_json::from_str::<Config>(s)` produces only the declared type. The format has no tags that could name executable things, and the target type is fixed by the caller.
**Deno**: `deno run script.ts` with no `--allow-*` flags means the code cannot read files, use the network or read env. Capabilities are deny-by-default and granted per run.
Adopt in Warp: add a `warp read file.wasp` / `load(text)` entry point that stops at `Node` and cannot evaluate, and make it the default for data files. Evaluating foreign code should run as a WASM component with an empty import set, plus only the capabilities the caller grants explicitly (the WIT imports backstop in DESIGN.md → Effects as enforced capabilities).

### Date guessing in data
Solved elsewhere: **TOML 1.0**: `d = 2001-12-14` → `date(2001,12,14)`, while `gene = "SEPT2"` → `'SEPT2'`. Dates are a separate literal grammar (RFC 3339 only) and are never guessed from strings.
**pandas**: `read_csv(..., dtype=str)` → `{'gene':'SEPT2','zip':'01234'}`. Date parsing is off unless `parse_dates=` is passed. Without `dtype`, the zip still turns into `1234`.
Counter-example worth citing: **JS**: `new Date("SEPT 2")` → *Sun Sep 02 2001* (V8 fills in the year 2001 on its own); only ISO `2001-12-14` is specified.
Adopt in Warp: keep today's rule (`SEPT2` stays a symbol). If date literals are added, accept only an unambiguous RFC 3339 / ISO 8601 form, as TOML does, and parse anything else as a date only when a schema expects one.

### "The" length of a string
Solved elsewhere: **Swift**: `s.count` / `s.utf8.count` / `s.utf16.count` / `s.unicodeScalars.count` → `1/8/4/2` for `"👍🏽"`. Every unit has a name, and only the grapheme count gets the short name.
**JS Intl / ICU**: `"i".toLocaleUpperCase("tr")` → `İ`, while `"i".toUpperCase()` → `I`. Locale-dependent case mapping takes the locale as an explicit argument; the default is locale-independent.
**Python**: `unicodedata.unidata_version` exposes the Unicode version, so the segmentation rules can at least be seen and pinned.
Adopt in Warp: a string has no `length`. It has `bytes.count`, `codepoints.count` and `graphemes.count` (`count` means graphemes), and case mapping takes an explicit locale or defaults to the invariant locale. The Unicode version used for segmentation should be recorded in the semantic artifact so results are reproducible across builds.
