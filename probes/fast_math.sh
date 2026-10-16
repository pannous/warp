#!/bin/sh
# The largest errors of `warp fast` (lib/fast_math.warp) against libm, and the time of 10^6 calls of each
cd "$(dirname "$0")/.." || exit 1
export WARP_NO_WINDOW=1 WARP_CRASH_CARDS=0 WARP_HINTS=0
WARP=${WARP:-scratch/warp}
mkdir -p data/fast_math
$WARP probes/fast_math.warp > data/fast_math/libm.txt
$WARP fast probes/fast_math.warp > data/fast_math/fast.txt
python3 -I - data/fast_math/libm.txt data/fast_math/fast.txt <<'PY'
import sys
names = ["sin", "cos", "exp (relative)", "pow (relative)", "atan2"]
def rows(path):
    return [[float(v) for v in line.strip().strip("[]").split()] for line in open(path) if line.startswith("[")]
worst = [0.0] * len(names)
for exact, fast in zip(rows(sys.argv[1]), rows(sys.argv[2])):
    for k, (e, f) in enumerate(zip(exact, fast)):
        error = abs(e - f) / (abs(e) if "relative" in names[k] and e else 1)
        worst[k] = max(worst[k], error)
for name, error in zip(names, worst):
    print(f"{name:16} {error:.2e}")
PY
for word in 'sin(x)' 'exp(x / 2000000)' 'pow(1.5, x / 1000000)' 'atan2(x, 3.0)'; do
	code="sum = 0.0; for i in 1 to 1000000 { x = i * 1.0; sum = sum + $word }; sum"
	for flag in "" fast; do
		start=$(python3 -c 'import time; print(time.time())')
		$WARP $flag "$code" > /dev/null
		echo "$word ${flag:-libm}: $(python3 -c "import time; print(round(time.time() - $start, 2))") s"
	done
done
