def sum_to (n : Int) : Int := if n ≤ 0 then 0 else n + sum_to (n - 1)
termination_by n.toNat
decreasing_by all_goals omega
def gcd2 (a b : Int) : Int := if b ≤ 0 then a else if a ≤ 0 then b else if a > b then gcd2 (a - b) b else gcd2 a (b - a)
termination_by a.toNat + b.toNat
decreasing_by all_goals omega
def power (b e : Int) : Int := if e ≤ 0 then 1 else b * power b (e - 1)
termination_by b.toNat + e.toNat
decreasing_by all_goals omega
theorem power_law (b e : Int) : e ≥ 0 → power b (e + 1) = b * power b e := by
  fun_induction power b e <;> first | (grind [power]; done) | (simp_all; done) | (omega; done)
theorem power_law2 (b e : Int) : e ≥ 0 → power b (e + 1) = b * power b e := by
  first | (grind [power]; done) | (intro h; rw [power]; simp; omega)
