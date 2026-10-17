#!/usr/bin/env python3
"""Delete a function (with its doc comments and attributes) from a Rust file: probes/cleanup/drop_function.py <file> <name>...
Used for merging similar-name duplicates (data/restructuring/similar_names.md); bodies found like similar_names.py."""
import re, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from similar_names import FN, body_lines

path = Path(sys.argv[1])
for name in sys.argv[2:]:
    lines = path.read_text().splitlines(keepends=True)
    starts = [i for i, line in enumerate(lines) if (m := FN.match(line)) and m.group(1) == name]
    if len(starts) != 1:
        sys.exit(f"{path}: {len(starts)} definitions of {name}")
    start = starts[0]
    end = start + len(body_lines([line.rstrip("\n") for line in lines], start))
    while start > 0 and re.match(r"\s*(///|#\[)", lines[start - 1]):
        start -= 1
    if start > 0 and lines[start - 1].strip() == "" and end < len(lines) and lines[end].strip() == "":
        end += 1
    path.write_text("".join(lines[:start] + lines[end:]))
    print(f"{path}: dropped {name} ({end - start} lines)")
