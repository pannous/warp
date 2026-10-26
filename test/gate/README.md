# Gate probes (tier 1 of scripts/gate.sh)

One tiny program per probe, `<name>.warp`, and what it prints, `<name>.expected` (stdout, exact). Every probe runs in
both lanes, `warp pure` and `warp self`, in seconds once the tree's compilers are built. The expected output is the
right value, never today's error: a probe red in one lane is a real regression to fix, not to re-pin.
Add a probe by adding the two files. The runner sets WARP_GATE_PROBE=gate-ok for probes that read the environment.
