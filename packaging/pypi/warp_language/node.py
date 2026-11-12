"""Reads and writes warp data as Python values, no warp binary needed (`import warp_language.node as warp`).

    warp.parse('Person{name: "Alice", age: 30, hobbies: [reading hiking]}')
    # Tagged('Person', {'name': 'Alice', 'age': 30, 'hobbies': [Symbol('reading'), Symbol('hiking')]})
    warp.serialize(value)  # warp text that parses back to an equal value

A `{}` `[]` or `()` of `key: value` (or `key=value`) entries is a dict, otherwise a list; a bare word is a Symbol (a str);
`Name{…}` is Tagged; yes/no are True/False, ø is None; `//` and `/* */` comments are skipped.
"""

import math
import re
from dataclasses import dataclass
from typing import Any

BRACKETS = {"{": "}", "[": "]", "(": ")"}
KEY_OPERATORS = (":", "=")
SEPARATORS = ",;\n"
NUMBER = re.compile(r"[+-]?(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?\d+)?)")
SYMBOL = re.compile(r"[^\W\d][\w·]*", re.UNICODE)
WORDS = {"yes": True, "true": True, "no": False, "false": False, "ø": None, "null": None}
ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", '"': '"', "'": "'", "\\": "\\", "$": "$"}
WRITTEN_ESCAPES = {"\\": "\\\\", '"': '\\"', "\n": "\\n", "\t": "\\t", "\r": "\\r"}
INTERPOLATION_START = "${"
DOLLAR_ESCAPE = "\\x24"
OPEN_FOR_CLOSE = {close: open_ for open_, close in BRACKETS.items()}


class Symbol(str):
    """A bare word: data, not a text."""

    def __repr__(self) -> str:
        return f"Symbol({str.__repr__(self)})"


@dataclass
class Tagged:
    """`name{…}`: a body (dict or list) with the word written before it."""

    name: str
    body: Any
    bracket: str = "{"


class WarpSyntaxError(ValueError):
    pass


class _Reader:
    def __init__(self, text: str):
        self.text = text
        self.at = 0

    def fail(self, reason: str) -> WarpSyntaxError:
        line = self.text.count("\n", 0, self.at) + 1
        column = self.at - (self.text.rfind("\n", 0, self.at) + 1) + 1
        return WarpSyntaxError(f"{reason} at {line}:{column}")

    def skip(self, newlines: bool = True) -> None:
        while self.at < len(self.text):
            here = self.text[self.at]
            if here in " \t\r" or (newlines and here == "\n"):
                self.at += 1
            elif self.text.startswith("//", self.at):
                end = self.text.find("\n", self.at)
                self.at = len(self.text) if end < 0 else end
            elif self.text.startswith("/*", self.at):
                end = self.text.find("*/", self.at)
                if end < 0:
                    raise self.fail("unclosed /* comment")
                self.at = end + 2
            else:
                return

    def peek(self) -> str:
        return self.text[self.at] if self.at < len(self.text) else ""

    def document(self) -> Any:
        items = self.entries("")
        return items[0] if len(items) == 1 and not isinstance(items[0], _Entry) else _as_collection(self, items)

    def entries(self, closing: str) -> list:
        items: list = []
        while True:
            self.skip()
            if self.peek() == closing:
                return items
            if self.peek() == "":
                raise self.fail(f"missing {closing}")
            if self.peek() in OPEN_FOR_CLOSE:
                raise self.fail(f"unexpected {self.peek()}")
            if self.peek() in SEPARATORS:
                self.at += 1
                continue
            items.append(self.entry())

    def entry(self) -> Any:
        value = self.value()
        self.skip(newlines=False)
        if self.peek() in KEY_OPERATORS:
            self.at += 1
            self.skip()
            key = value
            if not isinstance(key, str):
                raise self.fail("a key is a word or a text")
            return _Entry(str(key), self.value())
        return value

    def value(self) -> Any:
        self.skip()
        here = self.peek()
        if here in BRACKETS:
            return self.collection()
        if here in "\"'":
            return self.quoted(here)
        number = NUMBER.match(self.text, self.at)
        if number:
            self.at = number.end()
            return _number(number.group())
        word = SYMBOL.match(self.text, self.at)
        if not word:
            raise self.fail(f"unexpected {here!r}" if here else "unexpected end")
        self.at = word.end()
        name = word.group()
        if name in WORDS and self.peek() not in BRACKETS:
            return WORDS[name]
        if self.peek() in BRACKETS:
            bracket = self.peek()
            return Tagged(name, self.collection(), bracket)
        return Symbol(name)

    def collection(self) -> Any:
        opening = self.peek()
        self.at += 1
        items = self.entries(BRACKETS[opening])
        self.at += 1
        return _as_collection(self, items)

    def quoted(self, quote: str) -> str:
        self.at += 1
        out = []
        while True:
            if self.at >= len(self.text):
                raise self.fail("unclosed text")
            here = self.text[self.at]
            self.at += 1
            if here == quote:
                return "".join(out)
            if here == "\\" and quote == '"':
                out.append(self.escape())
            else:
                out.append(here)

    def escape(self) -> str:
        code = self.text[self.at : self.at + 1]
        self.at += 1
        if code in ESCAPES:
            return ESCAPES[code]
        digits = {"x": 2, "u": 4, "U": 8}.get(code)
        if not digits:
            raise self.fail(f"unknown escape \\{code}")
        hexed = self.text[self.at : self.at + digits]
        self.at += digits
        return chr(int(hexed, 16))


