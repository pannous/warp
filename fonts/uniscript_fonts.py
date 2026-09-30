#!/usr/bin/env python3
"""Build uniscript fonts: invisible TAG characters (U+E0020..E007E) following a character
select a mirrored, rotated or colored variant of it via GSUB, and IDS sequences like ⿰犭句 compose to 狗.

Tags follow their character like variation selectors and emoji tags do: text engines split runs by script and
attach Common characters such as tags to the preceding run, so a prefix would be lost at every script change.

  python3 fonts/uniscript_fonts.py [sans|cjk|egyptian|mirror|all] [--install]
"""
import copy
import os
import shutil
import sys
from collections import defaultdict

from fontTools.colorLib.builder import buildCOLR, buildCPAL
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.otlLib.builder import buildLigatureSubstSubtable
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables import otTables
from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
from fontTools.ttLib.tables._g_l_y_f import Glyph, GlyphComponent, UNSCALED_COMPONENT_OFFSET

HOME = os.path.expanduser("~")
HERE = os.path.dirname(os.path.abspath(__file__))
SOURCES = os.path.join(HERE, "sources")
DIST = os.path.join(HERE, "dist")
USER_FONTS = os.path.join(HOME, "Library/Fonts")
MAX_GLYPHS = 65535
TAG_BASE = 0xE0000  # TAG character for ASCII c is U+E0000 + ord(c)
FEATURE = "ccmp"  # always-on feature, applied first by HarfBuzz and CoreText

GEOMETRY = {  # name: tag letter
    "mirror": "M", "flip": "F", "turn": "T", "left": "L", "right": "R",
}
COLORS = {  # name: (tag letter, RGB)
    "red": ("r", "#E53935"), "green": ("g", "#43A047"), "blue": ("b", "#1E88E5"),
    "brown": ("n", "#795548"), "pink": ("p", "#EC407A"), "purple": ("v", "#8E24AA"),
    "orange": ("o", "#FB8C00"), "yellow": ("y", "#FDD835"), "black": ("k", "#000000"),
    "white": ("w", "#FFFFFF"), "gray": ("a", "#9E9E9E"),
}
IDS_OPERATORS = range(0x2FF0, 0x3000)
IDS_DATA = os.path.join(SOURCES, "cjkvi-ids.txt")
IDS_URL = "https://raw.githubusercontent.com/cjkvi/cjkvi-ids/master/ids.txt"
OMNI_URL = "https://github.com/nederhof/newgardiner/raw/refs/heads/main/fonts/NewGardinerOmni2d4.ttf"
LIGATURES_PER_SUBTABLE = 1500  # keeps each LigatureSet below the 64 KB offset limit

SANS_BASE = os.path.join(USER_FONTS, "NotoSans-Regular.ttf")
SANS_MATH = os.path.join(USER_FONTS, "NotoSansMath-Regular.ttf")
CJK_BASE = os.path.join(USER_FONTS, "NotoSansCJK-Regular.ttf")
MIRROR_FONTS = {  # fonts configured in the user's editors and terminals
    "NFM-Indus Script": os.path.join(USER_FONTS, "NFM-IndusScript_Pure.ttf"),  # Sublime Text
    "JetBrains Mono": os.path.join(USER_FONTS, "JetBrainsMono-Regular.ttf"),  # JetBrains IDEs
    "Monaco": "/System/Library/Fonts/Monaco.ttf",  # iTerm
    "Menlo": "/System/Library/Fonts/Menlo.ttc",  # VS Code, Terminal
}


def tag_char(letter):
    return TAG_BASE + ord(letter)


def tag_glyph(letter):
    return "tag_%04X" % ord(letter)


def is_ascii_or_greek(code):
    return 0x21 <= code < 0x7F or 0x391 <= code <= 0x3C9


def is_symbol_or_latin(code):
    return 0xA0 <= code < 0x250 or 0x370 <= code < 0x400 or 0x2000 <= code < 0x2C00


# ---------------------------------------------------------------- glyph construction

