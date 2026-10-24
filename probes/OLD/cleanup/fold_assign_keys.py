#!/usr/bin/env python3
"""key(target, Op::Assign, value) → nodes::assign(target, value) over src/lowering (final-lowering pass).
Skips files with their own `assign` (fn or local binding) and the files warp-dispatch owns."""
import re, subprocess, sys

OWNED_ELSEWHERE = ("library_words", "saved_words", "property_accessors", "member_phrases", "of_lookups", "data/members", "nodes.rs")
CALL = re.compile(r"\bkey\(")

def top_level_commas(text, start):
    """Positions of depth-1 commas and the closing paren of the call opened at text[start-1]"""
    depth, commas, i, quote = 1, [], start, None
    while i < len(text):
        c = text[i]
        if quote:
            if c == "\\": i += 1
            elif c == quote: quote = None
        elif c == '"': quote = c
        elif c in "([{": depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0: return commas, i
        elif c == "," and depth == 1: commas.append(i)
        i += 1
    return None, None

def fold(text):
    out, pos, count = [], 0, 0
    for match in CALL.finditer(text):
        if match.start() < pos: continue
        commas, end = top_level_commas(text, match.end())
        if not commas or len(commas) < 2: continue
        if text[commas[0] + 1:commas[1]].strip() != "Op::Assign": continue
        out.append(text[pos:match.start()])
        out.append("assign(" + text[match.end():commas[0]] + "," + text[commas[1] + 1:end + 1])
        pos, count = end + 1, count + 1
    out.append(text[pos:])
    return "".join(out), count

def main():
    files = subprocess.run(["git", "ls-files", "src/lowering"], capture_output=True, text=True).stdout.split()
    for path in files:
        if not path.endswith(".rs") or any(owned in path for owned in OWNED_ELSEWHERE): continue
        text = open(path).read()
        if re.search(r"fn assign\b|let assign\b|\bassign\s*=>|mut assign\b", text): 
            if CALL.search(text) and "Op::Assign" in text: print("skip (own assign):", path, file=sys.stderr)
            continue
        folded, count = fold(text)
        if count:
            open(path, "w").write(folded)
            print(count, path)

if __name__ == "__main__":
    main()
