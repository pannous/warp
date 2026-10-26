#!/usr/bin/env python3
"""Split core/emitter.warp into used files by moving whole top-level items (card emitter-split).

An item is a top-level definition (function, class, constant) with the comment lines right above it. The plan
(PLAN below, or a file of that form) names each new file, its header comment and the items it takes; emitter.warp keeps the
rest and `use`s the new files. A pure move: no line changes, only its place.

  emitter_split.py list             the items: line, name, first comment line
  emitter_split.py apply [plan]     move the items as the plan says (default PLAN)
"""
import re
import sys
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parent.parent
COMPILER = REPOSITORY / "lib" / "compiler"
EMITTER = COMPILER / "emitter.warp"
# each `file:` of core gets the emitter.warp items named below it, in this order (`first..last`: the items
# between, as emitter.warp has them); a `#` line is the file's header
PLAN = """
file: emitter/encoding.warp
# The bytes of a wasm module (emitter.warp's part): LEB128 numbers, little-endian ints, vectors, names, sections
uleb..section

file: emitter/module_layout.warp
# The module's sections in order (emitter.warp's part): types, imports, functions, memory, globals, exports, code, data
# and the names section
value_type..import_entry

file: emitter/literals.warp
# Literals (emitter.warp's part): exact decimals, texts in the data segment, one code point as a character
exact_decimal_digits..text_offset one_code_point..text_bytes

file: emitter/lists.warp
# Lists (emitter.warp's part): literals, length, items read and written by index
list_value vector_code index_code..whole_index

file: emitter/instances.warp
# Maps and class instances (emitter.warp's part): literals, the declared classes and fields, field reads and writes
entry_code..text_value_of base_class_layers..field_defaults field_checks..checks_fields

file: emitter/runtime_words.warp
# The runtime words a program links (emitter.warp's part): wasm intrinsics, library words, type tests on boxed values
wasm_instruction..ratio_part

file: emitter/output.warp
# Output (emitter.warp's part): the text a value shows as and print writing it to stdout through WASI's fd_write
write_output..runtime_call

file: emitter/operators.warp
# Operators (emitter.warp's part): arithmetic and comparisons on numbers and on boxed values, equality, ø tests
operation_code..euclidean_remainder

file: emitter/functions.warp
# Functions (emitter.warp's part): calls and their arguments, each function's kinds and code, analyzer.warp's kinds
call..widen_parameter new_body..body_code

file: emitter/closures.warp
# Closures (emitter.warp's part): closure values, their calls through dispatchers, the text a closure shows
closure_call_prefix..dispatcher_code

file: emitter/control_flow.warp
# Statements and control flow (emitter.warp's part): try, if, loops with break and continue, statements for effect,
# sequences, appends in place
ran_without_error..appends_only

file: emitter/quotes.warp
# Quoted code (emitter.warp's part): `data e` as a node, the fields of a code view (line, column, comment)
is_quote..quoted_code

file: emitter/host.warp
# Host words (emitter.warp's part): the imports of the "host" module, the builders a host word makes nodes with,
# exported functions, runtime globals
host_module..runtime_global

file: emitter/times.warp
# Times (emitter.warp's part): instants, durations and zoned times as $Nodes, their intrinsics and host words
nanos_per_milli..time_intrinsic
"""
DEFINITION = re.compile(r"^(?!use )(?:def |class |enum |type )?([A-Za-z_][A-Za-z_0-9]*)")


def items_of(lines):
    """The header (lines before the first item) and the items as (name, first line index, lines)"""
    starts = []
    for index, line in enumerate(lines):
        if line and not line[0].isspace() and line[0] not in "#}])" and DEFINITION.match(line):
            starts.append(index)
    # each item begins at its comment block: the comment lines right above the definition
    beginnings = []
    for start in starts:
        begin = start
        while begin > 0 and lines[begin - 1].startswith("#") and (not beginnings or begin - 1 >= beginnings[-1]):
            begin -= 1
        beginnings.append(begin)
    header = lines[: beginnings[0]] if beginnings else lines
    items = []
    for number, (begin, start) in enumerate(zip(beginnings, starts)):
        end = beginnings[number + 1] if number + 1 < len(beginnings) else len(lines)
        name = DEFINITION.match(lines[start]).group(1)
        items.append((name, begin, lines[begin:end]))
    return header, items


def read_plan(text):
    """[(file, header lines, names)] from the plan: `file: name.warp`, `# header` lines, then item names"""
    files = []
    for raw in text.splitlines():
        line = raw.rstrip()
        if not line.strip() or line.startswith("//"):
            continue
        if line.startswith("file:"):
            files.append((line.split(":", 1)[1].strip(), [], []))
        elif line.startswith("#"):
            files[-1][1].append(line)
        else:
            files[-1][2].extend(line.split())
    return files


def apply(plan_text):
    lines = EMITTER.read_text().split("\n")
    header, items = items_of(lines)
    by_name = {}
    for item in items:
        by_name.setdefault(item[0], []).append(item)
    def named(file, name):
        if name not in by_name:
            sys.exit(f"{file}: no item {name} in emitter.warp")
        return by_name[name]

    def selected(file, token):
        """the items a plan token names: `name` (each definition of it), `first..last` (the items between, in order)"""
        if ".." not in token:
            return named(file, token)
        first, last = token.split("..")
        start, end = items.index(named(file, first)[0]), items.index(named(file, last)[-1])
        return items[start : end + 1]

    moved = set()
    uses = []
    for file, file_header, tokens in read_plan(plan_text):
        body = []
        for item in (item for token in tokens for item in selected(file, token)):
            if item[1] in moved:
                sys.exit(f"{file}: {item[0]} is moved twice")
            moved.add(item[1])
            body.extend(item[2])
        while body and body[-1] == "":
            body.pop()
        # as runtime/*.warp: a part names no `use` of its own, emitter.warp's are the parts' too
        (COMPILER / file).parent.mkdir(exist_ok=True)
        (COMPILER / file).write_text("\n".join(file_header + body) + "\n")
        uses.append(f'use "{file}"')
    kept = [line for item in items if item[1] not in moved for line in item[2]]
    last_use = max(index for index, line in enumerate(header) if line.startswith("use "))
    EMITTER.write_text("\n".join(header[: last_use + 1] + uses + header[last_use + 1 :] + kept))
    print(f"moved {len(moved)} of {len(items)} items into {len(uses)} files")


def list_items():
    _, items = items_of(EMITTER.read_text().split("\n"))
    for name, begin, item_lines in items:
        comment = next((line for line in item_lines if line.startswith("#")), "")
        print(f"{begin + 1:5} {len(item_lines):4} {name:28} {comment[:90]}")


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else "list"
    if command == "apply":
        apply(Path(sys.argv[2]).read_text() if len(sys.argv) > 2 else PLAN)
    else:
        list_items()
