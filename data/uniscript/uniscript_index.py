#!/usr/bin/env python3
"""Uniscript entity data: seed the readable entity file, build its binary index, check that both agree.

    python3 data/uniscript/uniscript_index.py seed    # sources → entities.wasp (overwrites it!)
    python3 data/uniscript/uniscript_index.py build   # entities.wasp → entities.idx
    python3 data/uniscript/uniscript_index.py check   # every entry of entities.wasp resolves the same in entities.idx

entities.wasp is the source of truth once seeded: edit it, then build. The index format is documented in README.md.
"""
import html.entities
import re
import struct
import sys
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
ENTITIES_FILE = HERE / "entities.wasp"
INDEX_FILE = HERE / "entities.idx"
UNICODE_MATH_TABLE = Path("/usr/local/texlive/2026basic/texmf-dist/tex/latex/unicode-math/unicode-math-table.tex")

MAGIC = b"USX1"
HASH_MULTIPLIER = 31
HASH_MODULUS = 1 << 32
RECORD_FIELDS = 5  # hash, key offset, key length, value offset, value length: all u32 little endian
TABLE_NAMES = ("names", "chars", "suffixes")  # forward lookup, reverse lookup, suffix control → block type

# sections holding plain entities, earlier ones win when a name occurs twice
ENTITY_SECTIONS = ("uniscript", "names", "latex", "html")
# the uniscript section: escapes of the '<:' marker (wiki/uniscript.md "Special remark") and short spec names
UNISCRIPT_NAMES = {"less": "<", "colon": ":", "greater": ">", "empty": "∅"}

ALGORITHMIC_NAMES = re.compile(r"^(CJK UNIFIED IDEOGRAPH|CJK COMPATIBILITY IDEOGRAPH|HANGUL SYLLABLE|TANGUT IDEOGRAPH|"
                               r"TANGUT COMPONENT|KHITAN SMALL SCRIPT CHARACTER|NUSHU CHARACTER|EGYPTIAN HIEROGLYPH-)")

# block type ← Unicode name patterns; the captured rest names the plain character the block maps from
NAME_BLOCKS = [
	("bold", [r"MATHEMATICAL BOLD (.+)"]),
	("italic", [r"MATHEMATICAL ITALIC (.+)"]),
	("bold-italic", [r"MATHEMATICAL BOLD ITALIC (.+)"]),
	("script", [r"MATHEMATICAL SCRIPT (.+)", r"SCRIPT (CAPITAL .+|SMALL .+)"]),
	("bold-script", [r"MATHEMATICAL BOLD SCRIPT (.+)"]),
	("fracture", [r"MATHEMATICAL FRAKTUR (.+)", r"BLACK-LETTER (CAPITAL .+|SMALL .+)"]),
	("bold-fracture", [r"MATHEMATICAL BOLD FRAKTUR (.+)"]),
	("double", [r"MATHEMATICAL DOUBLE-STRUCK (.+)", r"DOUBLE-STRUCK (CAPITAL .+|SMALL .+)"]),
	("sans", [r"MATHEMATICAL SANS-SERIF (.+)"]),
	("sans-bold", [r"MATHEMATICAL SANS-SERIF BOLD (.+)"]),
	("sans-italic", [r"MATHEMATICAL SANS-SERIF ITALIC (.+)"]),
	("sans-bold-italic", [r"MATHEMATICAL SANS-SERIF BOLD ITALIC (.+)"]),
	("monospace", [r"MATHEMATICAL MONOSPACE (.+)"]),
	("upper", [r"SUPERSCRIPT (.+)", r"MODIFIER LETTER (.+)", r"(.+ )SUPERSCRIPT (.+)"]),
	("lower", [r"SUBSCRIPT (.+)", r"(.+ )SUBSCRIPT (.+)"]),
	("small-capital", [r"LATIN LETTER SMALL CAPITAL (.+)"]),
	("circled", [r"CIRCLED (.+)", r"NEGATIVE CIRCLED (.+)"]),
	("parenthesized", [r"PARENTHESIZED (.+)"]),
	("squared", [r"SQUARED (.+)"]),
	("fullwidth", [r"FULLWIDTH (.+)"]),
	("reversed", [r"(.+ )REVERSED (.+)"]),
	("turned", [r"(.+ )TURNED (.+)"]),
]
# Unicode leaves holes in Mathematical Alphanumeric Symbols where a Letterlike Symbol already existed
LETTERLIKE_HOLES = {("italic", "h"): "PLANCK CONSTANT"}
# the standard Greek keyboard layout (ELOT 1000 / Windows Greek) as block type 'greek'
# block type 'greek': phonetic transliteration; letters without a clear counterpart (c h j q v w y) have none,
# so uniscript warns about them instead of guessing; the other letters by name: <:greek eta>, <:greek Omega>
GREEK_LETTERS = dict(zip("abgdezikl" "mnxoprstuf", "αβγδεζικλ" "μνξοπρστυφ"))
GREEK_DIGRAPHS = {"th": "θ", "ch": "χ", "ps": "ψ"}
GREEK_NAMED = re.compile(r"GREEK (SMALL|CAPITAL) LETTER ([A-Z]+(?: [A-Z]+)?)$")
GREEK_NAMED_RANGE = range(0x391, 0x3CA)
GREEK_SPELLINGS = {"lamda": "lambda"}  # Unicode spells it lamda
COLORS = {"red": "r", "green": "g", "blue": "b", "brown": "n", "pink": "p", "purple": "v", "orange": "o",
          "yellow": "y", "black": "k", "white": "w", "gray": "a"}
