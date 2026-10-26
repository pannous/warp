#!/bin/bash
# Opens files in Sublime and records how long it took. Only when an open is slow does it take a single
# `sample` of sublime_text while the stall is still happening. Normal opens cost nothing extra.
# (The old always-on heartbeat watchdog made stalls of its own, see reference_sublime_git_stall memory.)

SLOW_SECONDS=0
SAMPLE_SECONDS=3
MAX_SAMPLES=20
LOG_DIR="$HOME/Library/Logs/subl-timed"
LOG_FILE="$LOG_DIR/opens.log"

mkdir -p "$LOG_DIR"
now() { perl -MTime::HiRes=time -e 'printf "%.2f", time'; }
elapsed_since() { echo "$(now) - $1" | bc; }
is_slow() { [ "$(echo "$(elapsed_since "$1") > $SLOW_SECONDS" | bc)" = 1 ]; }
sample_count() { ls "$LOG_DIR"/sample_*.txt 2>/dev/null | wc -l; }

sample_sublime_once() {
  local label=$1 pid
  pid=$(pgrep -x sublime_text | head -1)
  [ -z "$pid" ] && return
  if [ "$(sample_count)" -ge "$MAX_SAMPLES" ]; then
    echo "$(date '+%F %T') slow, sample skipped: $MAX_SAMPLES samples already in $LOG_DIR" >> "$LOG_FILE"
    return
  fi
  local target="$LOG_DIR/sample_$(date +%Y%m%d_%H%M%S)_$label.txt"
  /usr/bin/sample "$pid" "$SAMPLE_SECONDS" -file "$target" >/dev/null 2>&1 &
  echo "$(date '+%F %T') slow, sampling pid $pid -> $target" >> "$LOG_FILE"
}

timed_step() {
  local label=$1; shift
  local start sampled=""
  start=$(now)
  "$@" &
  local step_pid=$!
  while kill -0 "$step_pid" 2>/dev/null; do
    if [ -z "$sampled" ] && is_slow "$start"; then
      sample_sublime_once "$label"
      sampled=1
    fi
    sleep 0.1
  done
  wait "$step_pid"
  local status=$?
  local running="warm"
  [ -n "$cold" ] && running="cold"
  echo "$(date '+%F %T') $label $(elapsed_since "$start")s $running ${*: -1}" >> "$LOG_FILE"
  return $status
}

cold=""
pgrep -x sublime_text >/dev/null || cold=1
timed_step open /usr/local/bin/sublime "$@"
timed_step focus /usr/local/bin/focus.scpt
