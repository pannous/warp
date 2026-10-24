#!/usr/bin/env python3
"""scripts/move_module.py follows a moved file's path written as joined parts (card py-misses):
os.path.join(REPOSITORY, "src", "ffi_parser.rs") broke 9 browser tests after layout-root."""
import importlib.util
import os

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
spec = importlib.util.spec_from_file_location("move_module", os.path.join(REPO, "scripts", "move_module.py"))
move_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(move_module)

CASES = [
	# (parent, module, folder, before, after)
	("", "ffi_parser", "ffi", 'os.path.join(REPOSITORY, "src", "ffi_parser.rs")',
		'os.path.join(REPOSITORY, "src", "ffi", "ffi_parser.rs")'),
	("", "ffi_parser", "ffi", "Path(ROOT) / 'src' / 'ffi_parser.rs'", "Path(ROOT) / 'src' / 'ffi' / 'ffi_parser.rs'"),
	("wasm_emitter", "arithmetic", "numbers", 'join(root, "src", "wasm_emitter", "arithmetic")',
		'join(root, "src", "wasm_emitter", "numbers", "arithmetic")'),
	("wasm_emitter", "arithmetic", "numbers", 'join("src", "arithmetic.rs")', 'join("src", "arithmetic.rs")'),
	("", "real", "numbers", 'join("src", "real_numbers.rs")', 'join("src", "real_numbers.rs")'),
]

failures = 0
for parent, module, folder, before, after in CASES:
	got = move_module.rewrite_joined_path(before, parent, module, folder)
	if got != after:
		failures += 1
		print(f"FAIL {before!r}\n  got      {got!r}\n  expected {after!r}")
print("ok" if not failures else f"{failures} failed")
exit(1 if failures else 0)
