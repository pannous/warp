/-! Types of the core calculus W0 (notes/type_theory.md) and their order.

`bool ≤ int ≤ number` is the chain of numbers (true/false act as 1/0), `never` is the bottom (the type of `error`
and of the elements of `[]`), `any` the top (a Node), and lists are covariant: warp lists are values. A function
value (a lambda, `x => x*2`) takes anything (its parameter is unannotated) and is covariant in its result.
A class type is its chain of ancestors, root first (`cls ["Shape", "Circle"]`): a subclass extends the chain, so
subtyping is the prefix order and needs no class table. A variant of a sum type is a class extending the sum
(P178, P179).
A fixed-width int type (`int16`, `byte`, D16) is the range of ints it holds, `ranged lo hi`: below int, and one range
below another that contains it.
A quantity (`2 m`, `3 km/h`, src/units/static_units.rs) has the type of its dimensions, `quantity [("Length", 1),
("Time", -1)]`: a type of its own, beside the numbers, since `1 m + 1` is a DimensionError. -/

/-- the powers of a quantity's base dimensions, sorted by name, none zero (`Dims.times` keeps them so) -/
abbrev Dims := List (String × Int)

namespace Warp

inductive Ty where
  | never | bool | int | number | text | unit
  /-- a one-character text (`"a"` parses as one): a text wherever a text is taken -/
  | codepoint
  | list (element : Ty)
  | fn (result : Ty)
  | cls (path : List String)
  | ranged (low high : Int)
  | quantity (dims : Dims)
  | any
  deriving DecidableEq, Repr

/-- d with the power p of the base dimension n multiplied in -/
def Dims.insert (n : String) (p : Int) : Dims → Dims
  | [] => if p = 0 then [] else [(n, p)]
  | (m, q) :: r =>
    if n = m then (if p + q = 0 then r else (m, p + q) :: r)
    else if n < m then (if p = 0 then (m, q) :: r else (n, p) :: (m, q) :: r)
    else (m, q) :: Dims.insert n p r

/-- the dimensions of a product: `m * m` is `m²`, `m/s * s` is `m` -/
def Dims.times (d e : Dims) : Dims := e.foldl (fun product (n, p) => Dims.insert n p product) d

/-- the dimensions of `1 / q` -/
def Dims.inverse (d : Dims) : Dims := d.map fun (n, p) => (n, -p)

namespace Ty

/-- subtyping, decided structurally -/
def sub : Ty → Ty → Bool
  | never, _ => true
  | _, any => true
  | list a, list b => sub a b
  | fn a, fn b => sub a b
  | cls p, cls q => decide (q <+: p)
  | ranged a b, ranged c d => decide (c ≤ a ∧ b ≤ d)
  | ranged _ _, int | ranged _ _, number => true
  | quantity d, quantity e => d == e
  | bool, bool | bool, int | bool, number | int, int | int, number | number, number => true
  | text, text | codepoint, codepoint | codepoint, text | unit, unit => true
  | _, _ => false

/-- warp's fixed-width int types (fixed_width.rs FIXED_WIDTHS) and their ranges -/
def FIXED_WIDTHS : List (String × Int × Int) :=
  [("byte", 0, 255), ("int8", -128, 127), ("int16", -32768, 32767), ("int32", -2147483648, 2147483647),
   ("int64", -9223372036854775808, 9223372036854775807), ("uint8", 0, 255), ("uint16", 0, 65535),
   ("uint32", 0, 4294967295), ("uint64", 0, 18446744073709551615)]

