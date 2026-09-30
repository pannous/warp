# Uniscript fonts

`uniscript_fonts.py` builds fonts in which invisible TAG characters (U+E0020–E007E) turn the character **before** them
into a mirrored, rotated or colored variant, and in which Ideographic Description Sequences compose to their character.

```
python3 fonts/uniscript_fonts.py [sans|cjk|egyptian|mirror|all] [--install]   # → fonts/dist/
python3 probes/uniscript_fonts_test.py                                        # shapes with hb-shape
```

Sources are the fonts in `~/Library/Fonts` and `/System/Library/Fonts`; downloads (IDS data, NewGardinerOmni) are cached in `fonts/sources/`.
`dist/` and `sources/` are not committed: Monaco and Menlo are Apple fonts, only for local use.

## Controls

A control is TAG + ASCII letter, i.e. U+E0000 + the letter, and follows its character: `A` U+E0072 = red A.
One geometry and one color may be combined in either order: `A` + mirror + red = `A` + red + mirror.

| geometry | tag | | color | tag | | color | tag |
|---|---|---|---|---|---|---|---|
| mirror | `M` E004D | | red | `r` E0072 | | orange | `o` E006F |
| flip (vertical) | `F` E0046 | | green | `g` E0067 | | yellow | `y` E0079 |
| turn (180°) | `T` E0054 | | blue | `b` E0062 | | black | `k` E006B |
| left (90° ccw) | `L` E004C | | brown | `n` E006E | | white | `w` E0077 |
| right (90° cw) | `R` E0052 | | pink | `p` E0070 | | gray | `a` E0061 |
| | | | purple | `v` E0076 | | | |

Tags are default-ignorable: without these fonts the text shows the plain characters.

## Fonts

| font | base | effects |
|---|---|---|
| **Uniscript Sans** | Noto Sans (Latin, Greek, Cyrillic) + Noto Sans Math | ASCII + Greek letters: all geometries × all colors; Latin-1/Ext-A, Greek, symbols, arrows, operators: every geometry or color, not combined; the rest, including math alphanumerics 𝔄𝕕: mirror and turn |
| **Uniscript CJK** | Noto Sans CJK | IDS composition (27,688 sequences from cjkvi-ids, nested ones too: ⿱木⿰木木 → 森); mirror for radicals, strokes and the 3,755 most common hanzi (GB 2312 level 1) |
| **NewGardinerOmni2d4** | Nederhof's NewGardiner, OFL, unchanged | Unicode 15 Egyptian format controls: joiners U+13430 vertical / U+13431 horizontal, insertions, U+13440 mirror, groups up to depth 4 |
| **… Mirror** | NFM-Indus Script (Sublime), JetBrains Mono (JetBrains IDEs), Monaco (iTerm), Menlo (VS Code) | mirror for every character, advance kept, so monospace stays monospace |

## Limits

- OpenType allows 65,535 glyphs. Uniscript Sans uses 50,159, CJK 64,975 (61,001 of them from Noto), and Omni uses 38,238 for its group state machine.
  Hence the tiers above, no mirror for rarer hanzi, and no per-sign colors for hieroglyphs.
- IDS only compose to characters that exist. Composing new layouts would need pre-scaled copies of every component (what Omni does for hieroglyphs), which the CJK glyph budget does not allow.
- Terminals and editors must shape with HarfBuzz or CoreText for tags to take effect, e.g. iTerm with ligatures enabled, Sublime, VS Code. Chrome and Firefox render the colors (COLR v0). The results were verified in headless Chrome.
