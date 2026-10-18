#!/usr/bin/env python3
"""The call heads the analyzer finds no definition for (card lib-compiler, builtin kinds): counts, over the lowered
programs of data/self_analyzer/cases.jsonl, every `name(…)` whose name the program does not define, most used first,
with the kinds Rust gives a variable assigned straight from such a call (`v = name(…)` → `v: text`), so the builtin
kinds the analyzer must know come from the corpus."""
import json, collections, sys

CASES = "data/self_analyzer/cases.jsonl"


def walk(node, visit):
    if isinstance(node, list):
        visit(node)
        for child in node[1:]:
            walk(child, visit)


def head_symbol(node):
    if isinstance(node, list) and node[:1] == ["list"] and len(node) > 3:
        first = node[3]
        if isinstance(first, list) and first[:1] == ["symbol"]:
            return first[1]
    return None


def assigned(node):
    """`v = call` or `v := call`: (v, call), else None"""
    if node[:1] == ["key"] and len(node) > 3 and node[1] in (":=", "="):
        return node[2], node[3]
    return None


def defined_names(program):
    names = set()

    def visit(node):
        pair = assigned(node)
        if pair:
            left = pair[0]
            name = head_symbol(left) or (left[1] if isinstance(left, list) and left[:1] == ["symbol"] else None)
            if name:
                names.add(name)

    walk(program, visit)
    return names


def is_call(node, defined):
    name = head_symbol(node)
    return name if name and node[1] == "(" and name not in defined else None


def main():
    counts, sources = collections.Counter(), collections.defaultdict(set)
    rust_kinds = collections.defaultdict(collections.Counter)
    for line in open(CASES):
        case = json.loads(line)
        program, defined = case["program"], defined_names(case["program"])
        variables = dict(kind.split(": ", 1) for kind in case["kinds"] if ": " in kind and "(" not in kind)

        def visit(node):
            name = is_call(node, defined)
            if name:
                counts[name] += 1
                sources[name].add(case["path"])
            pair = assigned(node)
            if pair and pair[0][:1] == ["symbol"]:
                called = is_call(pair[1], defined)
                if called and pair[0][1] in variables:
                    rust_kinds[called][variables[pair[0][1]]] += 1

        walk(program, visit)
    for name, count in counts.most_common(int(sys.argv[1]) if len(sys.argv) > 1 else 80):
        kinds = " ".join(f"{kind}×{n}" for kind, n in rust_kinds[name].most_common(3))
        print(f"{count:5} {len(sources[name]):4} {name:24} {kinds}")


main()