/-- the type as warp writes it -/
def name : Ty → String
  | never => "never"
  | bool => "bool"
  | int => "int"
  | number => "number"
  | text => "text"
  | codepoint => "codepoint"
  | unit => "empty"
  | list t => s!"list of {t.name}"
  | fn t => s!"function to {t.name}"
  | cls p => p.getLastD "object"
  | quantity d => s!"quantity {d}"
  | ranged lo hi => ((FIXED_WIDTHS.find? fun (_, l, h) => l == lo && h == hi).map (·.1)).getD s!"int {lo}…{hi}"
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
  | ranged a b, ranged c d => ranged (min a c) (max b d)
  | bool, ranged _ _ | ranged _ _, bool => int
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
  | ranged a b => by simp [sub]
  | quantity d => by simp [sub]
  | never | bool | int | number | text | codepoint | unit | any => rfl

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
    cases b <;> cases c <;> simp_all [sub] <;> omega

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
  | _ => intro b hab hba; cases b <;> simp_all [sub] <;> omega

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
  | quantity d => intro b; cases b <;> simp [join, sub] <;> (repeat' split) <;> (try simp_all [sub, eq_comm])
  | _ => intro b; cases b <;> simp [join, sub] <;> (try split) <;> (try simp_all) <;> omega

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
  | quantity d => intro b; cases b <;> simp [join, sub] <;> (repeat' split) <;> (try simp_all [sub, eq_comm])
  | _ => intro b; cases b <;> simp [join, sub, sub_refl] <;> (try split) <;> (try simp_all) <;> omega

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
    cases b <;> cases c <;> simp_all [join, sub] <;> (try split) <;> (try simp_all) <;> omega

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
value may go where a cast checks it, as an int may go to a fixed width whose range a cast checks -/
def consub : Ty → Ty → Bool
  | any, _ => true
  | int, ranged _ _ | ranged _ _, ranged _ _ => true
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

def isQuantity : Ty → Bool
  | quantity _ => true
  | _ => false

/-- the dimensions of a quantity type, none (`[]`) of a number type; none of anything else -/
def dimsOf : Ty → Option Dims
  | quantity d => some d
  | t => if sub t number then some [] else none

/-- the type of a value of dimensions d: a number when they cancel (`6 m / 3 m` is 2) -/
def ofDims (d : Dims) : Ty := if d = [] then number else quantity d

/-- a quantity type has no subtypes but itself and `never`, and no supertypes but itself and `any` -/
theorem quantity_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : a ≠ any) :
    isQuantity a' = isQuantity a := by
  cases a' <;> cases a <;> simp_all [sub, isQuantity]

theorem quantity_eq {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (q : isQuantity a = true) : a' = a := by
  cases a' <;> cases a <;> simp_all [sub, isQuantity]

theorem dimsOf_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : a ≠ any) : dimsOf a' = dimsOf a := by
  cases a' <;> cases a <;> simp_all [sub, dimsOf]

/-- `+` and `-` of quantities: one dimension on both sides, else a DimensionError -/
def sameQuantity (a b : Ty) : Ty := if a = b then a else never

theorem sameQuantity_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) (na : a' ≠ never)
    (nb : b' ≠ never) (ya : a ≠ any) (yb : b ≠ any) (q : (isQuantity a || isQuantity b) = true) :
    sub (sameQuantity a' b') (sameQuantity a b) = true := by
  unfold sameQuantity
  by_cases e : a = b
  · subst e
    have qa : isQuantity a = true := by simpa using q
    rw [quantity_eq ha na qa, quantity_eq hb nb qa]; simp [sub_refl]
  · rw [if_neg e]
    split
    · rename_i e'
      have qa := quantity_up ha na ya
      have qb := quantity_up hb nb yb
      subst e'
      rw [qa] at qb
      have q' : isQuantity a = true ∧ isQuantity b = true := by cases h : isQuantity a <;> simp_all
      exact absurd ((quantity_eq ha na q'.1).symm.trans (quantity_eq hb nb q'.2)) e
    · exact sub_refl _

/-- a text or a codepoint (a one-character text) -/
def isText : Ty → Bool
  | text | codepoint => true
  | _ => false

/-- the type of a text literal: one character is a codepoint, as warp's parser reads `"a"` -/
def textTy (s : String) : Ty := if s.length = 1 then codepoint else text

@[simp] theorem textTy_sub_text (s : String) : sub (textTy s) text = true := by
  unfold textTy; split <;> rfl

@[simp] theorem textTy_isText (s : String) : isText (textTy s) = true := by
  unfold textTy; split <;> rfl

@[simp] theorem textTy_ne_never (s : String) : textTy s ≠ never := by
  unfold textTy; split <;> simp

@[simp] theorem textTy_ne_any (s : String) : textTy s ≠ any := by
  unfold textTy; split <;> simp

/-- between `never` and `any`, widening keeps a text a text and a non-text a non-text -/
theorem isText_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : a ≠ any) : isText a' = isText a := by
  cases a' <;> cases a <;> simp_all [sub, isText]

/-- between `never` and `any`, widening keeps a number a number and a non-number a non-number -/
theorem number_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : a ≠ any) :
    sub a' number = sub a number := by
  cases a' <;> cases a <;> simp_all [sub]