GEOMETRIES = {"mirror": "M", "flip": "F", "turn": "T", "left": "L", "right": "R"}
TAG_BASE = 0xE0000
EMOJI_START = 0x1F000
EGYPTIAN_MIRROR = "\U00013440"  # the Unicode 15 format controls for hieroglyphs: mirror, vertical and horizontal joiner
EGYPTIAN_VERTICAL_JOINER = "\U00013430"
EGYPTIAN_HORIZONTAL_JOINER = "\U00013431"
IDS_ABOVE_TO_BELOW = "\u2FF1"
IDS_LEFT_TO_RIGHT = "\u2FF0"
VARIATION_SUFFIXES = {"iconic": "\uFE0F", "plain": "\uFE0E"}
BLOCK_ALIASES = {"fraktur": "fracture", "double-struck": "double", "superscript": "upper", "subscript": "lower",
                 "reverseInPlace": "reversed", "reverse": "mirror", "grey": "gray", "emoji": "iconic", "text": "plain"}
SUFFIX_KEY = "*suffix"  # follows any character without its own entry; "*suffix egyptian" only hieroglyphs
PREFIX_KEY = "*prefix"  # "*prefix cjk": goes before the parts of a group (an IDS operator)
GROUP_KEY = "*group"    # the block joins its operands (above, beside) instead of styling them
INFIX_KEY = "*infix"    # "*infix egyptian": goes between the parts of a group (a hieroglyph joiner)
PLAIN_CATEGORIES = "LNPS"  # letters, numbers, punctuation, symbols: no marks, controls or separators in block tables
LETTER_LIGATURES = "AE|DZ|LJ|NJ"  # Unicode calls these LETTER, not LIGATURE


def name_key(unicode_name):
	return unicode_name.lower().replace(" ", "-")


def character_named(name):
	try:
		return unicodedata.lookup(name)
	except KeyError:
		return None


def plain_character(rest):
	"""The character a styled name refers to: 'CAPITAL A' → A, 'SMALL ALPHA' → α, 'TWO' → 2"""
	candidates = [rest, "DIGIT " + rest]
	for script in ("LATIN", "GREEK", "CYRILLIC"):
		for case in ("CAPITAL", "SMALL"):
			candidates.append(re.sub(rf"^{case} ({script} )?", f"{script} {case} LETTER ", rest))
			candidates.append(re.sub(rf"^{script} {case} ", f"{script} {case} LETTER ", rest))
	for candidate in candidates:
		character = character_named(candidate)
		if character:
			return character
	return None


def greek_transliteration():
	table = dict(GREEK_LETTERS)
	table.update({latin.upper(): greek.upper() for latin, greek in GREEK_LETTERS.items()})
	for latin, greek in GREEK_DIGRAPHS.items():
		table.update({latin: greek, latin.capitalize(): greek.upper(), latin.upper(): greek.upper()})
	for code in GREEK_NAMED_RANGE:
		match = GREEK_NAMED.match(unicodedata.name(chr(code), ""))
		if match:
			name = match.group(2).lower().replace(" ", "-")
			for spelling in [name] + ([GREEK_SPELLINGS[name]] if name in GREEK_SPELLINGS else []):
				table.setdefault(spelling if match.group(1) == "SMALL" else spelling.capitalize(), chr(code))
	return table


def is_plain(character):
	return len(character) == 1 and unicodedata.category(character)[0] in PLAIN_CATEGORIES


