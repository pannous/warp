#!/usr/bin/env python3
"""Shapes text with the built uniscript fonts through real HarfBuzz (hb-shape) and checks the substitutions."""
import json
import os
import subprocess
import sys

DIST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "fonts", "dist")
TAG = {name: chr(0xE0000 + ord(letter)) for name, letter in
       {"mirror": "M", "flip": "F", "turn": "T", "left": "L", "right": "R", "red": "r", "blue": "b"}.items()}
failures = []


def glyphs(font, text):
    """Glyph names, without the zero-width glyphs HarfBuzz leaves for hidden TAG characters."""
    return [g["g"] for g in shape(font, text) if not (g["ax"] == 0 and (g["g"] == "space" or g["g"].startswith("tag_")))]


def shape(font, text):
    output = subprocess.run(["hb-shape", "--output-format=json", os.path.join(DIST, font), text],
                            capture_output=True, text=True, check=True).stdout
    return json.loads(output)


def check(font, text, predicate, description):
    if not os.path.exists(os.path.join(DIST, font)):
        failures.append("%s missing" % font)
        return
    shaped = glyphs(font, text)
    status = "ok" if predicate(shaped) else "FAIL"
    if status == "FAIL":
        failures.append("%s: %s → %s" % (font, description, shaped))
    print("%-4s %-36s %-30s %s" % (status, font, description, shaped[:6]))


def ends(suffix):
    return lambda shaped: shaped[-1].endswith(suffix)


for font in ["NFM-IndusScriptMirror-Regular.ttf", "JetBrainsMonoMirror-Regular.ttf", "MonacoMirror-Regular.ttf",
             "MenloMirror-Regular.ttf", "MenloMirror-BoldItalic.ttf"]:
    sample = "" if "Indus" in font else "b"
    check(font, sample + TAG["mirror"], ends(".mirror"), "mirror tag")
    check(font, sample + sample, lambda shaped: not any(".mirror" in g for g in shaped), "no tag, no mirror")

sans = "UniscriptSans-Regular.ttf"
check(sans, "e" + TAG["mirror"], ends("e.mirror"), "mirror e")
check(sans, "e" + TAG["turn"], ends("e.turn"), "turn e")
check(sans, "A" + TAG["red"], ends("A.red"), "red A")
check(sans, "A" + TAG["red"] + TAG["mirror"], ends("A.mirror.red"), "red mirror A")
check(sans, "A" + TAG["mirror"] + TAG["red"], ends("A.mirror.red"), "mirror red A")
check(sans, "α" + TAG["blue"], ends(".blue"), "blue alpha")
check(sans, "A" + TAG["left"], ends("A.left"), "rotate A left")
check(sans, "∫" + TAG["mirror"], ends(".mirror"), "mirror integral (math)")
check(sans, "∑" + TAG["red"], ends(".red"), "red sum (math)")
check(sans, "𝔄" + TAG["mirror"], ends(".mirror"), "mirror fraktur A (math alphanumeric)")
check(sans, "AB", lambda shaped: shaped == ["A", "B"], "plain text untouched")
check(sans, "A" + TAG["red"] + "B", lambda shaped: shaped == ["A.red", "B"], "tag colors only the character before it")

cjk = "UniscriptCJK-Regular.otf"
check(cjk, "⿰犭句", lambda shaped: shaped == glyphs(cjk, "狗"), "⿰犭句 → 狗")
check(cjk, "⿱龹豕", lambda shaped: shaped == glyphs(cjk, "豢"), "⿱龹豕 → 豢")
check(cjk, "⿱木⿰木木", lambda shaped: shaped == glyphs(cjk, "森"), "nested ⿱木⿰木木 → 森")
check(cjk, "犭" + TAG["mirror"], lambda shaped: len(shaped) == 1 and shaped != glyphs(cjk, "犭"), "mirror radical")
check(cjk, "的" + TAG["mirror"], lambda shaped: shaped != glyphs(cjk, "的"), "mirror common hanzi")

omni = "NewGardinerOmni2d4.ttf"
check(omni, "𓀀𓑀", lambda shaped: "u13000" not in shaped, "U+13440 mirrors the sign")
check(omni, "𓀀𓐰𓁐", lambda shaped: len(shaped) > 3, "vertical joiner groups")

print("\n%d failures" % len(failures))
for failure in failures:
    print("  " + failure)
sys.exit(1 if failures else 0)