/-- the result of `+`: `never` when a side raises, dynamic when a side is, two lists' concatenation a list of their
elements' join, a text when a side is a text, a number type otherwise -/
def plus (a b : Ty) : Ty :=
  if a = never ∨ b = never then never else if a = any ∨ b = any then any
  else if (isQuantity a || isQuantity b) = true then sameQuantity a b
  else if (isListTy a && isListTy b) = true then list (join ((element a).getD any) ((element b).getD any))
  else if (isText a || isText b) = true then text else arith a b

/-- the result of `-` and `*`: dynamic when a side is -/
def arithTy (a b : Ty) : Ty := if a = any ∨ b = any then any else arith a b

theorem sub_to_text : ∀ {t : Ty}, sub t text = true → t = never ∨ t = text ∨ t = codepoint := by
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
  else if (isText a && sub b number || sub a number && isText b) = true then text
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
  rw [isText_up ha n'.1 y.1, isText_up hb n'.2 y.2, number_up ha n'.1 y.1, number_up hb n'.2 y.2]
  split
  · exact sub_refl _
  · exact arith_mono ha hb

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
  have na : a' ≠ never := fun h => n' (.inl h)
  have nb : b' ≠ never := fun h => n' (.inr h)
  have ya : a ≠ any := fun h => y (.inl h)
  have yb : b ≠ any := fun h => y (.inr h)
  rw [quantity_up ha na ya, quantity_up hb nb yb]
  by_cases q : (isQuantity a || isQuantity b) = true
  · rw [ite_eq_left q, ite_eq_left q]; exact sameQuantity_mono ha hb na nb ya yb q
  rw [ite_eq_right q, ite_eq_right q]
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
  rw [if_neg l, if_neg l', isText_up ha na ya, isText_up hb nb yb]
  split
  · exact sub_refl _
  · exact arith_mono ha hb

/-- the element type `#` gives: a list's elements', a text's codepoints, an error's `never`, anything else's `any`
(checked when it runs) -/
def elementTy (t : Ty) : Ty := if isText t = true then codepoint else (element t).getD any

/-- the element type `++` takes: a list's elements', anything else's `any` (checked when it runs) -/
def listElem (t : Ty) : Ty := (element t).getD any

@[simp] theorem elementTy_textTy (s : String) : elementTy (textTy s) = codepoint := by
  simp [elementTy]

theorem elementTy_of_element {t e : Ty} (h : element t = some e) : elementTy t = e := by
  cases t <;> simp [element] at h <;> subst h <;> simp [elementTy, element, isText]

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
  | never => rw [sub_to_never h]; simp [elementTy, element, isText]
  | list a =>
    rcases sub_to_list h with rfl | ⟨b, rfl, hb⟩
    · simp [elementTy, element, isText]
    · simp [elementTy, element, isText, hb]
  | text | codepoint => cases l' <;> simp_all [sub, elementTy, element, isText]
  | _ => simp [elementTy, element, isText]

end Ty
end Warp