def styled_blocks(named):
	blocks = {name: {} for name, _ in NAME_BLOCKS}
	for block, patterns in NAME_BLOCKS:
		for pattern in patterns:
			compiled = re.compile(pattern + "$")
			for character, name in named:
				match = compiled.match(name)
				if not match:
					continue
				rest = "".join(match.groups()).strip()
				base = plain_character(rest)
				if base and base != character and is_plain(base) and is_plain(character) and base not in blocks[block]:
					blocks[block][base] = character
	for (block, base), name in LETTERLIKE_HOLES.items():
		blocks[block].setdefault(base, unicodedata.lookup(name))
	return blocks


def ligatures(named):
	pattern = re.compile(rf"LATIN (SMALL|CAPITAL) (?:LIGATURE ([A-Z]{{2,3}})|LETTER ({LETTER_LIGATURES}))$")
	table = {}
	for character, name in named:
		match = pattern.match(name)
		if match:
			letters = match.group(2) or match.group(3)
			letters = letters.lower() if match.group(1) == "SMALL" else letters
			table.setdefault(letters, character)
	return table


def colored(named, color):
	word = color.upper()
	table = {}
	for character, name in named:
		words = name.split()
		if ord(character) >= EMOJI_START and word in words:
			rest = [w for w in words if w not in (word, "LARGE")]
			if rest:
				table.setdefault(name_key(" ".join(rest)), character)
	return table


def tag(letter):
	return chr(TAG_BASE + ord(letter))


def unicode_math_names():
	pattern = re.compile(r'\\UnicodeMathSymbol\{"([0-9A-F]+)\}\{\\(\w+)\s*\}')
	names = {}
	if not UNICODE_MATH_TABLE.exists():
		sys.exit(f"missing {UNICODE_MATH_TABLE} (TeX Live unicode-math)")
	for match in pattern.finditer(UNICODE_MATH_TABLE.read_text()):
		names.setdefault(match.group(2), chr(int(match.group(1), 16)))
	return names


def html_names():
	return {name.rstrip(";"): text for name, text in sorted(html.entities.html5.items())}


def seed_sections():
	named = [(chr(cp), unicodedata.name(chr(cp))) for cp in range(0x110000) if unicodedata.name(chr(cp), None)]
	named_plain = [(c, n) for c, n in named if not ALGORITHMIC_NAMES.match(n)]
	blocks = {"greek": greek_transliteration()}
	blocks.update(styled_blocks(named))
	blocks["ligature"] = ligatures(named)
	# an empty suffix: the fonts cannot apply the effect to that script (fonts/README.md), uniscript warns
	for geometry, letter in GEOMETRIES.items():
		blocks[geometry] = {SUFFIX_KEY: tag(letter), f"{SUFFIX_KEY} egyptian": "", f"{SUFFIX_KEY} cjk": ""}
	blocks["mirror"][f"{SUFFIX_KEY} egyptian"] = EGYPTIAN_MIRROR
	del blocks["mirror"][f"{SUFFIX_KEY} cjk"]
	for color, letter in COLORS.items():
		blocks[color] = {SUFFIX_KEY: tag(letter), f"{SUFFIX_KEY} egyptian": "", f"{SUFFIX_KEY} cjk": "", **colored(named, color)}
	for block, suffix in VARIATION_SUFFIXES.items():
		blocks[block] = {SUFFIX_KEY: suffix}
	# groups keep their parts unstyled; a script without prefix or infix cannot be grouped, uniscript warns
	blocks["above"] = {GROUP_KEY: "", f"{PREFIX_KEY} cjk": IDS_ABOVE_TO_BELOW, f"{INFIX_KEY} egyptian": EGYPTIAN_VERTICAL_JOINER}
	blocks["beside"] = {GROUP_KEY: "", f"{PREFIX_KEY} cjk": IDS_LEFT_TO_RIGHT, f"{INFIX_KEY} egyptian": EGYPTIAN_HORIZONTAL_JOINER}
	return {
		"uniscript": dict(UNISCRIPT_NAMES),
		"names": {name_key(n): c for c, n in named_plain},
		"latex": unicode_math_names(),
		"html": html_names(),
		"blocks": blocks,
		"block-aliases": dict(BLOCK_ALIASES),
	}


# ---- the readable file -------------------------------------------------------------------------------------------

