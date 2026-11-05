# path and file words against python

`gen_path_cases.py` writes `path_cases.json`: 157 cases of lib/path (path_join, normalize_path, relative_path,
basename, dirname, extension, stem, glob_match) and lib/file (read/write, append_file, read_bytes/write_bytes, exists,
list_files, glob), each with python 3.13's answer (posixpath.join, normpath, relpath; pathlib name, parent, suffix,
stem, full_match; the os module and glob.glob for the file system). Edge cases: trailing slashes, .. past the root,
dotfiles, double extensions (.tar.gz), unicode names and empty strings. File cases name the folder TMP; the runner
uses a fresh temp dir. `path_test.py` prints counts by word group and `--fails GROUP|all` lists differences, each
marked by design or held back for a decision. lib has no absolute or walk word, so they aren't covered (no new API).
Outside smoke; run from the repo root.
