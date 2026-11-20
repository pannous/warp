#!/usr/bin/env bash
# a Meta match arm matches no node, as a position rides with its node unseen: it warns, and the next arm takes the node
# (card match-arm-skipped). Run it with bash test/node/test_unseen_case_arm.sh; a tree's warp: WARP=<tree>/data/gate/warp
source "$(dirname "$0")/../native/checks.sh"

check "a Meta arm warns" "$(warning_of 'n = parse("a b c"); match n { Meta(node, data): "meta"; List(items, bracket, separator): "list ${#items}"; _: "other" }')" 'warning: a Meta arm matches no node: a position rides with its node unseen; fix: match the node itself'
[ $failures = 0 ]
