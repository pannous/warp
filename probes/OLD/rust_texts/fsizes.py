# fsizes.py x.cwasm: machine code bytes per function name (from wasmtime objdump addresses)
import sys, subprocess, re
out = subprocess.run(['wasmtime', 'objdump', '--addresses', sys.argv[1]], capture_output=True, text=True).stdout
starts = [(int(m.group(1), 16), m.group(2)) for m in re.finditer(r'^([0-9a-f]{8}) wasm\[0\]::function\[\d+\]::(\S+):$', out, re.M)]
for (start, name), (next_start, _) in zip(starts, starts[1:] + [(starts[-1][0], '')]):
    print(name, next_start - start)
