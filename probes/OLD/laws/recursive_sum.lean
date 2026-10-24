def sum_to (n : Int) : Int := if n ≤ 0 then 0 else n + sum_to (n - 1)
termination_by n.toNat
decreasing_by omega

theorem sum_to_law (n : Int) : n ≥ 0 → sum_to n * 2 = n * (n + 1) := by
  fun_induction sum_to n <;> first | (grind [sum_to]; done) | (simp_all; done) | (omega; done)

def fact (n : Int) : Int := if n ≤ 0 then 1 else n * fact (n - 1)
termination_by n.toNat
decreasing_by omega

theorem fact_law (n : Int) : fact n ≥ 1 := by
  fun_induction fact n <;> first | (grind [fact]; done) | (simp_all; done) | (omega; done)

theorem fact_law2 (n : Int) : fact n ≥ 1 := by
  fun_induction fact n
  all_goals first | (grind [fact]; done) | (rw [fact]; simp_all; done) | (rw [fact]; simp_all; grind; done)
