#!/usr/bin/env python3
"""scripts/unicode-operator-names.py: write each operator row's `names` in lib/aliases.warp, the Unicode character
names of the one-character symbols it spells (`≤` less_than_or_equal_to, `·` middle_dot), as fallback aliases
(decision operator-unicode-aliases). Run once after a symbol joins the table; the compiler holds no Unicode database.
Only the row's own symbol and its math glyphs (Unicode category Sm: ≤ ×, never the dash and arrow look-alikes), each
name once. Keyword rows (quoted words: and, or, not, to) get none: their word aliases become keyword forms."""
import re
import sys
import unicodedata
from pathlib import Path

TABLE = Path(__file__).resolve().parent.parent / "lib" / "aliases.warp"
# an entry line: its word (bare, or a quoted symbol) and its fields
ROW = re.compile(r'^(?P<word>[A-Za-z_][A-Za-z_0-9]*|"[^"]+") \{ (?P<fields>.*) \}(?P<rest>.*)$')
QUOTED = re.compile(r'"([^"]*)"')
NAMES_FIELD = re.compile(r', names: \[[^\]]*\]')
MATH_SYMBOL = "Sm"


def character_name(symbol: str):
    if len(symbol) != 1:
        return None
    name = unicodedata.name(symbol, "")
    return name.lower().replace(" ", "_").replace("-", "_") or None


def spelled_symbols(word: str, fields: str) -> list:
    """the symbols a row spells: its operator, the row's own key when quoted, its symbol aliases that are math glyphs"""
    symbols = [word.strip('"')] if word.startswith('"') else []
    operator = re.search(r'(?:prefix-)?operator: "([^"]*)"', fields)
    if operator:
        symbols.append(operator.group(1))
    aliases = re.search(r'aliases: \[([^\]]*)\]', fields)
    if aliases:
        symbols += [alias for alias in QUOTED.findall(aliases.group(1)) if len(alias) == 1 and unicodedata.category(alias) == MATH_SYMBOL]
    return symbols


def named_row(line: str, taken: set) -> str:
    row = ROW.match(line)
    if not row or row.group("word").startswith('"') and row.group("word").strip('"').isalpha():
        return line
    word, fields = row.group("word"), NAMES_FIELD.sub("", row.group("fields"))
    names = []
    for symbol in spelled_symbols(word, fields):
        name = character_name(symbol)
        if name and name not in taken:
            names.append(name)
            taken.add(name)
    if not names:
        return f'{word} {{ {fields} }}{row.group("rest")}'
    return f'{word} {{ {fields}, names: [{", ".join(names)}] }}{row.group("rest")}'


def main() -> int:
    lines = TABLE.read_text().split("\n")
    # every word the table writes already, never taken again as a name
    taken = set(re.findall(r'\b[a-z_][a-z_0-9]*\b', NAMES_FIELD.sub("", "\n".join(l for l in lines if not l.startswith("#")))))
    TABLE.write_text("\n".join(named_row(line, taken) for line in lines))
    return 0


if __name__ == "__main__":
    sys.exit(main())
