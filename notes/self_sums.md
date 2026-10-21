# `out = out + x` in place (card out-list)

`out += x` grows a list in place, seen by every holder (P200b); `out = out + x` builds a new list, so a loop of it
copied the whole list each step (quadratic). src/lowering/collections/self_sums.rs, the last lowering pass, rewrites
`out = out + x` as `out += x` for a function's local that cannot have another holder:
- not a parameter, not a name main assigns (a global the function would write);
- given only `[]`, list literals (not holding itself) and its own sums `out + e`;
- read only where no reference is kept: `out#i`, `out.m(…)`, an operand of arithmetic or a comparison, `#out`,
  count/size/sum/min/max/join/print; the bare name only as the body's last value.
Anything else (`b = out`, `[out]`, `f(out)`, a lowered `for v in out`, a lambda mentioning it, `return out` before the
end) keeps the copy. Numbers and texts are never rewritten (their start is no list).