class Outlines:
    """Adds glyphs to a glyf (TrueType) or CFF font behind one interface."""

    def __init__(self, font):
        self.font = font
        self.glyph_set = font.getGlyphSet()
        self.is_cff = "CFF " in font
        self.order = list(font.getGlyphOrder())
        self.new_names = set(self.order)
        self.bounds_cache = {}
        self.top = font["CFF "].cff.topDictIndex[0] if self.is_cff else None
        self.is_cid = self.is_cff and hasattr(self.top, "ROS")
        if self.is_cid:
            used = {int(name[3:]) for name in self.order if name.startswith("cid")}
            self.free_cids = (cid for cid in range(1, MAX_GLYPHS) if cid not in used)

    def advance(self, name):
        return self.font["hmtx"][name][0]

    def bounds(self, name):
        if name not in self.bounds_cache:
            from fontTools.pens.boundsPen import BoundsPen
            pen = BoundsPen(self.glyph_set)
            self.glyph_set[name].draw(pen)
            self.bounds_cache[name] = pen.bounds or (0, 0, 0, 0)
        return self.bounds_cache[name]

    def unique_name(self, wanted):
        if self.is_cid:  # CID-keyed CFF derives each glyph's CID (< 65536) from its name
            wanted = "cid%05d" % next(self.free_cids)
        name, n = wanted, 1
        while name in self.new_names:
            name, n = "%s.%d" % (wanted, n), n + 1
        self.new_names.add(name)
        return name

    def add(self, wanted, source, transform=(1, 0, 0, 1, 0, 0), advance=None):
        """New glyph drawing `source` through the affine transform (xx, xy, yx, yy, dx, dy)."""
        name = self.unique_name(wanted)
        advance = self.advance(source) if advance is None else advance
        if self.is_cff:
            self._add_cff(name, source, transform, advance)
        else:
            self._add_composite(name, source, transform)
        self.font["hmtx"][name] = (advance, int(transformed_x_min(self.bounds(source), transform)))
        if "vmtx" in self.font:
            self.font["vmtx"][name] = self.font["vmtx"][source]
        self.order.append(name)
        return name

    def add_empty(self, wanted):
        name = self.unique_name(wanted)
        if self.is_cff:
            fd_index = self._fd_index(self.order[0])
            self._store_cff(name, self._cff_pen(0, fd_index), fd_index)
        else:
            glyph = Glyph()
            glyph.numberOfContours = 0
            self.font["glyf"][name] = glyph
        self.font["hmtx"][name] = (0, 0)
        if "vmtx" in self.font:
            self.font["vmtx"][name] = (0, 0)
        self.order.append(name)
        return name

    def _add_composite(self, name, source, transform):
        xx, xy, yx, yy, dx, dy = transform
        component = GlyphComponent()
        component.glyphName = source
        component.x, component.y = int(round(dx)), int(round(dy))
        component.flags = UNSCALED_COMPONENT_OFFSET
        if (xx, xy, yx, yy) != (1, 0, 0, 1):
            component.transform = [[xx, xy], [yx, yy]]
        glyph = Glyph()
        glyph.numberOfContours = -1
        glyph.components = [component]
        self.font["glyf"][name] = glyph

    def _fd_index(self, source):
        return self.top.CharStrings.getItemAndSelector(source)[1] if self.is_cid else None

    def _private(self, fd_index):
        return self.top.FDArray[fd_index].Private if self.is_cid else self.top.Private

    def _cff_pen(self, advance, fd_index):
        """Charstring widths are stored relative to the private dict's nominalWidthX."""
        return T2CharStringPen(advance - getattr(self._private(fd_index), "nominalWidthX", 0), self.glyph_set)

    def _add_cff(self, name, source, transform, advance):
        fd_index = self._fd_index(source)
        pen = self._cff_pen(advance, fd_index)
        self.glyph_set[source].draw(TransformPen(pen, transform))
        self._store_cff(name, pen, fd_index)

    def _store_cff(self, name, pen, fd_index):
        charstring = pen.getCharString(private=self._private(fd_index), globalSubrs=self.top.GlobalSubrs)
        charstrings = self.top.CharStrings
        charstrings.charStringsIndex.append(charstring)
        charstrings.charStrings[name] = len(charstrings.charStringsIndex) - 1
        if self.is_cid:
            charstring.fdSelectIndex = fd_index
            self.top.FDSelect.append(fd_index)
        self.top.charset.append(name)

    def commit(self):
        for table in ("hdmx", "LTSH", "VDMX"):  # per-glyph device metrics would lack the new glyphs
            if table in self.font:
                del self.font[table]
        self.font.setGlyphOrder(self.order)
        if not self.is_cff:
            self.font["glyf"].glyphOrder = self.order
        if self.is_cid:
            self.top.numGlyphs = len(self.order)
        self.font["maxp"].numGlyphs = len(self.order)
        if not self.is_cff:  # format 2 stores the glyph names, which make shaping output readable
            post = self.font["post"]
            post.formatType, post.extraNames, post.mapping = 2.0, [], {}


