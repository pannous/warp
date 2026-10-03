#!/usr/bin/env python3
"""Slices the large web fonts into unicode-range subsets (like Google Fonts), so a page downloads only the slices of the
characters it shows. Writes fonts.css (one @font-face per slice) and fonts.json for sequence_fonts.js.

A browser shapes each unicode-range face on its own and maps only the characters its range declares, so sequences a
font shapes together cannot be split over slices. They get faces outside fonts.css, which sequence_fonts.js assigns to
the sequence as one unranged FontFace:
- IDS composition `⿰犭句`: faces of an operator's ligatures sorted by operands, found by the operands.
- hieroglyph groups and mirroring (format controls 13430–1345F): one face with every sign.
- TAG effects `你<TAG M>`: the base character's plain slice, whose cmap carries the TAG controls undeclared.
Plain characters are ordered by frequency (wordfreq zh/ja/ko): the first slices hold the common characters,
the rest follows in code point order, which keeps the unicode-range lists short.

Usage: slice_fonts.py SOURCE_DIRS OUT_DIR   (SOURCE_DIRS: colon-separated directories searched for the font files)
"""
import json
import os
import sys
from concurrent.futures import ProcessPoolExecutor

from fontTools import subset
from fontTools.ttLib import TTFont

FREQUENT_CHARACTERS = 4000  # sorted by frequency into FREQUENT_SLICE_SIZE slices, the rest by code point
FREQUENT_SLICE_SIZE = 250
TAIL_SLICE_SIZE = 500
LIGATURES_PER_FACE = 50
FREQUENCY_LANGUAGES = ("zh", "ja", "ko")
TAG_CONTROLS = range(0xE0000, 0xE0080)
PRIVATE_USE = (range(0xE000, 0xF900), range(0xF0000, 0x110000))
LIGATURES, RUN = "ligatures", "run"  # structure faces per operator's ligatures, or one for every run containing a control

FONTS = [
	dict(family="Uniscript Sans", file="UniscriptSans-Regular.ttf", directory=None),  # served whole
	dict(family="Uniscript CJK", file="UniscriptCJK-Regular.otf", directory="cjk",
		structure=range(0x2FF0, 0x3000), sequences=LIGATURES, carried=TAG_CONTROLS),
	dict(family="NewGardinerOmni", file="NewGardinerOmni2d4.ttf", directory="gardiner",
		structure=range(0x13430, 0x13460), sequences=RUN, carried=()),
]
SUBSET_OPTIONS = dict(layout_features=["*"], name_IDs=["*"], name_languages=["*"], notdef_outline=True,
	hinting=False, flavor="woff2")


def character_frequencies():
	import wordfreq
	frequency = {}
	for language in FREQUENCY_LANGUAGES:
		for word, weight in wordfreq.get_frequency_dict(language).items():
			for character in word:
				frequency[ord(character)] = frequency.get(ord(character), 0) + weight
	return frequency


def chunks(items, size):
	return [items[start:start + size] for start in range(0, len(items), size)]


def plain_slices(code_points, frequency):
	by_frequency = sorted((point for point in code_points if point in frequency), key=lambda point: -frequency[point])
	frequent = by_frequency[:FREQUENT_CHARACTERS]
	rest = sorted(set(code_points) - set(frequent))
	return chunks(frequent, FREQUENT_SLICE_SIZE) + chunks(rest, TAIL_SLICE_SIZE)


def ranges(code_points):
	"""sorted [first, last] pairs of consecutive code points"""
	result = []
	for point in sorted(code_points):
		if result and result[-1][1] == point - 1:
			result[-1][1] = point
		else:
			result.append([point, point])
	return result


def unicode_range(code_points):
	return ", ".join(f"U+{first:X}" if first == last else f"U+{first:X}-{last:X}" for first, last in ranges(code_points))


def is_private(point):
	return any(point in block for block in PRIVATE_USE)


def ligatures_by_first_glyph(font_file):
	"""{first glyph: [component glyph lists]} of all GSUB ligatures"""
	ligatures = {}
	gsub = font_file["GSUB"].table if "GSUB" in font_file else None
	for lookup in gsub.LookupList.Lookup if gsub else []:
		for table in lookup.SubTable:
			table = getattr(table, "ExtSubTable", table)
			for first, entries in getattr(table, "ligatures", {}).items():
				ligatures.setdefault(first, []).extend(entry.Component for entry in entries)
	return ligatures


def ligature_faces(operator, operand_lists):
	"""an operator's ligatures, sorted by operands, in faces of LIGATURES_PER_FACE: [(first operands, code points)].
	Few ligatures per face keep it small: a face holds every ligature its components can form."""
	return [(operands[0], {operator}.union(*operands)) for operands in chunks(sorted(operand_lists), LIGATURES_PER_FACE)]