BARE_KEY = re.compile(r"^[A-Za-z][A-Za-z0-9-]*$")
HEADER = """// Uniscript entities (wiki/uniscript.md), the human readable source of data/uniscript/entities.idx
// Seeded by data/uniscript/uniscript_index.py from Unicode {unicode} character names, the HTML5 entity list and
// unicode-math-table.tex; edit freely, then run `python3 data/uniscript/uniscript_index.py build`.
//   uniscript      own names, win over all others
//   names          Unicode names, lower case, spaces as hyphens: <:greek-small-letter-alpha> or <:greek small letter alpha>
//   latex          unicode-math command names without backslash: <:alpha> <:infty> <:mfrakA>
//   html           HTML5 entities, backwards compatible but discouraged: <:dopf>
//   blocks         block types: <:fracture A>, <:greek> a b <:/greek>; control keys: "*suffix" follows any other character,
//                  "*suffix egyptian" a hieroglyph, "*prefix cjk" / "*infix egyptian" go before / between the parts of a group
//   block-aliases  other names of block types
// Values are quoted text; invisible and combining characters are written \\u{{hex}}.
"""


def is_visible(character):
	return unicodedata.category(character)[0] not in "MCZ" or character == " "


def quote(text):
	escaped = "".join(c if is_visible(c) and c not in '"\\' else
	                  ("\\" + c if c in '"\\' else f"\\u{{{ord(c):X}}}") for c in text)
	return f'"{escaped}"'


def key_text(key):
	return key if BARE_KEY.match(key) else quote(key)


def write_entities(sections, path):
	lines = [HEADER.format(unicode=unicodedata.unidata_version)]
	for section, entries in sections.items():
		lines.append(f"{section} {{")
		if section == "blocks":
			for block, table in entries.items():
				lines.append(f"\t{block} {{")
				lines.extend(f"\t\t{key_text(k)}: {quote(v)}" for k, v in table.items())
				lines.append("\t}")
		elif section == "block-aliases":
			lines.extend(f"\t{k}: {v}" for k, v in entries.items())
		else:
			lines.extend(f"\t{key_text(k)}: {quote(v)}" for k, v in entries.items())
		lines.append("}\n")
	path.write_text("\n".join(lines))


ENTRY = re.compile(r'^("(?:[^"\\]|\\.)*"|[^\s:"]+):\s*("(?:[^"\\]|\\.)*"|\S+)$')
ESCAPE = re.compile(r'\\u\{([0-9A-Fa-f]+)\}|\\(.)')


def unquote(token):
	if not token.startswith('"'):
		return token
	return ESCAPE.sub(lambda m: chr(int(m.group(1), 16)) if m.group(1) else m.group(2), token[1:-1])


def read_entities(path):
	"""The sections of entities.wasp: one `key: value` per line, `name {` opens a table, `}` closes it"""
	sections, stack = {}, []
	for number, raw in enumerate(path.read_text().splitlines(), 1):
		line = raw.strip()
		if not line or line.startswith("//"):
			continue
		if line.endswith("{"):
			table = {}
			(stack[-1] if stack else sections)[line[:-1].strip()] = table
			stack.append(table)
		elif line == "}":
			stack.pop()
		else:
			match = ENTRY.match(line)
			if not match or not stack:
				sys.exit(f"{path}:{number}: expected `key: value`, got {line!r}")
			stack[-1][unquote(match.group(1))] = unquote(match.group(2))
	return sections


# ---- the binary index --------------------------------------------------------------------------------------------

def text_hash(text):
	value = 0
	for byte in text.encode():
		value = (value * HASH_MULTIPLIER + byte) % HASH_MODULUS
	return value


def block_types(sections):
	blocks = dict(sections["blocks"])
	for alias, block in sections.get("block-aliases", {}).items():
		blocks.setdefault(alias, blocks[block])
	return blocks


def forward_entries(sections):
	"""name → text; a block entry is 'block operand' (or 'block *suffix'…), the block itself 'block ' → """""
	entries = {}
	for section in ENTITY_SECTIONS:
		for name, text in sections.get(section, {}).items():
			entries.setdefault(name, text)
	for block, table in block_types(sections).items():
		entries[block + " "] = ""
		for operand, text in table.items():
			entries[f"{block} {operand}"] = text
	return entries


def reverse_entries(sections):
	"""text → its preferred uniscript: own name, well known short name, block form, Unicode name"""
	latex, html_table = sections.get("latex", {}), sections.get("html", {})
	# well known short names: in HTML and LaTeX alike, or an HTML name that is the last word of the Unicode name (alpha α)
	agreed = {text: name for name, text in latex.items() if html_table.get(name) == text and len(text) == 1}
	for name, text in html_table.items():
		if len(text) == 1 and unicodedata.name(text, "").lower().split(" ")[-1] == name.lower():
			agreed.setdefault(text, name)
	chosen = {}
	for name, text in sections.get("uniscript", {}).items():
		chosen.setdefault(text, f"<:{name}>")
	for text, name in agreed.items():
		chosen.setdefault(text, f"<:{name}>")
	for block, table in sections["blocks"].items():
		for operand, text in table.items():
			if not is_control_key(operand) and len(text) == 1:
				chosen.setdefault(text, f"<:{block} {operand}>")
	for name, text in sections.get("names", {}).items():
		chosen.setdefault(text, f"<:{name}>")
	return {text: form for text, form in chosen.items() if not text.isascii()}


