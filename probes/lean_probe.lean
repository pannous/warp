def square (x : Int) : Int := x * x
def add (a b : Int) : Int := a + b
theorem square_law_1 (x : Int) : square (-x) = square x := by
  try simp only [square]
  first | rfl | omega | ac_rfl | grind | simp
theorem add_law_1 (a b : Int) : add a b = add b a := by
  try simp only [add]
  first | rfl | omega | ac_rfl | grind | simp
theorem sq_nonneg' (x : Int) : square x >= 0 := by
  try simp only [square]
  first | rfl | omega | ac_rfl | grind | simp
theorem bad (x : Int) : square x = x := by
  try simp only [square]
  first | rfl | omega | ac_rfl | grind | simp
