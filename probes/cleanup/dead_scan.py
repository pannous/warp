# Repo-wide never-called function scan (card cleanup-dead-scan, notes/cleanup.md "Dead scan"): rustc says nothing about an
# unused `pub fn`, so this makes every pub fn crate-private whose name nothing outside the library mentions (tests,
# crates, web, the bins), and every `pub use` too; then `cargo check --lib --message-format short` (also with
# --all-features and for wasm32) lists the never-used ones. Undo after with `git diff > p.patch && git apply -R p.patch`.
# Methods with common names (len, new) stay pub when a test mentions the word: not exhaustive, no false positives
# except in-file #[cfg(test)] modules, which clippy --all-targets then reports
import re, subprocess
outside_files = subprocess.run(["git","ls-files","tests","crates","web","build.rs","src/main.rs","src/cli","src/page_main.rs"],capture_output=True,text=True).stdout.split()
outside = set()
for f in outside_files:
    if f.endswith((".rs",".js",".html",".ts")):
        try: outside |= set(re.findall(r"[A-Za-z_][A-Za-z_0-9]*", open(f).read()))
        except Exception: pass
public_use = re.compile(r"^(\s*)pub use ([^;]*);", re.M)
# a glob re-export, or one naming an item used outside, is how tests and the bins reach the items: keeping it pub
# keeps their callers from looking dead (2026-10-16: 97 false hits under src/law and src/cli users)
def reaches_outside(path): return path.endswith("*") or any(name in outside for name in re.findall(r"\w+", path.split("::")[-1]))
public_fn = re.compile(r"^(\s*)pub ((?:const |unsafe |async )*fn ([a-z_][a-z_0-9]*))", re.M)
library_files = [f for f in subprocess.run(["git","ls-files","src"],capture_output=True,text=True).stdout.split()
                 if f.endswith(".rs") and f not in ("src/main.rs","src/page_main.rs") and not f.startswith("src/cli/")]
for f in library_files:  # what a kept re-export names (`pub use x::{a, b as c}`) stays pub with it
    for m in public_use.finditer(open(f).read()):
        if reaches_outside(m.group(2)): outside |= set(re.findall(r"\w+", m.group(2)))
for f in library_files:
    text = open(f).read()
    changed = public_fn.sub(lambda m: m.group(0) if m.group(3) in outside else f"{m.group(1)}pub(crate) {m.group(2)}", text)
    changed = public_use.sub(lambda m: m.group(0) if reaches_outside(m.group(2)) else f"{m.group(1)}pub(crate) use {m.group(2)};", changed)
    if changed != text: open(f,"w").write(changed)