def suffix_entries(sections):
	suffixes = {}
	for block, table in sections["blocks"].items():
		for key, text in table.items():
			if key.split()[0] == SUFFIX_KEY and text:
				suffixes.setdefault(text, block)
	return suffixes


def is_control_key(key):
	return key.startswith("*")


def build_index(sections):
	tables = [forward_entries(sections), reverse_entries(sections), suffix_entries(sections)]
	pool, offsets = bytearray(), {}

	def intern(text):
		data = text.encode()
		if data not in offsets:
			offsets[data] = len(pool)
			pool.extend(data)
		return offsets[data], len(data)

	header_size = len(MAGIC) + 4 + 8 * len(tables)
	records_size = sum(len(t) for t in tables) * RECORD_FIELDS * 4
	pool_start = header_size + records_size
	header, records = bytearray(MAGIC + struct.pack("<I", len(tables))), bytearray()
	for table in tables:
		header += struct.pack("<II", header_size + len(records), len(table))
		for key in sorted(table, key=lambda k: (text_hash(k), k.encode())):
			key_offset, key_length = intern(key)
			value_offset, value_length = intern(table[key])
			records += struct.pack("<5I", text_hash(key), pool_start + key_offset, key_length,
			                       pool_start + value_offset, value_length)
	return bytes(header + records + pool)


class Index:
	"""Reads entities.idx exactly the way lib/uniscript.wasp does: binary search on the hash, then compare keys"""

	def __init__(self, data):
		assert data[:4] == MAGIC, "not a uniscript index"
		self.data = data
		count = struct.unpack_from("<I", data, 4)[0]
		self.tables = [struct.unpack_from("<II", data, 8 + 8 * i) for i in range(count)]

	def record(self, table, position):
		return struct.unpack_from("<5I", self.data, self.tables[table][0] + position * RECORD_FIELDS * 4)

	def lookup(self, table, key):
		wanted, encoded = text_hash(key), key.encode()
		low, high = 0, self.tables[table][1]
		while low < high:
			middle = (low + high) // 2
			if self.record(table, middle)[0] < wanted:
				low = middle + 1
			else:
				high = middle
		while low < self.tables[table][1]:
			hash_value, key_offset, key_length, value_offset, value_length = self.record(table, low)
			if hash_value != wanted:
				return None
			if self.data[key_offset:key_offset + key_length] == encoded:
				return self.data[value_offset:value_offset + value_length].decode()
			low += 1
		return None

	def entries(self, table):
		for position in range(self.tables[table][1]):
			_, key_offset, key_length, value_offset, value_length = self.record(table, position)
			yield (self.data[key_offset:key_offset + key_length].decode(),
			       self.data[value_offset:value_offset + value_length].decode())


def check(sections, data):
	index = Index(data)
	expected = [forward_entries(sections), reverse_entries(sections), suffix_entries(sections)]
	failures = 0
	for table, entries in enumerate(expected):
		stored = dict(index.entries(table))
		if len(stored) != len(entries):
			print(f"{TABLE_NAMES[table]}: {len(stored)} entries in the index, {len(entries)} in the readable file")
			failures += 1
		for key, value in entries.items():
			found = index.lookup(table, key)
			if found != value:
				failures += 1
				if failures < 20:
					print(f"{TABLE_NAMES[table]}: {key!r} → {found!r}, readable file says {value!r}")
	print(f"{'FAILED' if failures else 'OK'}: " +
	      ", ".join(f"{len(e)} {name}" for name, e in zip(TABLE_NAMES, expected)))
	return failures == 0


def main(command):
	if command == "seed":
		write_entities(seed_sections(), ENTITIES_FILE)
		print(f"wrote {ENTITIES_FILE}")
	elif command == "build":
		INDEX_FILE.write_bytes(build_index(read_entities(ENTITIES_FILE)))
		print(f"wrote {INDEX_FILE} ({INDEX_FILE.stat().st_size} bytes)")
	elif command == "check":
		sys.exit(0 if check(read_entities(ENTITIES_FILE), INDEX_FILE.read_bytes()) else 1)
	else:
		sys.exit(__doc__)


if __name__ == "__main__":
	main(sys.argv[1] if len(sys.argv) > 1 else "")
