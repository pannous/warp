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
public_fn = re.compile(r"^(\s*)pub ((?:const |unsafe |async )*fn ([a-z_][a-z_0-9]*))", re.M)
for f in subprocess.run(["git","ls-files","src"],capture_output=True,text=True).stdout.split():
    if not f.endswith(".rs") or f in ("src/main.rs","src/page_main.rs") or f.startswith("src/cli/"): continue
    text = open(f).read()
    changed = public_fn.sub(lambda m: m.group(0) if m.group(3) in outside else f"{m.group(1)}pub(crate) {m.group(2)}", text)
    changed = re.sub(r"^(\s*)pub use ", r"\1pub(crate) use ", changed, flags=re.M)
    if changed != text: open(f,"w").write(changed)
