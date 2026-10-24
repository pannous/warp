#!/bin/bash
# tests/queue.sh kills a run whose memory passes WARP_TEST_MEMORY_CAP_GB (on 2026-10-09 one test grew to 154 GB and
# took the whole Mac and every agent session down). Here a child allocates 3 GB against a 1 GB cap.
# Usage: probes/queue_memory_cap.sh  (exit 0 when the run was killed with the cap message)
REPO="$(cd "$(dirname "$0")/.." && pwd)"
ALLOCATED_GB=3
CAP_GB=1

hog="$REPO/scratch/memory_hog.sh"
mkdir -p "$REPO/scratch"
cat > "$hog" <<EOF
#!/bin/bash
exec python3 -c "
import time
blocks = []
for _ in range($ALLOCATED_GB * 8):
    blocks.append(bytearray(128 * 2**20))
    time.sleep(0.2)
print('hog survived')
"
EOF
chmod +x "$hog"

output=$(WARP_TEST_MEMORY_CAP_GB=$CAP_GB "$REPO/tests/queue.sh" "$hog" 2>&1)
status=$?
echo "$output"
echo "exit $status"
if echo "$output" | grep -q "hog survived"; then echo "FAIL: the hog was not killed"; exit 1; fi
if ! echo "$output" | grep -q "memory cap"; then echo "FAIL: no memory cap message"; exit 1; fi
echo "OK: killed at the cap"
