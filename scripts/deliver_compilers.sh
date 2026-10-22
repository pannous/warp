#!/bin/zsh
# Delivers `warp pure`'s and `warp self`'s compilers (compiler.wasm, compiler-self.wasm beside the binary) with every
# update of the main checkout (user 2026-10-22: "it should automatically be delivered with every update of main"):
# git's post-merge, post-rewrite and post-checkout hooks (~/.git-hooks) start it after a pull, the Integrator after
# installing a binary. It rebuilds each compiler whose build stamp names another lib/ path or fingerprint, and checks
# again until both match: a pull during a build (the 10:12 race of 2026-10-22) only makes it build once more.
# `warp pure` / `warp self` themselves never rebuild. usage: scripts/deliver_compilers.sh [--background]
checkout=/Users/me/dev/angles/warp
bin=/Users/me/dev/bin
log=$checkout/data/deliver_compilers.log
lock=$checkout/scratch/deliver_compilers.lock
MAX_ROUNDS=3

if [[ $1 == --background ]]; then
  [[ $(git rev-parse --show-toplevel 2>/dev/null) == $checkout ]] || exit 0
  mkdir -p ${log:h}
  nohup $0 >> $log 2>&1 &!
  exit 0
fi

cd $checkout || exit 1
mkdir -p ${lock:h}
if ! mkdir $lock 2>/dev/null; then
  holder=$(cat $lock/pid 2>/dev/null)
  # a running delivery checks again after its build, so it also covers this update
  [[ -n $holder ]] && kill -0 $holder 2>/dev/null && { touch $lock/again; exit 0; }
  rm -rf $lock; mkdir $lock || exit 0
fi
echo $$ > $lock/pid
trap "rm -rf $lock" EXIT

# the compilers whose stamp (warp.build section: lib path, lib fingerprint) differs from now; a new binary comes with
# its compilers from the Integrator's refresh
stale_compilers() {
  python3 -I - $checkout/lib $bin <<'EOF'
import hashlib, os, subprocess, sys
library, bin = sys.argv[1], sys.argv[2]
files = [os.path.join(d, f) for d, _, fs in os.walk(library) for f in fs]
files.sort(key=lambda p: p.split(os.sep))
fingerprint = hashlib.sha256()
for f in files:
    fingerprint.update((os.path.relpath(f, library) + "\n").encode())
    fingerprint.update(open(f, "rb").read())
for name, command in (("compiler", "pure"), ("compiler-self", "self")):
    try:
        data = open(f"{bin}/{name}.wasm", "rb").read()
        stamp = data[data.rindex(b"warp.build") + len(b"warp.build"):].split(b"\n")
        fresh = stamp[0].decode(errors="replace") == library and stamp[1].decode() == fingerprint.hexdigest()
    except (OSError, ValueError, IndexError):
        fresh = False
    if not fresh:
        print(command)
EOF
}

export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
for round in {1..$MAX_ROUNDS}; do
  rm -f $lock/again
  stale=($(stale_compilers))
  (( ${#stale} )) || exit 0
  for compiler in $stale; do
    started=$(date +%s)
    if nice $bin/warp rebuild $compiler > /dev/null; then
      echo "$(date '+%F %T') delivered $compiler for main $(git rev-parse --short HEAD) in $(( $(date +%s) - started )) s"
    else
      echo "$(date '+%F %T') FAILED: warp rebuild $compiler for main $(git rev-parse --short HEAD)"
      osascript -e "display notification \"warp rebuild $compiler failed: data/deliver_compilers.log\" with title \"warp compilers\"" 2>/dev/null
      exit 1
    fi
  done
done
echo "$(date '+%F %T') still stale after $MAX_ROUNDS rounds: lib/ keeps changing"
