#!/usr/bin/env bash
# Run a heavy warp job in one of 4 box slots. Slot 1 is the old single lock,
# so jobs still waiting on `flock /tmp/warp-box-heavy.lock` count against it.
# usage: /workspace/box-slot.sh <cmd> [args...]
#        /workspace/box-slot.sh --status     # show slots + FIFO queue
#        /workspace/box-slot.sh -h|--help
#
# Output (quiet by default, to save agent tokens): the job's stdout+stderr go to
# a log ($BOX_SLOT_LOG_DIR, default /tmp/warp-logs/slot-<ticket>-<cmd>.log). On
# stderr you get the slot line, then one summary line
#   box-slot: ok|FAIL rc=N <secs>s log=<path> | last: <last output line>
# and on failure up to 30 error lines (error:/panic/FAIL/failed/refused/Error(...)
# with 2 lines of context; the log tail if none match).
# VERBOSE=1 (or BOX_SLOT_VERBOSE=1) streams the job's output as before (no log);
# a caller whose stdout is already a regular file (`> x.log`) also gets the full
# output there.
# Exit status: the job's own; a signal death is 128+sig with a "killed: signal"
# line; an OOM kill of the job or of any process it started (seen in dmesg) is a
# failure ("killed: out of memory", rc 137) even when the job itself exited 0.
# SIGTERM/SIGINT/SIGHUP to box-slot are forwarded to the job.
#
# Fair waiting: every waiter takes a numbered ticket in $BOX_SLOT_QUEUE_DIR
# (counter bumped under flock). A waiter may only try the slot locks when the
# number of live waiters ahead of it is < the number of free slots. Waiters
# still running the pre-queue version of this script (no ticket) are counted
# by process start time, so they are not starved; they keep competing as before.
# Tickets whose PID is dead (or reused) are removed by any waiter.
# Slot 4 was added later: legacy waiters (and pre-slot-4 ticket holders, whose
# ticket lacks the "caps=all" line) can only use slots 1-3, so they only
# reserve slots 1-3; current waiters can use any slot.
#
# Env overrides (for tests; defaults = production values):
#   BOX_SLOT_LOCKS      space-separated lock paths (default: the 4 /tmp/warp-box-heavy*.lock)
#   BOX_SLOT_LEGACY_SLOTS  how many leading slots old versions know about (default 3)
#   BOX_SLOT_QUEUE_DIR  ticket dir (default /tmp/warp-box-queue)
#   BOX_SLOT_MIN_GB     MemAvailable guard in GB (default 4)
#   BOX_SLOT_POLL       seconds between attempts (default 2)
#   BOX_SLOT_LEGACY     1 = count ticketless old-version waiters as queued (default 1)
#   BOX_SLOT_LOG_DIR    job log dir (default /tmp/warp-logs)
#   BOX_SLOT_ERR_LINES  max error lines shown on failure (default 30)
if [ -n "${BOX_SLOT_LOCKS:-}" ]; then
  read -r -a slots <<<"$BOX_SLOT_LOCKS"
else
  slots=(/tmp/warp-box-heavy.lock /tmp/warp-box-heavy.2.lock /tmp/warp-box-heavy.3.lock /tmp/warp-box-heavy.4.lock)
fi
nleg=${BOX_SLOT_LEGACY_SLOTS:-3}
qdir=${BOX_SLOT_QUEUE_DIR:-/tmp/warp-box-queue}
min_gb=${BOX_SLOT_MIN_GB:-4}
poll=${BOX_SLOT_POLL:-2}
legacy=${BOX_SLOT_LEGACY:-1}
self_path=$(readlink -f -- "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")

