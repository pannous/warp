/-! Types of the core calculus W0 (notes/type_theory.md) and their order.

`bool ≤ int ≤ number` is the chain of numbers (true/false act as 1/0), `never` is the bottom (the type of `error`
and of the elements of `[]`), `any` the top (a Node), and lists are covariant: warp lists are values. A function
value (a lambda, `x => x*2`) takes anything (its parameter is unannotated) and is covariant in its result.
A class type is its chain of ancestors, root first (`cls ["Shape", "Circle"]`): a subclass extends the chain, so
subtyping is the prefix order and needs no class table. A variant of a sum type is a class extending the sum
(P178, P179). -/

namespace Warp

inductive Ty where
  | never | bool | int | number | text | unit
  | list (element : Ty)
  | fn (result : Ty)
  | cls (path : List String)
  | any
  deriving DecidableEq, Repr

namespace Ty

/-- subtyping, decided structurally -/
def sub : Ty → Ty → Bool
  | never, _ => true
  | _, any => true
  | list a, list b => sub a b
  | fn a, fn b => sub a b
  | cls p, cls q => decide (q <+: p)
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
  | unit => "empty"
  | list t => s!"list of {t.name}"
  | fn t => s!"function to {t.name}"
  | cls p => p.getLastD "object"
  | any => "any"

/-- the longest common prefix: the nearest common ancestor of two class chains -/
def commonPrefix : List String → List String → List String
  | a :: p, b :: q => if a = b then a :: commonPrefix p q else []
  | _, _ => []

theorem commonPrefix_left : ∀ p q : List String, commonPrefix p q <+: p
  | a :: p, b :: q => by
    unfold commonPrefix; split
    · exact (List.prefix_cons_inj a).2 (commonPrefix_left p q)
    · exact List.nil_prefix
  | [], _ | _ :: _, [] => by simp [commonPrefix]

theorem commonPrefix_right : ∀ p q : List String, commonPrefix p q <+: q
  | a :: p, b :: q => by
    unfold commonPrefix; split
    · subst_vars; exact (List.prefix_cons_inj _).2 (commonPrefix_right p q)
    · exact List.nil_prefix
  | [], _ | _ :: _, [] => by simp [commonPrefix]

theorem commonPrefix_greatest : ∀ {r p q : List String}, r <+: p → r <+: q → r <+: commonPrefix p q
  | [], _, _, _, _ => List.nil_prefix
  | c :: r, a :: p, b :: q, hp, hq => by
    rw [List.cons_prefix_cons] at hp hq
    obtain ⟨rfl, hp⟩ := hp; obtain ⟨rfl, hq⟩ := hq
    simp only [commonPrefix, ite_true]
    exact (List.prefix_cons_inj _).2 (commonPrefix_greatest hp hq)
  | _ :: _, [], _, hp, _ => by simp at hp
  | _ :: _, _ :: _, [], _, hq => by simp at hq

/-- least upper bound -/
def join : Ty → Ty → Ty
  | never, b => b
  | a, never => a
  | list a, list b => list (join a b)
  | fn a, fn b => fn (join a b)
  | cls p, cls q => cls (commonPrefix p q)
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

@[simp] theorem sub_fn (a b : Ty) : sub (fn a) (fn b) = sub a b := rfl

theorem sub_refl : ∀ t : Ty, sub t t = true
  | list a => by simp [sub_refl a]
  | fn a => by simp [sub_refl a]
  | cls p => by simp [sub]
  | never | bool | int | number | text | unit | any => rfl

theorem sub_to_never : ∀ {t : Ty}, sub t never = true → t = never := by
  intro t h; cases t <;> simp_all [sub]

theorem sub_from_any : ∀ {t : Ty}, sub any t = true → t = any := by
  intro t h; cases t <;> simp_all [sub]

theorem sub_to_list : ∀ {t a : Ty}, sub t (list a) = true → t = never ∨ ∃ b, t = list b ∧ sub b a = true := by
  intro t a h; cases t <;> simp_all [sub]

theorem sub_from_list : ∀ {t a : Ty}, sub (list a) t = true → t = any ∨ ∃ b, t = list b ∧ sub a b = true := by
  intro t a h; cases t <;> simp_all [sub]

theorem sub_to_fn : ∀ {t a : Ty}, sub t (fn a) = true → t = never ∨ ∃ b, t = fn b ∧ sub b a = true := by
  intro t a h; cases t <;> simp_all [sub]

