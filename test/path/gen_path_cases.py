#!/usr/bin/env python3
# Writes test/path/path_cases.json: lib/path and lib/file words against python's posixpath and pathlib (3.13), each
# case [group, warp expression, expected value (JSON), the python reference]. File system cases name the folder
# TMP, replaced by a fresh temp dir by the runner. Run from the repo root.
import json, os, posixpath as pp
from pathlib import PurePosixPath as P
cases = []
def q(s): return json.dumps(s, ensure_ascii=False)
def add(group, expr, want, ref): cases.append([group, expr, want, ref])
J = [("a", "b"), ("a/", "b"), ("a", "/b"), ("", "b"), ("a", ""), ("/", "a"), ("a//", "b"), ("a", "b/"), ("a/b", "../c"),
     ("", ""), ("/a", "b/c"), ("ä", "ö.txt"), (".", "a"), ("a", "."), ("a", "~/b")]
for a, b in J: add("join", "path_join(%s, %s)" % (q(a), q(b)), pp.join(a, b), "posixpath.join")
N = ["a/./b//../c/", "/../x", "../a/../..", "", ".", "/", "//", "///a", "a/b/..", "a/..", "/a/b/../../..", "./a", "a//b",
     "a/b/./", "../", "/./", "..", "ä/../ö", ".hidden/./x", "a/../../b"]
for p in N: add("normalize", "normalize_path(%s)" % q(p), pp.normpath(p), "posixpath.normpath")
R = [("a/b", "a/c/d"), ("a", "a"), ("a/b", "a"), ("a", "a/b"), ("/a/b", "/a/c"), ("/", "/a"), ("/a", "/"), ("a/b/", "a/b/c"),
     ("a/./b", "a/b/c"), ("x", "y"), ("", "a"), ("a", ""), ("ä/b", "ä/c")]
for f, t in R: add("relative", "relative_path(%s, %s)" % (q(f), q(t)), pp.relpath(t or ".", f or "."), "posixpath.relpath(to, from)")
NAMES = ["a/b.txt", "a/b/", "/", "", "c", "/c", "a.tar.gz", ".bashrc", "a.", "..", ".", "a/b/c", ".x.y", "a..b", "dir.d/file",
         "ünï/cödé.md", "a/b.txt/", "//a", "x/.git"]
for p in NAMES:
    add("basename", "basename(%s)" % q(p), P(p).name, "pathlib name")
    add("dirname", "dirname(%s)" % q(p), str(P(p).parent), "pathlib parent")
    add("extension", "extension(%s)" % q(p), P(p).suffix, "pathlib suffix")
    add("stem", "stem(%s)" % q(p), P(p).stem, "pathlib stem")
G = [("a/*.txt", "a/b.txt"), ("*.txt", "a/b.txt"), ("a/**", "a/b/c"), ("**/a", "a"), ("**/c", "a/b/c"), ("a/?.txt", "a/b.txt"),
     ("a/?.txt", "a/bb.txt"), ("[ab].md", "a.md"), ("[!ab].md", "c.md"), ("[a-c]x", "bx"), ("*", ".hidden"), ("*.gz", "a.tar.gz"),
     ("a/*/c", "a/b/c"), ("a/*/c", "a/b/d/c"), ("ä*", "äb"), ("*", "")]
for pat, s in G: add("glob_match", "glob_match(%s, %s)" % (q(pat), q(s)), P(s).full_match(pat) if s else False, "pathlib full_match")
# the file system, in TMP
add("files", 'write("TMP/a.txt", "hello"); read("TMP/a.txt")', "hello", "open().read")
add("files", 'write("TMP/u.txt", "ünï 🌍\\nline"); read("TMP/u.txt")', "ünï 🌍\nline", "open().read")
add("files", 'write("TMP/e.txt", ""); read("TMP/e.txt")', "", "open().read")
add("files", 'write("TMP/o.txt", "one"); write("TMP/o.txt", "two"); read("TMP/o.txt")', "two", "open('w') truncates")
add("files", 'append_file("TMP/p.txt", "a"); append_file("TMP/p.txt", "b"); read("TMP/p.txt")', "ab", "open('a')")
add("bytes", 'write_bytes("TMP/b.bin", [0, 1, 255, 10]); read_bytes("TMP/b.bin")', [0, 1, 255, 10], "open('rb')")
add("bytes", 'write_bytes("TMP/z.bin", []); read_bytes("TMP/z.bin")', [], "open('rb')")
add("bytes", 'write("TMP/t.txt", "é"); read_bytes("TMP/t.txt")', list("é".encode()), "utf-8 bytes")
add("exists", 'exists("TMP")', True, "os.path.exists")
add("exists", 'exists("TMP/nope")', False, "os.path.exists")
add("exists", 'write("TMP/x.txt", "x"); exists("TMP/x.txt")', True, "os.path.exists")
add("exists", 'exists("")', False, "os.path.exists")
add("exists", 'write("TMP/.dot", "x"); exists("TMP/.dot")', True, "os.path.exists")
add("list_files", 'write("TMP/l/b.txt", "1"); write("TMP/l/a.md", "2"); write("TMP/l/.h", "3"); list_files("TMP/l").sort', sorted(["a.md", "b.txt", ".h"]), "os.listdir")
add("glob", 'write("TMP/g/a.txt", "1"); write("TMP/g/b.txt", "2"); write("TMP/g/c.md", "3"); glob("TMP/g/*.txt")', ["TMP/g/a.txt", "TMP/g/b.txt"], "glob.glob sorted")
add("glob", 'write("TMP/h/x1", "1"); write("TMP/h/x22", "2"); glob("TMP/h/x?")', ["TMP/h/x1"], "glob.glob sorted")
add("glob", 'glob("TMP/none/*.zz")', [], "glob.glob")
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
json.dump(cases, open(os.path.join(root, "test/path/path_cases.json"), "w"), ensure_ascii=False, indent=0)
print(len(cases), "cases")
