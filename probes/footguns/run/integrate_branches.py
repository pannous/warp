"""Cherry-picks agent branches onto the current branch; conflicts in append-only files keep both sides, others stop."""
import re, subprocess, sys

APPEND_ONLY = {"notes/footguns.md", "probes/footguns/cases.warp"}
BRANCHES = sys.argv[1:]
CONFLICT = re.compile(r"^<<<<<<< [^\n]*\n(.*?)^(?:\|\|\|\|\|\|\| [^\n]*\n.*?)?^=======\n(.*?)^>>>>>>> [^\n]*\n", re.S | re.M)

def git(*args, check=True):
    return subprocess.run(["git", *args], capture_output=True, text=True, check=check).stdout

def keep_both_sides(path):
    text = open(path).read()
    open(path, "w").write(CONFLICT.sub(lambda m: m.group(1) + m.group(2), text))

def resolve_or_stop():
    conflicted = git("diff", "--name-only", "--diff-filter=U").split()
    code_conflicts = [f for f in conflicted if f not in APPEND_ONLY]
    if code_conflicts:
        sys.exit(f"code conflict, resolve by hand: {code_conflicts}")
    for path in conflicted:
        keep_both_sides(path)
    git("add", *conflicted)
    subprocess.run(["git", "-c", "core.editor=true", "cherry-pick", "--continue"], check=True, capture_output=True)

def applied_subjects():
    return set(git("log", "--format=%s", "origin/main..HEAD").splitlines())

if git("rev-parse", "-q", "--verify", "CHERRY_PICK_HEAD", check=False).strip():
    resolve_or_stop()
for branch in BRANCHES:
    for commit in git("rev-list", "--reverse", f"origin/main..origin/claude/{branch}").split():
        if git("log", "-1", "--format=%s", commit).strip() in applied_subjects():
            continue
        if subprocess.run(["git", "cherry-pick", commit], capture_output=True).returncode != 0:
            resolve_or_stop()
    print(f"{branch}: integrated")
