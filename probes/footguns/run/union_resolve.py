"""Resolves conflicts in append-only files by keeping both sides (ours first, then theirs)."""
import re, sys
for path in sys.argv[1:]:
    text = open(path).read()
    pattern = re.compile(r"^<<<<<<< [^\n]*\n(.*?)^(?:\|\|\|\|\|\|\| [^\n]*\n.*?)?^=======\n(.*?)^>>>>>>> [^\n]*\n", re.S | re.M)
    open(path, "w").write(pattern.sub(lambda m: m.group(1) + m.group(2), text))
    assert "<<<<<<<" not in open(path).read(), path