def transformed_x_min(bounds, transform):
    x_min, y_min, x_max, y_max = bounds
    xx, xy, yx, yy, dx, dy = transform
    return min(xx * x + yx * y + dx for x in (x_min, x_max) for y in (y_min, y_max))


def geometry_transform(outlines, name, geometry):
    """(transform, advance) placing the turned glyph inside its own box, rotations centered vertically."""
    x_min, y_min, x_max, y_max = outlines.bounds(name)
    advance = outlines.advance(name)
    center_y = (y_min + y_max) / 2
    side = max(0, x_min)
    if geometry == "mirror":
        return (-1, 0, 0, 1, advance, 0), advance
    if geometry == "flip":
        return (1, 0, 0, -1, 0, y_min + y_max), advance
    if geometry == "turn":
        return (-1, 0, 0, -1, advance, y_min + y_max), advance
    rotated_advance = int(y_max - y_min + 2 * side)
    if geometry == "left":  # (x, y) -> (-y, x)
        return (0, 1, -1, 0, y_max + side, center_y - (x_min + x_max) / 2), rotated_advance
    if geometry == "right":  # (x, y) -> (y, -x)
        return (0, -1, 1, 0, side - y_min, center_y + (x_min + x_max) / 2), rotated_advance
    raise ValueError(geometry)


# ---------------------------------------------------------------- cmap / GSUB plumbing

def map_characters(font, mapping):
    """Add code point -> glyph to every Unicode cmap, creating a format 12 table for astral code points."""
    cmap = font["cmap"]
    if not any(t.format == 12 for t in cmap.tables):
        full = CmapSubtable.newSubtable(12)
        full.platformID, full.platEncID, full.language = 3, 10, 0
        full.cmap = dict(font.getBestCmap())
        cmap.tables.append(full)
    for table in cmap.tables:
        if table.isUnicode() and (table.format == 12 or table.format == 4):
            for code, glyph in mapping.items():
                if table.format == 12 or code <= 0xFFFF:
                    table.cmap[code] = glyph


def ensure_gsub(font):
    if "GSUB" not in font:
        gsub = newTable("GSUB")
        gsub.table = otTables.GSUB()
        gsub.table.Version = 0x00010000
        gsub.table.ScriptList = otTables.ScriptList()
        gsub.table.ScriptList.ScriptRecord = []
        gsub.table.FeatureList = otTables.FeatureList()
        gsub.table.FeatureList.FeatureRecord = []
        gsub.table.LookupList = otTables.LookupList()
        gsub.table.LookupList.Lookup = []
        font["GSUB"] = gsub
    return font["GSUB"].table


def new_lang_sys():
    lang_sys = otTables.LangSys()
    lang_sys.LookupOrder, lang_sys.ReqFeatureIndex, lang_sys.FeatureIndex = None, 0xFFFF, []
    return lang_sys


def all_lang_systems(gsub):
    """Every LangSys, adding a DFLT script (the fallback for any script) when missing.
    Other scripts are never added: a new script record would hide the font's own DFLT features."""
    if not any(record.ScriptTag == "DFLT" for record in gsub.ScriptList.ScriptRecord):
        record = otTables.ScriptRecord()
        record.ScriptTag, record.Script = "DFLT", otTables.Script()
        record.Script.DefaultLangSys, record.Script.LangSysRecord = new_lang_sys(), []
        gsub.ScriptList.ScriptRecord.insert(0, record)
    for record in gsub.ScriptList.ScriptRecord:
        if record.Script.DefaultLangSys is None:
            record.Script.DefaultLangSys = new_lang_sys()
        yield record.Script.DefaultLangSys
        for lang in record.Script.LangSysRecord:
            yield lang.LangSys


def shift_nested_lookups(lookup, offset):
    for subtable in lookup.SubTable:
        inner = subtable.ExtSubTable if lookup.LookupType == 7 else subtable
        records = list(getattr(inner, "SubstLookupRecord", None) or [])
        for rule_set in (getattr(inner, "ChainSubRuleSet", None) or []) + (getattr(inner, "ChainSubClassSet", None) or []):
            for rule in (getattr(rule_set, "ChainSubRule", None) or getattr(rule_set, "ChainSubClassRule", None) or []):
                records += rule.SubstLookupRecord
        for record in records:
            record.LookupListIndex += offset


