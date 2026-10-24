#!/usr/bin/env python3
"""Cluster suspiciously similar (not identical) function names in src/ and rank the pairs by body similarity.

Usage: probes/cleanup/similar_names.py [repo] [out.md]   (defaults: this checkout, data/restructuring/similar_names_raw.md)
       probes/cleanup/similar_names.py show <name>...      prints every definition of these names with its body
Clusters: same word stems after dropping filler words and folding synonyms, and small edit distance.
Each pair gets a body similarity (difflib over tokens, own names masked) and the lines a merge could save.
"""
import difflib, itertools, re, sys
from collections import defaultdict
from pathlib import Path

SHOW = sys.argv[1:2] == ["show"]
REPO = Path(sys.argv[1] if len(sys.argv) > 1 and not SHOW else Path(__file__).resolve().parents[2])
OUT = Path(sys.argv[2]) if len(sys.argv) > 2 and not SHOW else REPO / "data/restructuring/similar_names_raw.md"
FILLER = {"a", "the", "of", "in", "on", "at", "to", "from", "for", "with", "by", "as", "and", "or", "into", "all",
          "get", "is", "has", "do", "emit", "try", "make", "new", "maybe", "any", "some", "fn", "impl", "inner", "helper"}
SYNONYMS = {"lookup": "find", "resolve": "find", "search": "find", "locate": "find", "fetch": "find",
            "walk": "scan", "visit": "scan", "traverse": "scan", "collect": "scan", "gather": "scan",
            "string": "text", "str": "text", "items": "list", "elements": "list", "vec": "list",
            "ident": "name", "identifier": "name", "build": "make", "create": "make", "construct": "make",
            "check": "test", "matches": "test", "names": "name", "words": "word", "nodes": "node",
            "args": "arg", "arguments": "arg", "argument": "arg", "params": "param", "parameters": "param",
            "parameter": "param", "fields": "field", "types": "type", "keys": "key", "statements": "statement",
            "stmts": "statement", "stmt": "statement", "exprs": "expr", "expression": "expr", "values": "value"}
MAX_CLUSTER = 14       # bigger stem clusters are generic words (name, node): only their close pairs are listed
MIN_EDIT_LEN = 8       # edit-distance pairs only between names this long
MIN_SIMILARITY = 0.55  # pairs below this body similarity are not reported
FN = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+\"C\"\s+)?fn\s+(\w+)")
TOKEN = re.compile(r"[A-Za-z_]\w*|\d+|\S")


def function_bodies():
    for path in sorted((REPO / "src").rglob("*.rs")):
        lines = path.read_text(errors="replace").splitlines()
        for index, line in enumerate(lines):
            match = FN.match(line)
            if match:
                yield match.group(1), str(path.relative_to(REPO)), index + 1, body_lines(lines, index)


def body_lines(lines, start):
    depth, opened = 0, False
    for end in range(start, min(len(lines), start + 3000)):
        code = re.sub(r'"(?:\\.|[^"\\])*"|//.*', "", lines[end])
        depth += code.count("{") - code.count("}")
        opened = opened or "{" in code
        if (opened and depth <= 0) or (not opened and code.rstrip().endswith(";")):
            return lines[start:end + 1]
    return lines[start:start + 1]


def core(name):
    words = [SYNONYMS.get(word, word) for word in name.lower().split("_") if word]
    return frozenset(word for word in words if word not in FILLER)


def tokens(body, name):
    return [("NAME" if token == name else token) for token in TOKEN.findall("\n".join(body[1:]))]


def near_names(names):
    """Pairs within edit distance 2 (one deletion on each side, symmetric-delete buckets)."""
    buckets = defaultdict(set)
    for name in names:
        buckets[name].add(name)
        for i in range(len(name)):
            buckets[name[:i] + name[i + 1:]].add(name)
    return {pair for bucket in buckets.values() for pair in itertools.combinations(sorted(bucket), 2)}


def show(names):
    for name, file, line, body in function_bodies():
        if name in names:
            print(f"── {name} {file}:{line}")
            print("\n".join(body))


def main():
    if SHOW:
        return show(set(sys.argv[2:]))
    functions = list(function_bodies())
    by_name = defaultdict(list)
    for function in functions:
        by_name[function[0]].append(function)
    clusters = defaultdict(set)
    for name in by_name:
        key = core(name)
        if key:
            clusters[" ".join(sorted(key))].add(name)
    for left, right in near_names(sorted(name for name in by_name if len(name) >= MIN_EDIT_LEN)):
        if core(left) != core(right):
            clusters[f"~{left}"].update({left, right})
    pairs = {}
    for key, members in clusters.items():
        if len(members) < 2:
            continue
        for left, right in itertools.combinations(sorted(members), 2):
            for a, b in itertools.product(by_name[left], by_name[right]):
                pair_key = (a[1], a[2], b[1], b[2])
                if pair_key in pairs:
                    continue
                matcher = difflib.SequenceMatcher(None, tokens(a[3], left), tokens(b[3], right), autojunk=False)
                similarity = matcher.ratio()
                if similarity >= MIN_SIMILARITY or len(members) <= MAX_CLUSTER and similarity >= 0.35:
                    saved = int(min(len(a[3]), len(b[3])) * similarity)
                    pairs[pair_key] = (saved, similarity, key, a, b)
    groups = sorted(similar_groups(pairs.values()), key=lambda group: -group[0])
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w") as out:
        out.write(f"# Similar function names ({len(functions)} functions, {len(pairs)} candidate pairs, "
                  f"{len(groups)} groups)\n\nsaved≈ = (all bodies − the longest) × mean similarity; "
                  "stems = shared words after folding synonyms (~name = edit distance ≤ 2)\n\n")
        for saved, similarity, stems, members in groups:
            out.write(f"## saved≈{saved} sim {similarity:.2f} [{stems}]\n")
            for name, file, line, body in members:
                out.write(f"- `{name}` {file}:{line} ({len(body)} lines)\n")
            out.write("\n")
    print(f"{len(functions)} functions, {len(pairs)} pairs, {len(groups)} groups -> {OUT}")


def similar_groups(pairs):
    """Connected components of the similar pairs, each with its estimated saving."""
    parent = {}

    def root(key):
        while parent.setdefault(key, key) != key:
            key = parent[key]
        return key
    for _, _, _, a, b in pairs:
        parent[root(a[1:3])] = root(b[1:3])
    components = defaultdict(lambda: ({}, [], set()))
    for _, similarity, stems, a, b in pairs:
        members, similarities, all_stems = components[root(a[1:3])]
        members[a[1:3]], members[b[1:3]] = a, b
        similarities.append(similarity)
        all_stems.add(stems)
    for members, similarities, all_stems in components.values():
        sizes = [len(member[3]) for member in members.values()]
        similarity = sum(similarities) / len(similarities)
        yield (int((sum(sizes) - max(sizes)) * similarity), similarity, ", ".join(sorted(all_stems)),
               sorted(members.values(), key=lambda member: member[1:3]))


if __name__ == "__main__":
    main()