theorem sub_from_fn : ∀ {t a : Ty}, sub (fn a) t = true → t = any ∨ ∃ b, t = fn b ∧ sub a b = true := by
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
  | fn x ih =>
    intro b c hab hbc
    rcases sub_from_fn hab with rfl | ⟨y, rfl, hxy⟩
    · rw [sub_from_any hbc]; simp
    · rcases sub_from_fn hbc with rfl | ⟨z, rfl, hyz⟩
      · simp
      · simpa using ih hxy hyz
  | any => intro b c hab hbc; rw [sub_from_any hab] at hbc; exact hbc
  | cls p =>
    intro b c hab hbc
    cases b <;> cases c <;> simp_all [sub]
    exact hbc.trans hab
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
  | fn x ih =>
    intro b hab hba
    rcases sub_from_fn hab with rfl | ⟨y, rfl, hxy⟩
    · simp [sub] at hba
    · simp at hba; rw [ih hxy hba]
  | cls p =>
    intro b hab hba
    cases b <;> simp_all [sub]
    exact List.IsPrefix.eq_of_length hba (Nat.le_antisymm hba.length_le hab.length_le)
  | _ => intro b hab hba; cases b <;> simp_all [sub]

theorem join_upper_left : ∀ a b : Ty, sub a (join a b) = true := by
  intro a
  induction a with
  | list x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | fn x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | never => intro b; simp
  | cls p => intro b; cases b <;> simp [join, sub, commonPrefix_left] <;> (try split) <;> simp_all [sub]
  | _ => intro b; cases b <;> simp [join, sub] <;> (try split) <;> simp_all

theorem join_upper_right : ∀ a b : Ty, sub b (join a b) = true := by
  intro a
  induction a with
  | list x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | fn x ih =>
    intro b; cases b <;> simp [join, sub_refl, ih]
    all_goals (split <;> simp_all [sub])
  | never => intro b; simp [join, sub_refl]
  | cls p => intro b; cases b <;> simp [join, sub, commonPrefix_right] <;> (try split) <;> simp_all [sub]
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
  | fn x ih =>
    intro b c hac hbc
    cases b with
    | never => simpa [join] using hac
    | fn y =>
      rcases sub_from_fn hac with rfl | ⟨z, rfl, hxz⟩
      · simp
      · simp at hbc; simpa [join] using ih hxz hbc
    | any => rw [sub_from_any hbc]; simp
    | _ => rcases sub_from_fn hac with rfl | ⟨z, rfl, _⟩ <;> simp_all [join, sub]
  | never => intro b c _ hbc; simpa [join] using hbc
  | any => intro b c hac _; rw [sub_from_any hac]; simp
  | cls p =>
    intro b c hac hbc
    cases b <;> cases c <;> simp_all [join, sub]
    all_goals first | exact commonPrefix_greatest hac hbc | (split <;> simp_all [sub])
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

/-- the result type of calling a value of type t: a function's result, `never` of an error, `any` of anything else
(a dynamic value is checked when it runs; the checker rejects calling a value known not to be a function) -/
def resultTy : Ty → Ty
  | never => never
  | fn r => r
  | _ => any