def append_lookups(font, lookups, active):
    """Append lookups (nested indices relative to `lookups`) and run the `active` ones in FEATURE.
    HarfBuzz uses only the first feature of a tag per LangSys, so existing FEATURE records are extended."""
    gsub = ensure_gsub(font)
    offset = len(gsub.LookupList.Lookup)
    for lookup in lookups:
        shift_nested_lookups(lookup, offset)
    gsub.LookupList.Lookup.extend(lookups)
    gsub.LookupList.LookupCount = len(gsub.LookupList.Lookup)
    indices = [offset + i for i in active]
    records = gsub.FeatureList.FeatureRecord
    shared_new_feature, extended = None, set()
    for lang_sys in all_lang_systems(gsub):
        existing = [i for i in lang_sys.FeatureIndex if records[i].FeatureTag == FEATURE]
        if not existing:
            if shared_new_feature is None:
                feature = otTables.FeatureRecord()
                feature.FeatureTag, feature.Feature = FEATURE, otTables.Feature()
                feature.Feature.FeatureParams, feature.Feature.LookupListIndex = None, []
                records.append(feature)
                shared_new_feature = len(records) - 1
            lang_sys.FeatureIndex.append(shared_new_feature)
            existing = [shared_new_feature]
        for i in set(existing) - extended:
            records[i].Feature.LookupListIndex += indices
            extended.add(i)
    gsub.FeatureList.FeatureCount = len(records)


def lookups_from_features(font, fea):
    """Compile feature code against the font's glyph order without touching its existing tables."""
    scratch = TTFont()
    scratch.setGlyphOrder(font.getGlyphOrder())
    addOpenTypeFeaturesFromString(scratch, fea, tables=["GSUB"])
    gsub = scratch["GSUB"].table
    return gsub.LookupList.Lookup, gsub.FeatureList.FeatureRecord[0].Feature.LookupListIndex


def glyph_class(names):
    return "[" + " ".join("\\" + name for name in names) + "]"


# ---------------------------------------------------------------- effects

def add_effects(font, tiers):
    """tiers: [(code point predicate, geometries, colors, combine)] — first matching tier wins.
    Returns {effect: {source glyph: variant glyph}}."""
    outlines = Outlines(font)
    cmap = font.getBestCmap()
    variants = defaultdict(dict)
    color_layers = {}
    palette = list(COLORS)
    seen = set()
    for code, base in sorted(cmap.items()):
        if base in seen or base == ".notdef":
            continue
        tier = next((tier for tier in tiers if tier[0](code)), None)
        if not tier:
            continue
        seen.add(base)
        _, geometries, colors, combine = tier
        shapes = [base]
        for geometry in geometries:
            transform, advance = geometry_transform(outlines, base, geometry)
            variant = outlines.add("%s.%s" % (base, geometry), base, transform, advance)
            variants[geometry][base] = variant
            if combine:
                shapes.append(variant)
        for color in colors:
            for shape in shapes:
                variant = outlines.add("%s.%s" % (shape, color), shape)
                variants[color][shape] = variant
                color_layers[variant] = [(shape, palette.index(color))]
    tag_glyphs = {letter: outlines.add_empty(tag_glyph(letter)) for letter in all_tag_letters()}
    map_characters(font, {tag_char(letter): glyph for letter, glyph in tag_glyphs.items()})
    outlines.commit()
    assert len(outlines.order) <= MAX_GLYPHS, "%d glyphs exceed the OpenType limit" % len(outlines.order)
    if color_layers:
        font["COLR"] = buildCOLR(color_layers, glyphMap=font.getReverseGlyphMap())
        font["CPAL"] = buildCPAL([[hex_to_rgba(COLORS[c][1]) for c in palette]])
    return variants, tag_glyphs


def hex_to_rgba(hex_color):
    return tuple(int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)) + (1.0,)


def all_tag_letters():
    return list(GEOMETRY.values()) + [letter for letter, _ in COLORS.values()]


def effect_rules(effect_letter, variants, other_letters, tag_glyphs):
    """A tag right after the glyph, or separated from it by one tag of the other kind."""
    if not variants:
        return ""
    tag = "\\" + tag_glyphs[effect_letter]
    others = glyph_class(tag_glyphs[letter] for letter in other_letters)
    sources, targets = glyph_class(variants), glyph_class(variants.values())
    rules = "  sub %s' %s by %s;\n" % (sources, tag, targets)
    if other_letters:
        rules += "  sub %s' %s %s by %s;\n" % (sources, others, tag, targets)
    return rules


