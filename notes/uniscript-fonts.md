# Uniscript fonts: pitfalls found while building them (see fonts/README.md)

- **Controls must follow their character.** Chrome and CoreText split text into runs by script; Common characters (TAG chars)
  join the *preceding* run. `α ⟨red⟩R` put the tag in the Greek run and R in a Latin one, so no GSUB rule saw both.
  Suffixes work everywhere, which is why Unicode's variation selectors, emoji tags and U+13440 are suffixes too.
- HarfBuzz uses only the **first feature of a tag per LangSys**: add lookups to the existing `ccmp`, never a second `ccmp` record.
- Never add new script records to an existing GSUB: a `hani` record without the font's features hides its DFLT features.
- CID-keyed CFF: a glyph's CID comes from its name `cidNNNNN` and must stay below 65536. Noto CJK already uses CIDs up to 65530, so reuse the gaps.
- CFF charstring width is stored relative to the private dict's `nominalWidthX`.
- Drop `hdmx`/`LTSH`/`VDMX` when adding glyphs, and `morx`/`feat` too: CoreText prefers AAT over GSUB.
- The fonts from Google have post format 3 (no glyph names). Switch to format 2 so hb-shape output is readable.
- A LigatureSet keyed on one IDS operator overflows 64 KB offsets, so chunk the ligatures into several subtables of one lookup; HarfBuzz tries the next subtable when one does not apply.
- Egyptian grouping already exists: NewGardinerOmni (Nederhof) implements the Unicode 15 format controls as a GSUB/GPOS state machine.
