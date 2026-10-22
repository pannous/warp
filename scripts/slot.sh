#!/bin/zsh
# Runs a heavy warp command in one of the Mac's fleet-wide slots (user 2026-10-22, via the Grok Supervisor): self
# builds, pure compiles of compiler files, test sweeps. At most heavy_slots run at once; a further command waits for a
# free slot. A sweep runs its programs serially inside one slot. Inside a slot WARP_SLOT names the slot's directory
# (a command can keep its scratch files there), and a nested slot.sh runs its command at once.
# usage: scripts/slot.sh <command> [<args>…]   (exit status: the command's)
slots=/Volumes/Build/self-check/slot  # off the Data volume (fseventsd load)
heavy_slots=2

[[ -n $WARP_SLOT ]] && exec "$@"

# a slot is a directory holding its holder's pid; a slot whose holder died is freed
take_slot() {
  local slot holder
  mkdir -p ${slots:h}
  while true; do
    for slot in {1..$heavy_slots}; do
      if mkdir $slots.$slot.lock 2>/dev/null; then echo $$ > $slots.$slot.lock/pid; WARP_SLOT=$slots.$slot.lock; return; fi
      holder=$(cat $slots.$slot.lock/pid 2>/dev/null)
      [[ -n $holder ]] && ! kill -0 $holder 2>/dev/null && rm -rf $slots.$slot.lock
    done
    sleep 5
  done
}
take_slot
trap "rm -rf $WARP_SLOT" EXIT
export WARP_SLOT
"$@"
exit $?