# --- helpers ---------------------------------------------------------------
_starttime() {   # $1=pid -> process start time (clock ticks since boot), or fail
  local s
  s=$(cat "/proc/$1/stat" 2>/dev/null) || return 1
  s=${s##*) }
  local -a f
  read -r -a f <<<"$s"
  [ -n "${f[19]:-}" ] || return 1
  echo "${f[19]}"
}
_clk=$(getconf CLK_TCK 2>/dev/null || echo 100)
_uptime_ticks() { local u; read -r u _ </proc/uptime; echo $(( ${u%.*} * _clk )); }
_age() {         # $1=starttime ticks -> "1h02m03s"
  local s=$(( ( $(_uptime_ticks) - $1 ) / _clk )); [ $s -lt 0 ] && s=0
  printf '%dh%02dm%02ds' $((s/3600)) $((s%3600/60)) $((s%60))
}
_cmdof() {       # $1=pid -> command line (args after box-slot.sh if a wrapper)
  local -a a; local i
  mapfile -d '' -t a 2>/dev/null <"/proc/$1/cmdline" || return 1
  for ((i=0; i<${#a[@]}; i++)); do
    case "${a[i]}" in *box-slot.sh) echo "${a[*]:i+1}"; return 0;; esac
  done
  echo "${a[*]}"
}
_ticket_live() { # $1=ticket file; echo "pid start" if live, else remove + fail
  local pid st cur
  { read -r pid; read -r st; } 2>/dev/null <"$1"
  if [ -n "$pid" ] && [ -n "$st" ] && cur=$(_starttime "$pid") && [ "$cur" = "$st" ]; then
    echo "$pid $st"; return 0
  fi
  # empty file may be a ticket mid-write: only drop it if it is > 60 s old
  if [ -z "$pid" ] && [ -n "$(find "$1" -newermt '-60 seconds' 2>/dev/null)" ]; then return 1; fi
  rm -f -- "$1"; return 1
}
_holds_lock() {  # $1=pid $2=fd $3=lock path: does that fd hold an flock on the lock?
  [ "$(readlink "/proc/$1/fd/$2" 2>/dev/null)" = "$3" ] || return 1
  grep -q '^lock:.*FLOCK' "/proc/$1/fdinfo/$2" 2>/dev/null
}
_lock_holders() { # $1=lock path -> PIDs whose open fd holds the flock (top-most only)
  # /proc/locks is pid-namespaced on this box, so look at /proc/*/fdinfo instead.
  local l f pid fd pp; local -A seen=()
  l=$(readlink -f -- "$1" 2>/dev/null) || return 0
  while read -r f; do
    pid=${f#/proc/}; pid=${pid%%/*}; fd=${f##*/}
    _holds_lock "$pid" "$fd" "$l" && seen[$pid]=1
  done < <(find /proc/[0-9]*/fd -maxdepth 1 -lname "$l" 2>/dev/null)
  for pid in "${!seen[@]}"; do   # children inherit the fd; report only the top-most
    pp=$(awk '/^PPid:/{print $2}' "/proc/$pid/status" 2>/dev/null)
    [ -n "${seen[$pp]:-}" ] || echo "$pid"
  done
}
_slot_free() { ( exec 7>>"$1" && flock -n 7 ) 2>/dev/null; }
_legacy_waiters() { # "start pid" of ticketless box-slot.sh waiters not holding a slot
  [ "$legacy" = 1 ] || return 0
  local pid l st tick pp
  # order matters: read tickets first, then holders (a waiter takes the lock
  # before deleting its ticket, so it is always seen in one of the two)
  declare -A ticketed=()
  for tick in "$qdir"/t.*; do
    [ -e "$tick" ] || continue
    read -r pid 2>/dev/null <"$tick" && [ -n "$pid" ] && ticketed[$pid]=1
  done
  local -a rl=(); for l in "${slots[@]}"; do rl+=("$(readlink -f -- "$l" 2>/dev/null)"); done
  for pid in $(pgrep -f -- 'box-slot\.sh' 2>/dev/null); do
    [ "$pid" = "$$" ] && continue
    [ -n "${ticketed[$pid]:-}" ] && continue
    # holders keep fd 9 open (and locked) on a slot lock for the whole job
    local held=0; for l in "${rl[@]}"; do _holds_lock "$pid" 9 "$l" && held=1; done
    [ $held = 1 ] && continue
    local -a a; mapfile -d '' -t a 2>/dev/null <"/proc/$pid/cmdline" || continue
    [ ${#a[@]} -gt 0 ] || continue
    # skip forked subshells of a box-slot.sh process (same cmdline as parent)
    pp=$(awk '/^PPid:/{print $2}' "/proc/$pid/status" 2>/dev/null)
    [ -n "$pp" ] && cmp -s "/proc/$pid/cmdline" "/proc/$pp/cmdline" 2>/dev/null && continue
    # must be a shell running a box-slot.sh script (not grep/editors/--status)
    case "${a[0]##*/}" in bash|sh) ;; *) continue;; esac
    case "${a[1]:-}" in *box-slot.sh) ;; *) continue;; esac
    # only waiters of *this* script file (so test copies never mix with the real queue)
    local sp=${a[1]}; [ "${sp:0:1}" = / ] || sp="/proc/$pid/cwd/$sp"
    [ "$(readlink -f -- "$sp" 2>/dev/null)" = "$self_path" ] || continue
    [ "${a[2]:-}" = "--status" ] && continue
    st=$(_starttime "$pid") || continue
    echo "$st $pid"
  done
}

# --- --help ----------------------------------------------------------------
case "${1:-}" in
  -h|--help) sed -n '2,/^[^#]/{/^#/s/^# \{0,1\}//p}' "$self_path"; exit 0;;
  "") echo "box-slot: usage: box-slot.sh <cmd> [args...] | --status | --help" >&2; exit 2;;
