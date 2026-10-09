# Reads table 0 (names → text) of uniscript's USX1 index (format: uniscript/AGENTS.md "Index format") and reports what a
# parser table of single-word \:name entities would hold
import struct, sys
data = open(sys.argv[1], "rb").read()
assert data[:4] == b"USX1"
tables = struct.unpack_from("<I", data, 4)[0]
offset, count = struct.unpack_from("<II", data, 8)
names = {}
for i in range(count):
    _, key_at, key_len, value_at, value_len = struct.unpack_from("<5I", data, offset + 20 * i)
    names[data[key_at:key_at + key_len].decode()] = data[value_at:value_at + value_len].decode()
print("names", len(names))
import re
word = {k: v for k, v in names.items() if re.fullmatch(r"[A-Za-z]+", k) and v}
print("single-word names", len(word), "bytes", sum(len(k) + len(v.encode()) + 2 for k, v in word.items()))
hyphen = {k: v for k, v in names.items() if re.fullmatch(r"[A-Za-z0-9-]+", k.replace(" ", "-")) and v}
print("names as \\:a-b-c", len(hyphen), "bytes", sum(len(k) + len(v.encode()) + 2 for k, v in hyphen.items()))
for k in ["world", "alpha", "infinity", "leq", "nat", "Delta", "in", "to", "fracture A", "xyzzy", "alphabet"]:
    print(repr(k), repr(names.get(k)))
# warp's hand table (src/uniscript_entities.rs ENTITIES) against the index
if len(sys.argv) > 2:
    table = re.findall(r'\("(\w+)", \'(.)\'\)', open(sys.argv[2]).read())
    missing = [(k, c) for k, c in table if k not in names]
    differs = [(k, c, names[k]) for k, c in table if k in names and names[k] != c]
    print("warp entities", len(table), "missing in index", missing)
    print("different in index", differs)