def add_prefix_rules(font, effects):
    """Geometry first, then color, so `A red mirror` and `A mirror red` both give A.mirror.red."""
    variants, tag_glyphs = effects
    geometry_letters, color_letters = list(GEOMETRY.values()), [c[0] for c in COLORS.values()]
    geometry = "".join(effect_rules(GEOMETRY[g], variants.get(g), color_letters, tag_glyphs) for g in GEOMETRY)
    color = "".join(effect_rules(COLORS[c][0], variants.get(c), geometry_letters, tag_glyphs) for c in COLORS)
    fea = "languagesystem DFLT dflt;\nfeature %s {\n lookup geometry {\n%s } geometry;\n lookup color {\n%s } color;\n} %s;\n"
    append_lookups(font, *lookups_from_features(font, fea % (FEATURE, geometry, color, FEATURE)))


# ---------------------------------------------------------------- IDS composition

def ids_ligatures(font):
    """{(operator, part, part…) glyphs: composed glyph} for flat IDS whose parts are all in the font."""
    if not os.path.exists(IDS_DATA):
        download(IDS_URL, IDS_DATA)
    cmap = font.getBestCmap()
    ligatures = {}
    for line in open(IDS_DATA, encoding="utf-8"):
        if line.startswith("#") or "\t" not in line:
            continue
        _, character, *sequences = line.rstrip("\n").split("\t")
        if len(character) != 1 or ord(character) not in cmap:
            continue
        for sequence in sequences:
            sequence = sequence.split("[")[0]  # drop region annotations like [GTJ]
            if len(sequence) < 3 or ord(sequence[0]) not in IDS_OPERATORS:
                continue
            if all(ord(c) in cmap for c in sequence):
                key = tuple(cmap[ord(c)] for c in sequence)
                ligatures.setdefault(key, cmap[ord(character)])
    return ligatures