@dataclass
class _Entry:
    key: str
    value: Any


def _number(written: str) -> int | float:
    digits = written.replace("_", "")
    lowered = digits.lower().lstrip("+-")
    if lowered.startswith(("0x", "0b")):
        return int(digits, 0)
    return float(digits) if any(mark in lowered for mark in ".e") else int(digits)


def _as_collection(reader: _Reader, items: list) -> Any:
    entries = [item for item in items if isinstance(item, _Entry)]
    if not entries:
        return items
    if len(entries) != len(items):
        raise reader.fail("a group mixes key: value entries with plain items")
    return {entry.key: entry.value for entry in entries}


def parse(text: str) -> Any:
    """The Python value of warp data text."""
    return _Reader(text).document()


def _written_key(key: str) -> str:
    return key if SYMBOL.fullmatch(key) and key not in WORDS else _written_text(key)


def _written_text(text: str) -> str:
    out = []
    for position, character in enumerate(text):
        if character in WRITTEN_ESCAPES:
            out.append(WRITTEN_ESCAPES[character])
        elif character == "$" and text.startswith(INTERPOLATION_START, position):
            out.append(DOLLAR_ESCAPE)
        elif ord(character) < 32:
            out.append(f"\\x{ord(character):02x}")
        else:
            out.append(character)
    return '"' + "".join(out) + '"'


def serialize(value: Any) -> str:
    """Warp text that `parse` reads back to an equal value."""
    if isinstance(value, Tagged):
        return value.name + serialize_body(value.body, value.bracket)
    if isinstance(value, Symbol):
        return str(value)
    if value is None:
        return "ø"
    if isinstance(value, bool):
        return "yes" if value else "no"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError(f"{value} has no warp spelling")
        return repr(value)
    if isinstance(value, str):
        return _written_text(value)
    if isinstance(value, (dict, list, tuple)):
        return serialize_body(value, "[" if not isinstance(value, dict) else "{")
    raise TypeError(f"cannot write {type(value).__name__} as warp")


def serialize_body(body: Any, bracket: str) -> str:
    if isinstance(body, dict):
        inside = ", ".join(f"{_written_key(str(key))}: {serialize(item)}" for key, item in body.items())
    else:
        inside = ", ".join(serialize(item) for item in body)
    return bracket + inside + BRACKETS[bracket]
