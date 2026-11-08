# Sample status

The live status is what the sweeps write, one line a sample, never by hand:
- test/samples.txt: scripts/samples.sh, every samples/**/*.warp natively (WARP_NO_WINDOW=1): ok, SRV serves, WIN needs
  a window, EXP fails on purpose, FAIL, TIME
- test/samples-web.txt: scripts/samples-web.sh, the same samples in the headless playground page against the native
  output: ok, drew, DIFF, FAIL, EXP, ran

The nightly (scripts/integrate/nightly.sh) runs both and commits them; a red sample becomes a card (the samples focus,
notes/agents/common.md). The topic sample-gaps that grouped the reds of the first sweeps (2026-10-26/27, 80/224 run)
is closed: its causes are fixed or carded on their own.

Open on 2026-11-08 (nightly at 16d63e4bed, rechecked on main 48ebaaca4d):
- sound/music.warp, laws/kitchensink.warp: red in the page (warp-splatting)
- servers/hosting.warp, servers/route_headers.warp: FAIL in the nightly (an out-of-bounds trap, exit 137), both serve
  on main (SRV): watch the next nightly
