# features-docs-examples handover

Wiki commits `ec87d5b` and `9c51a9f` are published on warp.wiki master. They document repeating decimals, numerator and denominator, inferred list types, union first parameters, text cuts, range arms, callable instances and sound buses. Twelve earlier example expressions and the callable example were evaluated directly; the linked `/Users/me/dev/bin/warp` returned `[3, 12]` after the 8ec528d97 install. Both `room(signal)` and `signal |> room` rendered 2.85 seconds through a delay bus, rounding to 29 tenths. Direct expression evaluation checks code and values, not the `is!` harness. The user forbids `warp test` until `warp-test-timeout` is installed.

The wiki is a separate repository: `/Users/me/dev/angles/warp/wiki`, remote `https://github.com/pannous/warp.wiki.git`; the isolated wiki worktree is `/Volumes/Worktrees/features-docs-wiki`, branch `features-docs-examples`. The Warp task worktree is `/Volumes/Worktrees/features-docs-examples`.