esac

# --- --status --------------------------------------------------------------
if [ "${1:-}" = "--status" ] && [ $# -eq 1 ]; then
  avail_kb=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
  echo "MemAvailable: $((avail_kb/1024)) MB (slots need >= ${min_gb} GB)   queue dir: $qdir"
  echo "SLOTS:"
  i=0
  for lock in "${slots[@]}"; do
    i=$((i+1))
    hs=$(_lock_holders "$lock")
    tag=""; [ $i -gt "$nleg" ] && tag=" [new-version only]"
    if [ -z "$hs" ]; then
      if _slot_free "$lock"; then echo "  slot $i  $(basename "$lock")  FREE$tag"
      else echo "  slot $i  $(basename "$lock")  held (holder pid unknown)"; fi
    fi
    for h in $hs; do
      st=$(_starttime "$h") && age=$(_age "$st") || age="?"
      printf '  slot %d  %-24s pid %-8s age %-10s %s%s\n' "$i" "$(basename "$lock")" "$h" "$age" "$(_cmdof "$h")" "$tag"
    done
  done
  echo "QUEUE (FIFO order; 'legacy' = old-version waiter without ticket, ordered by start time):"
  {
    for tick in "$qdir"/t.*; do
      [ -e "$tick" ] || continue
      lv=$(_ticket_live "$tick") || continue
      set -- $lv
      # tickets are ordered by number; use start time only for merging with legacy
      k=ticket; grep -qx 'caps=all' "$tick" 2>/dev/null || k=ticket-1..$nleg
      echo "$2 $1 $k:$(basename "$tick")"
    done
    _legacy_waiters | while read -r st pid; do echo "$st $pid legacy"; done
  } | sort -n -k1,1 | {
    n=0
    while read -r st pid kind; do
      n=$((n+1))
      printf '  %2d. pid %-8s age %-10s %-24s %s\n' "$n" "$pid" "$(_age "$st")" "$kind" "$(_cmdof "$pid")"
    done
    [ $n -eq 0 ] && echo "  (empty)"
  }
  exit 0
fi

# --- run the job (quiet by default) -----------------------------------------
_desc_pids() {   # $1=root pid -> it and all descendants, one per line
  ps -e -o pid=,ppid= 2>/dev/null | awk -v r="$1" '{p[$1]=$2} END{s[r]=1; do{n=0; for(k in p) if(!(k in s) && (p[k] in s)){s[k]=1;n++}}while(n); for(k in s) print k}'
}
_run_job() {     # $1=lock, rest = command
  local lock=$1; shift
  local verbose=${BOX_SLOT_VERBOSE:-${VERBOSE:-0}} logdir=${BOX_SLOT_LOG_DIR:-/tmp/warp-logs}
  local tag=${n:-$$} name=${1##*/} log="" t0 up0 rc cpid spid
  [ -f /proc/self/fd/1 ] && verbose=1          # stdout already a file: keep it full
  name=${name//[^A-Za-z0-9._-]/_}
  if [ "$verbose" != 1 ]; then
    mkdir -p -- "$logdir" 2>/dev/null
    log="$logdir/slot-$tag-${name:0:40}.log"
    : >"$log" 2>/dev/null || { log=$(mktemp /tmp/box-slot-XXXXXX.log) || log=/dev/null; }
    echo "box-slot: $(basename "$lock") -> $* (log: $log)" >&2
  else
    echo "box-slot: $(basename "$lock") -> $*" >&2
  fi
  read -r up0 _ </proc/uptime; t0=$(date +%s)
  local pidf; pidf=$(mktemp /tmp/box-slot-pids.XXXXXX)
  if [ -n "$log" ]; then "$@" <&0 >"$log" 2>&1 &
  else "$@" <&0 & fi
  cpid=$!
  trap 'kill -TERM $cpid 2>/dev/null' TERM HUP
  trap 'kill -TERM $cpid 2>/dev/null' INT   # async jobs ignore SIGINT
  # remember the job's process tree, so a later OOM kill can be attributed to it
  ( exec 9>&-; while kill -0 $cpid 2>/dev/null; do _desc_pids $cpid >>"$pidf"; sleep 2; done ) </dev/null >/dev/null 2>&1 &
  spid=$!
  while :; do wait $cpid 2>/dev/null; rc=$?; kill -0 $cpid 2>/dev/null || break; done
  trap - TERM HUP INT
  kill $spid 2>/dev/null; wait $spid 2>/dev/null
  local dt=$(( $(date +%s) - t0 )) why="" oom=""
  # OOM kills since the job started whose victim was in its process tree
  local u pid line
  while read -r line; do
    u=${line#[}; u=${u%%]*}; u=${u// /}; u=${u%.*}
    [ -n "$u" ] && [ "$u" -ge "${up0%.*}" ] 2>/dev/null || continue
    pid=$(sed -n 's/.*Killed process \([0-9]*\).*/\1/p' <<<"$line")
    if [ -n "$pid" ] && grep -qx "$pid" "$pidf" 2>/dev/null; then oom=${line#*Killed process }; oom=${oom%%,*}; fi
  done < <(dmesg 2>/dev/null | grep 'Killed process')
  rm -f -- "$pidf"
  if [ -n "$oom" ]; then
    why="killed: out of memory (dmesg: process $oom)"; [ "$rc" -eq 0 ] && why="$why; the job itself exited 0"
    [ "$rc" -eq 0 ] && rc=137
  elif [ "$rc" -gt 128 ]; then
    why="killed: signal $(kill -l $((rc-128)) 2>/dev/null || echo $((rc-128)))"
    [ "$rc" -eq 137 ] && why="$why (SIGKILL: likely out of memory or killed by PID)"
  fi
  if [ -z "$why" ] && [ -n "$log" ] && [ -f "$log" ] && grep -qE '^Killed$|signal: (9|6|11), SIG|Segmentation fault|Aborted \(core' "$log"; then
    echo "box-slot: warning: the log shows a killed/crashed sub-step: $(grep -nE '^Killed$|signal: (9|6|11), SIG|Segmentation fault|Aborted \(core' "$log" | head -1 | cut -c1-160)" >&2
  fi
  local last=""
  [ -n "$log" ] && [ -f "$log" ] && last=$(grep -v '^[[:space:]]*$' "$log" | tail -1 | cut -c1-160)
  local status=ok; [ "$rc" -ne 0 ] && status=FAIL
  echo "box-slot: $status rc=$rc ${dt}s${log:+ log=$log}${last:+ | last: $last}" >&2
  [ -n "$why" ] && echo "box-slot: $why" >&2
  if [ "$rc" -ne 0 ] && [ -n "$log" ] && [ -s "$log" ]; then
    local cap=${BOX_SLOT_ERR_LINES:-30} pat='error(\[[A-Za-z0-9]+\])?:|panicked|panic:|FAIL|[Ff]ailed|refused|Error\(|CHECK RED|^Killed|SIGKILL|out of memory|No such file|command not found'
    if grep -qE "$pat" "$log"; then grep -nE -B2 -A2 "$pat" "$log" | head -n "$cap" >&2
    else echo "box-slot: no error lines matched; log tail:" >&2; tail -n 10 "$log" >&2; fi
  fi
  return "$rc"
}

# --- take a ticket ---------------------------------------------------------
ticket=""
my_start=$(_starttime $$)
if mkdir -p -- "$qdir" 2>/dev/null && chmod 1777 -- "$qdir" 2>/dev/null; [ -d "$qdir" ] && [ -w "$qdir" ]; then
  n=$( {
    flock 8 || exit 1
    c=$(cat "$qdir/seq" 2>/dev/null); c=${c//[^0-9]/}; c=$(( 10#${c:-0} + 1 ))
    echo "$c" >"$qdir/seq.tmp.$$" && mv -f "$qdir/seq.tmp.$$" "$qdir/seq" && echo "$c"
  } 8>>"$qdir/.lock" )
  if [ -n "$n" ]; then
    ticket=$(printf '%s/t.%012d.%d' "$qdir" "$n" $$)
    tmp="$qdir/.new.$$"
    printf '%s\n%s\n%s\n%s\ncaps=all\n' $$ "$my_start" "$(date +%s)" "${*//$'\n'/ }" >"$tmp" && mv -f "$tmp" "$ticket" || ticket=""
  fi
fi
[ -n "$ticket" ] && trap 'rm -f -- "$ticket"' EXIT
# (if the queue dir is unusable we fall back to the old unqueued behaviour)

_ahead() {       # "A L": live waiters ahead of us that can use any slot (A) / only slots 1-$nleg (L)
  local a=0 l=0 tick st pid
  for tick in "$qdir"/t.*; do
    [ -e "$tick" ] || continue
    [ "$tick" = "$ticket" ] && break
    _ticket_live "$tick" >/dev/null || continue
    if grep -qx 'caps=all' "$tick" 2>/dev/null; then a=$((a+1)); else l=$((l+1)); fi
  done
  while read -r st pid; do
    [ -n "$st" ] && [ "$st" -lt "$my_start" ] && l=$((l+1))
  done < <(_legacy_waiters)
  echo "$a $l"
}

while :; do
  avail=$(awk '/MemAvailable/{print int($2/1048576)}' /proc/meminfo)
  try=1
  if [ -n "$ticket" ] && [ "$avail" -ge "$min_gb" ]; then
    read -r ahead_a ahead_l < <(_ahead)
    if [ $((ahead_a + ahead_l)) -gt 0 ]; then
      fl=0 fn=0 i=0     # free legacy-visible slots (1..nleg) / free newer slots
      for lock in "${slots[@]}"; do
        i=$((i+1)); _slot_free "$lock" || continue
        if [ $i -le "$nleg" ]; then fl=$((fl+1)); else fn=$((fn+1)); fi
      done
      # hand free slots to everyone ahead (conservatively: any-slot waiters take
      # newer slots first, then the rest); try only if one is left over for us
      x=$(( ahead_a < fn ? ahead_a : fn )); fn=$((fn - x)); fl=$((fl - (ahead_a - x)))
      [ $fl -lt 0 ] && fl=0
      fl=$((fl - ahead_l)); [ $fl -lt 0 ] && fl=0
      [ $((fl + fn)) -gt 0 ] || try=0
    fi
  fi
  if [ $try = 1 ]; then
    for lock in "${slots[@]}"; do
      [ "$avail" -ge "$min_gb" ] || break
      exec 9>>"$lock"
      if flock -n 9; then
        [ -n "$ticket" ] && rm -f -- "$ticket"
        _run_job "$lock" "$@"
        exit $?
      fi
      exec 9>&-
    done
  fi
  # retry; slots need >= $min_gb GB available
  sleep "$poll"
done