theorem resultTy_mono {f f' : Ty} (hf : sub f' f = true) : sub (resultTy f') (resultTy f) = true := by
  cases f with
  | never => rw [sub_to_never hf]; simp [resultTy]
  | fn r =>
    rcases sub_to_fn hf with rfl | ⟨b, rfl, hb⟩
    · simp [resultTy]
    · simpa [resultTy] using hb
  | _ => cases f' <;> simp [resultTy]

/-- the type of a value of one of the alternatives ts: their join (`int | float` is `number`, `int | text` is `any`) -/
def joinAll (ts : List Ty) : Ty := ts.foldr join never

theorem sub_joinAll : ∀ {t : Ty} {ts : List Ty}, t ∈ ts → sub t (joinAll ts) = true
  | _, a :: ts, h => by
    simp only [joinAll, List.foldr_cons]
    rcases List.mem_cons.1 h with rfl | h
    · exact join_upper_left _ _
    · exact sub_trans (sub_joinAll h) (join_upper_right _ _)

/-- consistent subtyping (gradual typing): `any` stands for whatever type the value turns out to have, so a dynamic
value may go where a cast checks it -/
def consub : Ty → Ty → Bool
  | any, _ => true
  | list a, list b => consub a b
  | a, b => sub a b

theorem arith_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (arith a' b') (arith a b) = true := by
  unfold arith
  by_cases h : (sub a int && sub b int) = true
  · simp at h
    simp [sub_trans ha h.1, sub_trans hb h.2, h.1, h.2, sub_refl]
  · simp only [h, Bool.false_eq_true, ↓reduceIte]; split <;> simp [sub]

theorem arith_sub_number (a b : Ty) : sub (arith a b) number = true := by
  unfold arith; split <;> simp [sub]

/-- what `+` takes statically: numbers, texts (`"a" + 1` is "a1"), and a dynamic value (`any`, checked when it runs) -/
def addable (t : Ty) : Bool := sub t number || sub t text || t == any

/-- what `-`, `*` and `<` take statically: numbers and a dynamic value -/
def numeric (t : Ty) : Bool := sub t number || t == any

/-- a text, or a value of unknown type (checked when it runs) -/
def textual (t : Ty) : Bool := sub t text || t == any

/-- a list type -/
def isListTy : Ty → Bool
  | list _ => true
  | _ => false

/-- the result of `+`: `never` when a side raises, dynamic when a side is, two lists' concatenation a list of their
elements' join, a text when a side is a text, a number type otherwise -/
def plus (a b : Ty) : Ty :=
  if a = never ∨ b = never then never else if a = any ∨ b = any then any
  else if (isListTy a && isListTy b) = true then list (join ((element a).getD any) ((element b).getD any))
  else if a = text ∨ b = text then text else arith a b

/-- the result of `-` and `*`: dynamic when a side is -/
def arithTy (a b : Ty) : Ty := if a = any ∨ b = any then any else arith a b

theorem sub_to_text : ∀ {t : Ty}, sub t text = true → t = never ∨ t = text := by
  intro t h; cases t <;> simp_all [sub]

theorem sub_from_text : ∀ {t : Ty}, sub text t = true → t = text ∨ t = any := by
  intro t h; cases t <;> simp_all [sub]

theorem arithTy_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (arithTy a' b') (arithTy a b) = true := by
  unfold arithTy
  by_cases y : a = any ∨ b = any
  · rw [ite_eq_left y]; exact sub_any _
  rw [ite_eq_right y]
  have y' : ¬(a' = any ∨ b' = any) := by
    rintro (rfl | rfl)
    · exact y (.inl (sub_from_any ha))
    · exact y (.inr (sub_from_any hb))
  rw [ite_eq_right y']
  exact arith_mono ha hb

/-- the result of `*`: a text times a whole number (either order) repeats the text (P1), numbers multiply -/
def repeatTy (a b : Ty) : Ty :=
  if a = never ∨ b = never then never
  else if a = any ∨ b = any then any
  else if (a = text ∧ sub b number) ∨ (sub a number ∧ b = text) then text
  else arith a b

/-- above a number other than `never`, below `any`: a number -/
theorem sub_number_up {b b' : Ty} (h : sub b' number = true) (hb : sub b' b = true) (ny : b ≠ any) (nn : b' ≠ never) :
    sub b number = true := by
  cases b' <;> cases b <;> simp_all [sub]

theorem repeatTy_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (repeatTy a' b') (repeatTy a b) = true := by
  unfold repeatTy
  by_cases n' : a' = never ∨ b' = never
  · rw [ite_eq_left n']; exact sub_never _
  rw [ite_eq_right n']
  have n : ¬(a = never ∨ b = never) := by
    rintro (rfl | rfl)
    · exact n' (.inl (sub_to_never ha))
    · exact n' (.inr (sub_to_never hb))
  rw [ite_eq_right n]
  by_cases y : a = any ∨ b = any
  · rw [ite_eq_left y]; exact sub_any _
  rw [ite_eq_right y]
  have y' : ¬(a' = any ∨ b' = any) := by
    rintro (rfl | rfl)
    · exact y (.inl (sub_from_any ha))
    · exact y (.inr (sub_from_any hb))
  rw [ite_eq_right y']
  simp only [not_or] at n n' y y'
  -- a text side stays a text side when widened (only `any` is above text), and a number side stays a number
  have widened : (a' = text ∧ sub b' number = true) ∨ (sub a' number = true ∧ b' = text) →
      (a = text ∧ sub b number = true) ∨ (sub a number = true ∧ b = text) := by
    rintro (⟨rfl, hn⟩ | ⟨hn, rfl⟩)
    · exact .inl ⟨(sub_from_text ha).resolve_right y.1, sub_number_up hn hb y.2 n'.2⟩
    · exact .inr ⟨sub_number_up hn ha y.1 n'.1, (sub_from_text hb).resolve_right y.2⟩
  have narrowed : (a = text ∧ sub b number = true) ∨ (sub a number = true ∧ b = text) →
      (a' = text ∧ sub b' number = true) ∨ (sub a' number = true ∧ b' = text) := by
    rintro (⟨rfl, hn⟩ | ⟨hn, rfl⟩)
    · exact .inl ⟨(sub_to_text ha).resolve_left n'.1, sub_trans hb hn⟩
    · exact .inr ⟨sub_trans ha hn, (sub_to_text hb).resolve_left n'.2⟩
  by_cases r' : (a' = text ∧ sub b' number = true) ∨ (sub a' number = true ∧ b' = text)
  · rw [if_pos r', if_pos (widened r')]; exact sub_refl _
  · rw [if_neg r', if_neg (fun r => r' (narrowed r))]; exact arith_mono ha hb

theorem plus_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (plus a' b') (plus a b) = true := by
  unfold plus
  by_cases n' : a' = never ∨ b' = never
  · rw [ite_eq_left n']; exact sub_never _
  rw [ite_eq_right n']
  have n : ¬(a = never ∨ b = never) := by
    rintro (rfl | rfl)
    · exact n' (.inl (sub_to_never ha))
    · exact n' (.inr (sub_to_never hb))
  rw [ite_eq_right n]
  by_cases y : a = any ∨ b = any
  · rw [ite_eq_left y]; exact sub_any _
  rw [ite_eq_right y]
  have y' : ¬(a' = any ∨ b' = any) := by
    rintro (rfl | rfl)
    · exact y (.inl (sub_from_any ha))
    · exact y (.inr (sub_from_any hb))
  rw [ite_eq_right y']
  by_cases l : (isListTy a && isListTy b) = true
  · obtain ⟨x, rfl⟩ : ∃ x, a = list x := by cases a <;> simp [isListTy] at l ⊢
    obtain ⟨z, rfl⟩ : ∃ z, b = list z := by cases b <;> simp [isListTy] at l ⊢
    rcases sub_to_list ha with h | ⟨x', rfl, hx⟩
    · exact absurd (.inl h) n'
    rcases sub_to_list hb with h | ⟨z', rfl, hz⟩
    · exact absurd (.inr h) n'
    simp only [isListTy, Bool.and_self, if_true, element, Option.getD_some, sub_list]
    exact join_mono hx hz
  have l' : ¬(isListTy a' && isListTy b') = true := by
    intro h'
    apply l
    obtain ⟨x', rfl⟩ : ∃ x, a' = list x := by cases a' <;> simp [isListTy] at h' ⊢
    obtain ⟨z', rfl⟩ : ∃ z, b' = list z := by cases b' <;> simp [isListTy] at h' ⊢
    rcases sub_from_list ha with h | ⟨x, rfl, _⟩
    · exact absurd (.inl h) y
    rcases sub_from_list hb with h | ⟨z, rfl, _⟩
    · exact absurd (.inr h) y
    rfl
  rw [if_neg l, if_neg l']
  by_cases t : a = text ∨ b = text
  · rw [ite_eq_left t]
    have t' : a' = text ∨ b' = text := by
      rcases t with rfl | rfl
      · rcases sub_to_text ha with h | h
        · exact absurd (.inl h) n'
        · exact .inl h
      · rcases sub_to_text hb with h | h
        · exact absurd (.inr h) n'
        · exact .inr h
    rw [ite_eq_left t']; exact sub_refl _
  rw [ite_eq_right t]
  have t' : ¬(a' = text ∨ b' = text) := by
    rintro (rfl | rfl)
    · rcases sub_from_text ha with h | h
      · exact t (.inl h)
      · exact y (.inl h)
    · rcases sub_from_text hb with h | h
      · exact t (.inr h)
      · exact y (.inr h)
  rw [ite_eq_right t']
  exact arith_mono ha hb

/-- the element type `#` gives: a list's elements', a text's one-codepoint texts, an error's `never`, anything
else's `any` (checked when it runs) -/
def elementTy (t : Ty) : Ty := if t = text then text else (element t).getD any

/-- the element type `++` takes: a list's elements', anything else's `any` (checked when it runs) -/
def listElem (t : Ty) : Ty := (element t).getD any

theorem elementTy_of_element {t e : Ty} (h : element t = some e) : elementTy t = e := by
  cases t <;> simp_all [elementTy, element]

theorem listElem_of_element {t e : Ty} (h : element t = some e) : listElem t = e := by
  cases t <;> simp_all [listElem, element]

theorem listElem_mono {l l' : Ty} (h : sub l' l = true) : sub (listElem l') (listElem l) = true := by
  cases l with
  | never => rw [sub_to_never h]; simp [listElem, element]
  | list a =>
    rcases sub_to_list h with rfl | ⟨b, rfl, hb⟩
    · simp [listElem, element]
    · simp [listElem, element, hb]
  | _ => simp [listElem, element]

theorem elementTy_mono {l l' : Ty} (h : sub l' l = true) : sub (elementTy l') (elementTy l) = true := by
  cases l with
  | never => rw [sub_to_never h]; simp [elementTy, element]
  | list a =>
    rcases sub_to_list h with rfl | ⟨b, rfl, hb⟩
    · simp [elementTy, element]
    · simp [elementTy, element, hb]
  | text => rcases sub_to_text h with rfl | rfl <;> simp [elementTy, element, sub_refl]
  | _ => simp [elementTy, element]

end Ty
end Warp
