#!/usr/bin/env python3
"""Writes baseline.warp (the loop with a trivial lookup), chain.warp (a 70-arm text match) and tree.warp (the same match as a binary search on text order), into the folder given (default: here), each looking up
every key ROUNDS times; timed with the shared warp binary to see what a decision tree would gain."""
import sys

ROUNDS = 20000
OUT = sys.argv[1] if len(sys.argv) > 1 else "."
KEYS = ["...", "===", "!==", "..<", "&&=", "||=", "^^=", "**=", "÷=", "??", ":=", "~~", "::", "->", "=>", "**", "+=",
        "-=", "*=", "/=", "%=", "^=", "<<", ">>", "<=", ">=", "==", "!=", "!", "¬", "..", "&&", "||", ":", "=", ".",
        "+", "±", "-", "*", "×", "⋅", "/", "÷", "%", "^", "<", ">", "≤", "≥", "≠", "≈", "~", "&", "|", "#", "?", "…",
        "↑", "↓", "then", "else", "and", "xor", "or", "is", "do", "to", "as", "mod"]


def quoted(key):
    return '"' + key + '"'


def tree(keys, values, depth=1):
    tab = "\t" * depth
    if len(keys) <= 2:
        tests = " else ".join(f"if s == {quoted(k)} then {values[k]}" for k in keys)
        return f"{tests} else 0"
    middle = len(keys) // 2
    return (f"if s < {quoted(keys[middle])} then (\n{tab}\t{tree(keys[:middle], values, depth + 1)}\n{tab}) "
            f"else (\n{tab}\t{tree(keys[middle:], values, depth + 1)}\n{tab})")


def program(lookup):
    listed = ", ".join(quoted(k) for k in KEYS)
    return f"""keys = [{listed}]
{lookup}
total = 0
for round in 1 to {ROUNDS} {{
	for key in keys {{ total += look(key) }}
}}
print total
"""


values = {key: index + 1 for index, key in enumerate(KEYS)}
arms = "\n".join(f"\t{quoted(k)}: {values[k]}" for k in KEYS)
open(f"{OUT}/chain.warp", "w").write(program(f"look(s) := match s {{\n{arms}\n\t_: 0\n}}"))
open(f"{OUT}/tree.warp", "w").write(program(f"look(s) := {tree(sorted(KEYS), values)}"))
open(f"{OUT}/baseline.warp", "w").write(program("look(s) := count(s)"))
