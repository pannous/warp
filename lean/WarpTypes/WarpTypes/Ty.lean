/-! Types of the core calculus W0 (notes/type_theory.md) and their order.

`bool ≤ int ≤ number` is the chain of numbers (true/false act as 1/0), `never` is the bottom (the type of `error`
and of the elements of `[]`), `any` the top (a Node), and lists are covariant: warp lists are values. -/

namespace Warp

inductive Ty where
  | never | bool | int | number | text | unit
  | list (element : Ty)
  | any
  deriving DecidableEq, Repr

namespace Ty

/-- subtyping, decided structurally -/
def sub : Ty → Ty → Bool
  | never, _ => true
  | _, any => true
  | list a, list b => sub a b
  | bool, bool | bool, int | bool, number | int, int | int, number | number, number => true
  | text, text | unit, unit => true
  | _, _ => false

/-- the type as warp writes it -/
def name : Ty → String
  | never => "never"
  | bool => "bool"
  | int => "int"
  | number => "number"
  | text => "text"
  | unit => "unit"
  | list t => s!"list of {t.name}"
  | any => "any"

/-- least upper bound -/
def join : Ty → Ty → Ty
  | never, b => b
  | a, never => a
  | list a, list b => list (join a b)
  | a, b => if sub a b then b else if sub b a then a else any

/-- the element type of a list type; `never` (an error in list position) has elements of type `never` -/
def element : Ty → Option Ty
  | never => some never
  | list a => some a
  | _ => none

/-- the result of `+`: an int when both operands are, otherwise a number -/
def arith (a b : Ty) : Ty := if sub a int && sub b int then int else number

@[simp] theorem sub_never (t : Ty) : sub never t = true := by cases t <;> rfl

@[simp] theorem sub_any (t : Ty) : sub t any = true := by cases t <;> rfl

@[simp] theorem sub_list (a b : Ty) : sub (list a) (list b) = sub a b := rfl

theorem sub_refl : ∀ t : Ty, sub t t = true
  | list a => by simp [sub_refl a]
  | never | bool | int | number | text | unit | any => rfl

theorem sub_to_never : ∀ {t : Ty}, sub t never = true → t = never := by
  intro t h; cases t <;> simp_all [sub]

theorem sub_from_any : ∀ {t : Ty}, sub any t = true → t = any := by
  intro t h; cases t <;> simp_all [sub]

theorem sub_to_list : ∀ {t a : Ty}, sub t (list a) = true → t = never ∨ ∃ b, t = list b ∧ sub b a = true := by
  intro t a h; cases t <;> simp_all [sub]

theorem sub_from_list : ∀ {t a : Ty}, sub (list a) t = true → t = any ∨ ∃ b, t = list b ∧ sub a b = true := by
  intro t a h; cases t <;> simp_all [sub]

theorem sub_trans : ∀ {a b c : Ty}, sub a b = true → sub b c = true → sub a c = true := by
  intro a
  induction a with
  | never => intros; simp
  | list x ih =>
    intro b c hab hbc
    rcases sub_from_list hab with rfl | ⟨y, rfl, hxy⟩
    · rw [sub_from_any hbc]; simp
    · rcases sub_from_list hbc with rfl | ⟨z, rfl, hyz⟩
      · simp
      · simpa using ih hxy hyz
  | any => intro b c hab hbc; rw [sub_from_any hab] at hbc; exact hbc
  | _ =>
    intro b c hab hbc
    cases b <;> cases c <;> simp_all [sub]

theorem sub_antisymm : ∀ {a b : Ty}, sub a b = true → sub b a = true → a = b := by
  intro a
  induction a with
  | list x ih =>
    intro b hab hba
    rcases sub_from_list hab with rfl | ⟨y, rfl, hxy⟩
    · simp [sub] at hba
    · simp at hba; rw [ih hxy hba]
  | _ => intro b hab hba; cases b <;> simp_all [sub]