def ligature_lookup(ligatures):
    lookup = otTables.Lookup()
    lookup.LookupType, lookup.LookupFlag = 4, 0
    chunks = defaultdict(dict)
    for index, (key, glyph) in enumerate(sorted(ligatures.items(), key=lambda item: (-len(item[0]), item[0]))):
        chunks[index // LIGATURES_PER_SUBTABLE][key] = glyph
    lookup.SubTable = [buildLigatureSubstSubtable(chunk) for chunk in chunks.values()]
    lookup.SubTableCount = len(lookup.SubTable)
    return lookup


def add_ids_composition(font):
    """Two identical passes so nested IDS (⿱⿰AB C) compose inside out."""
    ligatures = ids_ligatures(font)
    append_lookups(font, [ligature_lookup(ligatures), ligature_lookup(ligatures)], [0, 1])
    return len(ligatures)


# ---------------------------------------------------------------- naming / io

def rename(font, family):
    postscript = family.replace(" ", "")
    style = font["name"].getDebugName(2) or "Regular"
    for record in font["name"].names:
        if record.nameID in (1, 16, 21):
            record.string = family
        elif record.nameID == 4:
            record.string = "%s %s" % (family, style) if style != "Regular" else family
        elif record.nameID in (6, 20):
            record.string = "%s-%s" % (postscript, style.replace(" ", ""))
        elif record.nameID == 3:
            record.string = "%s-%s;uniscript" % (postscript, style.replace(" ", ""))
    for table in ("morx", "mort", "feat"):  # CoreText prefers AAT over GSUB
        if table in font:
            del font[table]


def strip_hinting(font):
    for table in ("fpgm", "prep", "cvt ", "hdmx", "LTSH", "VDMX"):
        if table in font:
            del font[table]
    if "glyf" in font:
        for name in font.getGlyphOrder():
            glyph = font["glyf"][name]
            if hasattr(glyph, "program"):
                glyph.program.fromBytecode(b"")
    font["maxp"].maxSizeOfInstructions = 0


def download(url, path):
    import urllib.request
    os.makedirs(os.path.dirname(path), exist_ok=True)
    urllib.request.urlretrieve(url, path)


def save(font, filename):
    os.makedirs(DIST, exist_ok=True)
    path = os.path.join(DIST, filename)
    font.save(path)
    print("wrote %s (%d glyphs)" % (path, len(font.getGlyphOrder())))
    return path


# ---------------------------------------------------------------- math merge

def copy_missing_glyphs(target, donor_path):
    """Copy donor glyphs for code points the target lacks, with their components, stripped of hints."""
    donor = TTFont(donor_path)
    target_cmap, donor_cmap = target.getBestCmap(), donor.getBestCmap()
    names = set(target.getGlyphOrder())
    renamed = {}

    def import_glyph(name):
        if name in renamed:
            return renamed[name]
        new_name = name if name not in names else "math." + name
        renamed[name] = new_name
        names.add(new_name)
        glyph = copy.deepcopy(donor["glyf"][name])
        if glyph.isComposite():
            for component in glyph.components:
                component.glyphName = import_glyph(component.glyphName)
        elif hasattr(glyph, "program"):
            glyph.program.fromBytecode(b"")
        target["glyf"][new_name] = glyph
        target["hmtx"][new_name] = donor["hmtx"][name]
        return new_name

    added = {code: import_glyph(name) for code, name in donor_cmap.items() if code not in target_cmap}
    target.setGlyphOrder(target.getGlyphOrder() + [n for n in renamed.values() if n not in target.getGlyphOrder()])
    map_characters(target, added)
    return len(added)


# ---------------------------------------------------------------- builds

def build_sans():
    font = TTFont(SANS_BASE)
    strip_hinting(font)
    copied = copy_missing_glyphs(font, SANS_MATH)
    font["glyf"].glyphOrder = font.getGlyphOrder()
    everything = list(GEOMETRY)
    tiers = [
        (is_ascii_or_greek, everything, list(COLORS), True),
        (is_symbol_or_latin, everything, list(COLORS), False),
        (lambda code: True, ["mirror", "turn"], [], False),
    ]
    add_prefix_rules(font, add_effects(font, tiers))
    rename(font, "Uniscript Sans")
    print("copied %d math characters" % copied)
    return [save(font, "UniscriptSans-Regular.ttf")]


def is_mirrored_cjk(code):
    return 0x2E80 <= code < 0x2FE0 or 0x31C0 <= code < 0x31F0 or is_common_hanzi(code)


def is_common_hanzi(code):
    """GB 2312 level 1: the 3755 most frequent simplified characters (rows 16-55)."""
    if not 0x4E00 <= code < 0xA000:
        return False
    try:
        encoded = chr(code).encode("gb2312")
    except UnicodeEncodeError:
        return False
    return 0xB0 <= encoded[0] <= 0xD7


def build_cjk():
    font = TTFont(CJK_BASE)
    variants = add_effects(font, [(is_mirrored_cjk, ["mirror"], [], False)])
    add_prefix_rules(font, variants)
    compositions = add_ids_composition(font)
    rename(font, "Uniscript CJK")
    print("%d IDS compositions" % compositions)
    return [save(font, "UniscriptCJK-Regular.otf")]


def build_egyptian():
    """NewGardinerOmni implements the Unicode 15 format controls (joiners, insertions, U+13440 mirror)."""
    path = os.path.join(SOURCES, os.path.basename(OMNI_URL))
    if not os.path.exists(path):
        download(OMNI_URL, path)
    os.makedirs(DIST, exist_ok=True)
    target = os.path.join(DIST, os.path.basename(OMNI_URL))
    shutil.copy(path, target)
    print("copied %s" % target)
    return [target]


def build_mirror():
    paths = []
    for family, path in MIRROR_FONTS.items():
        count = 4 if path.endswith(".ttc") else 1
        for number in range(count):
            font = TTFont(path, fontNumber=number) if count > 1 else TTFont(path)
            style = (font["name"].getDebugName(2) or "Regular").replace(" ", "")
            add_prefix_rules(font, add_effects(font, [(lambda code: code > 0x20, ["mirror"], [], False)]))
            rename(font, family + " Mirror")
            paths.append(save(font, "%sMirror-%s.ttf" % (family.replace(" ", ""), style)))
    return paths


BUILDS = {"sans": build_sans, "cjk": build_cjk, "egyptian": build_egyptian, "mirror": build_mirror}


def main(args):
    names = [a for a in args if not a.startswith("--")] or ["all"]
    paths = []
    for name in (list(BUILDS) if names == ["all"] else names):
        paths += BUILDS[name]()
    if "--install" in args:
        for path in paths:
            shutil.copy(path, USER_FONTS)
            print("installed", os.path.basename(path))


if __name__ == "__main__":
    main(sys.argv[1:])