def plan(font, source, frequency):
	"""(plain slices [(name, code points)], structure faces [(name, code points)], sequences entries of fonts.json)"""
	font_file = TTFont(source, lazy=True)
	cmap = font_file.getBestCmap()
	available = {point for point in cmap if not is_private(point)}
	structure = available & set(font["structure"])
	plain = sorted(available - set(font["carried"]) - structure)
	directory = font["directory"]
	slices = [(f"{index}.woff2", set(declared)) for index, declared in enumerate(plain_slices(plain, frequency))]
	if font["sequences"] == RUN:
		return slices, [("s.woff2", available)], [dict(trigger=ranges(structure), run=ranges(available), url=f"{directory}/s.woff2")]
	points_of_glyph = {}
	for point in sorted(available):
		points_of_glyph.setdefault(cmap[point], []).append(point)
	ligatures = ligatures_by_first_glyph(font_file)
	faces, sequences, aliases = [], [], {}
	for operator in sorted(structure):
		glyph_lists = [components for components in ligatures.get(cmap[operator], []) if all(glyph in points_of_glyph for glyph in components)]
		if not glyph_lists:
			continue
		for glyph in {glyph for components in glyph_lists for glyph in components}:
			aliases.update((alias, points_of_glyph[glyph][0]) for alias in points_of_glyph[glyph][1:])
		# operands by their first code point; a face holds every code point of its component glyphs
		operand_lists = [[points_of_glyph[glyph][0] for glyph in components] for components in glyph_lists]
		buckets = []
		for index, (first_operands, points) in enumerate(ligature_faces(operator, operand_lists)):
			name = f"s{operator:X}-{index}.woff2"
			faces.append((name, {alias for point in points for alias in points_of_glyph[cmap[point]]}))
			buckets.append([first_operands, f"{directory}/{name}"])
		# nested IDS make ligatures longer: the shortest is the operator's own arity
		sequences.append(dict(trigger=[[operator, operator]], arity=min(map(len, operand_lists)), buckets=buckets))
	if aliases:
		sequences.append(dict(aliases=sorted(aliases.items())))
	return slices, faces, sequences


def write_slice(source, target, code_points):
	font = TTFont(source)
	subsetter = subset.Subsetter(subset.Options(**SUBSET_OPTIONS))
	subsetter.populate(unicodes=code_points)
	subsetter.subset(font)
	font.flavor = "woff2"
	font.save(target)
	return target, os.path.getsize(target)


def face(family, url, declared=None):
	unicode_range_line = f"\n\tunicode-range: {unicode_range(declared)};" if declared else ""
	return f'@font-face {{\n\tfont-family: "{family}"; src: url("{url}") format("woff2"); font-display: swap;{unicode_range_line}\n}}\n'


def find(file, directories):
	for directory in directories:
		path = os.path.join(os.path.expanduser(directory), file)
		if os.path.isfile(path):
			return path
	sys.exit(f"missing font {file}: run python3 fonts/uniscript_fonts.py all in the uniscript repository (github.com/pannous/uniscript)")


def main(source_directories, out):
	frequency = character_frequencies()
	css, jobs, sequences, tagged = ["/* generated by web/uniscript/slice_fonts.py */\n"], [], [], []
	for font in FONTS:
		source = find(font["file"], source_directories.split(":"))
		if font["directory"] is None:
			name = os.path.splitext(font["file"])[0] + ".woff2"
			jobs.append((source, os.path.join(out, name), sorted(TTFont(source, lazy=True).getBestCmap())))
			css.append(face(font["family"], name))
			continue
		os.makedirs(os.path.join(out, font["directory"]), exist_ok=True)
		slices, structure_faces, font_sequences = plan(font, source, frequency)
		carried = set(font["carried"])
		for name, declared in slices:
			url = f'{font["directory"]}/{name}'
			jobs.append((source, os.path.join(out, url), sorted(declared | carried)))
			css.append(face(font["family"], url, declared))
		jobs += [(source, os.path.join(out, font["directory"], name), sorted(points | carried)) for name, points in structure_faces]
		sequences += font_sequences
		tagged += [font["family"]] if carried else []
	newest_input = lambda source: max(os.path.getmtime(source), os.path.getmtime(__file__))
	stale = [job for job in jobs if not os.path.exists(job[1]) or os.path.getmtime(job[1]) < newest_input(job[0])]
	with ProcessPoolExecutor() as pool:
		for target, size in pool.map(write_slice, *zip(*stale)) if stale else []:
			print(f"{size // 1024:>6} KB  {os.path.relpath(target, out)}")
	with open(os.path.join(out, "fonts.css"), "w") as css_file:
		css_file.write("".join(css))
	with open(os.path.join(out, "fonts.json"), "w") as json_file:
		json.dump(dict(sequences=sequences, tag=ranges(TAG_CONTROLS), tagged=tagged), json_file, separators=(",", ":"))
	print(f"{len(jobs)} font files, {len(stale)} rebuilt, fonts.css, fonts.json")


if __name__ == "__main__":
	main(*sys.argv[1:])
