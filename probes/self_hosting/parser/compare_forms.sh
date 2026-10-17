#!/bin/sh
# Compares parser.warp with the Rust parser on each line of probes/self_hosting/parser/forms.warp
cd "$(dirname "$0")/../../.."  # the repo root
mkdir -p scratch
warp=scratch/warp  # scripts/own-warp.sh builds it; else the warp on the PATH
[ -x "$warp" ] || warp=warp
tests/queue.sh -- rust_forms --ignored --nocapture 2>&1 | sed -n '/running 1 test/,/^test /p' | grep '^  [^ ]' | sed 's/^  //' > scratch/forms_rust.txt
printf 'use "lib/compiler/parser.warp"\n' > scratch/forms_run.warp
while IFS= read -r line; do esc=$(printf '%s' "$line" | sed 's/\\/\\\\/g; s/"/\\"/g'); printf 'print tree("%s")\n' "$esc" >> scratch/forms_run.warp; done < probes/self_hosting/parser/forms.warp
WARP_NO_WINDOW=1 WARP_HINTS=0 "$warp" scratch/forms_run.warp > scratch/forms_out.txt 2>&1
python3 - <<'PY'
import re
r=open('scratch/forms_rust.txt').read().split('\n')
w=open('scratch/forms_out.txt').read().split('\n')
f=open('probes/self_hosting/parser/forms.warp').read().split('\n')
norm=lambda t: re.sub(r"'([^'\\]|\\.)'", lambda m: '"'+m.group(1)+'"', t)
bad=0
for i,line in enumerate(f):
    if not line: continue
    a=norm(r[i]) if i<len(r) else '?'; b=norm(w[i]) if i<len(w) else '?'
    if a!=b: bad+=1; print(line,'\n  rust:',a,'\n  warp:',b)
print(bad,'of',len([l for l in f if l]),'forms differ')
PY
