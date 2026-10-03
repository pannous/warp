import Std.Tactic.BVDecide
def square (x : BitVec 64) : BitVec 64 := (x * x)
def add (a b : BitVec 64) : BitVec 64 := (a + b)
def double (x : BitVec 64) : BitVec 64 := (x + x)
theorem t1 (x : BitVec 64) : square (-x) = square x := by
  try simp only [square]
  all_goals first | (grind; done) | (bv_decide; done)
theorem t2 (a b c : BitVec 64) : add a (add b c) = add (add a b) c := by
  try simp only [add]
  all_goals first | (grind; done) | (ac_rfl; done)
theorem t3 (x : BitVec 64) : double x = (2 : BitVec 64) * x := by
  try simp only [double]
  all_goals first | (grind; done) | (bv_decide; done)
theorem t4 (x : BitVec 64) : BitVec.sle 0 (square x) := by
  try simp only [square]
  all_goals first | (grind; done) | (bv_decide; done)
theorem t5 (x : BitVec 64) : x ^ 2 = x * x := by
  first | (grind; done) | (simp [BitVec.pow_succ]; done)
theorem t6 (x : BitVec 64) : (BitVec.sle x x) = true := by
  first | (grind; done) | (bv_decide; done)
theorem t7 (x : BitVec 64) : BitVec.slt x 5 → BitVec.slt x 6 := by
  first | (grind; done) | (bv_decide; done)
theorem t8 (x : BitVec 64) : (x * x) - x * x = 0 := by
  first | (grind; done) | (bv_decide; done)
