#!/usr/bin/env python3
"""warp_language.node (card pypi-warp-release): warp data text becomes Python values and back, and the wheel ships it.
Run it with python3 -I test/tools/test_warp_node.py"""

import sys
import tempfile
import zipfile
from pathlib import Path

PYPI = Path(__file__).resolve().parents[2] / "packaging" / "pypi"
sys.path.insert(0, str(PYPI))
from warp_language.node import Symbol, Tagged, WarpSyntaxError, parse, serialize  # noqa: E402

failures = 0


def check(what: str, got, expected) -> None:
    global failures
    if got == expected:
        print(f"✓ {what}")
    else:
        failures += 1
        print(f"✗ {what}: got {got!r}, expected {expected!r}")


check("a tagged map reads as Python values", parse('Person{name: "Alice", age: 30, ok: yes, gone: ø}'),
      Tagged("Person", {"name": "Alice", "age": 30, "ok": True, "gone": None}))
check("bare words are symbols", parse("[red green]"), [Symbol("red"), Symbol("green")])
check("the three brackets are one list", parse("{1 2 3}") == parse("[1 2 3]") == parse("(1, 2, 3)"), True)
check("= and : both bind a key", parse("{a=1 b:2.5}"), {"a": 1, "b": 2.5})
check("comments are skipped", parse("[1 // one\n 2 /* two */ 3]"), [1, 2, 3])
check("text escapes", parse('"a\\nb\\x24{c}"'), "a\nb${c}")
check("single quotes are literal", parse("'a${b}\\n'"), "a${b}\\n")
check("numbers", parse("[0x1F 1_000 1e3 -4]"), [31, 1000, 1000.0, -4])
for value in [{"a": [1, {"b": None}], "c d": "q\"uote\n${x}"}, Tagged("p", [1, 2], "("), [True, False, 1.5, "ø"], []]:
    check(f"round trip {value!r}", parse(serialize(value)), value)
check("a map prints as warp", serialize({"name": "Alice", "tags": [Symbol("a")]}), '{name: "Alice", tags: [a]}')
for bad in ["(1 2", "{a: 1 2}", '"open', ")"]:
    try:
        parse(bad)
        check(f"{bad!r} is refused", "accepted", "refused")
    except WarpSyntaxError:
        check(f"{bad!r} is refused", "refused", "refused")

sys.path.insert(0, str(PYPI))
import build_wheels  # noqa: E402

with tempfile.TemporaryDirectory() as folder:
    wheel = build_wheels.write_wheel("0.0.0", "aarch64-apple-darwin", {"warp": (b"#!", 0o755)}, "readme", Path(folder))
    names = zipfile.ZipFile(wheel).namelist()
check("the wheel carries the Python package", [n for n in names if n.startswith("warp_language/")],
      ["warp_language/__init__.py", "warp_language/node.py"])
sys.exit(1 if failures else 0)
