#!/bin/zsh
# Keeps `warp pure`'s and `warp self`'s compilers (compiler.wasm, compiler-self.wasm beside the binary) built from the
# main checkout's lib/, so they are ready whenever the user runs them (user 2026-10-22: "whenever the lib changes, the
# WASM compiler is automatically updated as well"); `warp self` itself never rebuilds a stale one. Cron runs this
# every minute: it rebuilds once lib/ (or the binary) changed and lib/ has been quiet for QUIET_SECONDS.
# usage: scripts/refresh_compilers.sh   (log: data/refresh_compilers.log)
checkout=/Users/me/dev/angles/warp
warp=/Users/me/dev/bin/warp
stamp=$checkout/data/refresh_compilers.stamp
log=$checkout/data/refresh_compilers.log
lock=$checkout/scratch/refresh_compilers.lock
QUIET_SECONDS=60

cd $checkout || exit 1
mkdir -p ${stamp:h} ${lock:h}

# lib/'s content plus the binary it is built with: either changing makes the built compilers stale
fingerprint() {
  { find lib -type f -name '*.warp' -print0 | sort -z | xargs -0 shasum; stat -f %m $warp; } | shasum | cut -c1-16
}

# seconds since the newest change in lib/
quiet_for() {
  local newest=$(find lib -type f -name '*.warp' -exec stat -f %m {} + | sort -n | tail -1)
  echo $(( $(date +%s) - newest ))
}

now=$(fingerprint)
[[ $now == $(cat $stamp 2>/dev/null) ]] && exit 0
(( $(quiet_for) >= QUIET_SECONDS )) || exit 0
# the Integrator's binary refresh builds both compilers itself: building them twice at once only doubles the load
pgrep -qf integrator_tools/refresh_main_binary.sh && exit 0
if ! mkdir $lock 2>/dev/null; then
  holder=$(cat $lock/pid 2>/dev/null)
  [[ -n $holder ]] && kill -0 $holder 2>/dev/null && exit 0
  rm -rf $lock; mkdir $lock || exit 0
fi
echo $$ > $lock/pid
trap "rm -rf $lock" EXIT

export WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
started=$(date +%s)
for compiler in pure self; do
  if ! nice $warp rebuild $compiler >/dev/null 2>>$log; then
    echo "$(date '+%F %T') FAILED: warp rebuild $compiler of lib $now (main $(git rev-parse --short HEAD))" >> $log
    osascript -e "display notification \"warp rebuild $compiler failed: see data/refresh_compilers.log\" with title \"warp compilers\"" 2>/dev/null
  fi
done
# a failed build is not retried until lib/ changes again: the log and the notification name it
echo $now > $stamp
echo "$(date '+%F %T') rebuilt pure and self for lib $now (main $(git rev-parse --short HEAD)) in $(( $(date +%s) - started )) s" >> $log
