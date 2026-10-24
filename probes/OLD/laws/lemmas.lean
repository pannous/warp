theorem warp_law_1 (x y : Int) : x ≥ 1 → y ≥ 1 → x * y ≥ 1 := by
  first | (grind; done) | (intro hx hy; have := Int.mul_le_mul hx hy (by omega) (by omega); simpa using this)

def fact (n : Int) : Int := if n ≤ 0 then 1 else n * fact (n - 1)
termination_by n.toNat
decreasing_by all_goals omega

theorem a (n : Int) : fact n ≥ 1 := by
  fun_induction fact n <;> first | (grind [warp_law_1]; done) | omega
theorem b (n : Int) : fact n ≥ 1 := by
  fun_induction fact n <;> first | (apply warp_law_1 <;> omega; done) | omega
theorem c (n : Int) : fact n ≥ 1 := by
  fun_induction fact n <;> first | (simp_all [warp_law_1]; done) | omega