theorem join_upper_left : ∀ a b : Ty, sub a (join a b) = true := by
  intro a
  induction a with
  | list x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | never => intro b; simp
  | _ => intro b; cases b <;> simp [join, sub] <;> (try split) <;> simp_all

theorem join_upper_right : ∀ a b : Ty, sub b (join a b) = true := by
  intro a
  induction a with
  | list x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | never => intro b; simp [join, sub_refl]
  | _ => intro b; cases b <;> simp [join, sub, sub_refl] <;> (try split) <;> simp_all

theorem join_least : ∀ {a b c : Ty}, sub a c = true → sub b c = true → sub (join a b) c = true := by
  intro a
  induction a with
  | list x ih =>
    intro b c hac hbc
    cases b with
    | never => simpa [join] using hac
    | list y =>
      rcases sub_from_list hac with rfl | ⟨z, rfl, hxz⟩
      · simp
      · simp at hbc; simpa [join] using ih hxz hbc
    | any => rw [sub_from_any hbc]; simp
    | _ => rcases sub_from_list hac with rfl | ⟨z, rfl, _⟩ <;> simp_all [join, sub]
  | never => intro b c _ hbc; simpa [join] using hbc
  | any => intro b c hac _; rw [sub_from_any hac]; simp
  | _ =>
    intro b c hac hbc
    cases b <;> cases c <;> simp_all [join, sub] <;> (try split) <;> simp_all

theorem join_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (join a' b') (join a b) = true :=
  join_least (sub_trans ha (join_upper_left a b)) (sub_trans hb (join_upper_right a b))

theorem element_mono {l l' e : Ty} (hl : sub l' l = true) (he : element l = some e) :
    ∃ e', element l' = some e' ∧ sub e' e = true := by
  cases l <;> simp [element] at he
  · subst he; rw [sub_to_never hl]; exact ⟨never, rfl, by simp⟩
  · subst he
    rcases sub_to_list hl with rfl | ⟨b, rfl, hb⟩
    · exact ⟨never, rfl, by simp⟩
    · exact ⟨b, rfl, hb⟩

theorem arith_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (arith a' b') (arith a b) = true := by
  unfold arith
  by_cases h : (sub a int && sub b int) = true
  · simp at h
    simp [sub_trans ha h.1, sub_trans hb h.2, h.1, h.2, sub_refl]
  · simp only [h, Bool.false_eq_true, ↓reduceIte]; split <;> simp [sub]

theorem arith_sub_number (a b : Ty) : sub (arith a b) number = true := by
  unfold arith; split <;> simp [sub]

/-- what `+` takes: numbers, and texts (`"a" + 1` is "a1") -/
def addable (t : Ty) : Bool := sub t number || sub t text

/-- the result of `+`: a text when a side is a text, a number type otherwise; `never` when a side raises -/
def plus (a b : Ty) : Ty :=
  if a == never || b == never then never else if a == text || b == text then text else arith a b

theorem addable_mono {a a' : Ty} (h : sub a' a = true) (ha : addable a = true) : addable a' = true := by
  unfold addable at *
  simp only [Bool.or_eq_true] at *
  rcases ha with ha | ha
  · exact .inl (sub_trans h ha)
  · exact .inr (sub_trans h ha)

theorem plus_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) (aa : addable a = true)
    (ab : addable b = true) : sub (plus a' b') (plus a b) = true := by
  have hbound : ∀ {t}, addable t = true → t = never ∨ t = bool ∨ t = int ∨ t = number ∨ t = text := by
    intro t ht; cases t <;> simp_all [addable, sub]
  rcases hbound aa with rfl | rfl | rfl | rfl | rfl <;>
  rcases hbound ab with rfl | rfl | rfl | rfl | rfl <;>
  rcases hbound (addable_mono ha aa) with rfl | rfl | rfl | rfl | rfl <;>
  rcases hbound (addable_mono hb ab) with rfl | rfl | rfl | rfl | rfl <;>
  simp_all [plus, arith, sub]

end Ty
end Warp
