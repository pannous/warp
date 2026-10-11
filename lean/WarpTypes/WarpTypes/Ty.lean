/-! Types of the core calculus W0 (notes/type_theory.md) and their order.

`bool ≤ int ≤ exact ≤ number` is the chain of numbers (true/false act as 1/0; an exact number is a rational, as warp's
`0.5` and `7/2` are; a number is a float, as `sqrt(2)` is), `never` is the bottom (the type of `error`
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
  /-- a rational: `0.5`, `7/2`, `1/3` (warp's `rational`, the exact numbers); a float is a `number` -/
  | exact
  /-- a one-character text (`"a"` parses as one): a text wherever a text is taken -/
  | codepoint
  | list (element : Ty)
  | fn (result : Ty)
  | cls (path : List String)
  | ranged (low high : Int)
  | quantity (dims : Dims)
  /-- `int?`: ø or a value of the inner type -/
  | opt (inner : Ty)
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

/-- ø is a value of the type: `empty`, an optional, `any` -/
def admitsUnit : Ty → Bool
  | unit | opt _ | any => true
  | _ => false

def isOptional : Ty → Bool
  | opt _ => true
  | _ => false

/-- the type under its optional marks: `int?` gives `int` -/
@[simp] def base : Ty → Ty
  | opt t => base t
  | t => t

/-- the type of the values other than ø: `int?` gives `int`, `empty` gives `never` -/
@[simp] def strip : Ty → Ty
  | unit => never
  | opt t => strip t
  | t => t

/-- subtyping of the types other than never, ø, optionals, lists and functions, against a type that is no optional -/
@[simp] def scalarSub : Ty → Ty → Bool
  | _, any => true
  | cls p, cls q => decide (q <+: p)
  | ranged a b, ranged c d => decide (c ≤ a ∧ b ≤ d)
  | ranged _ _, int | ranged _ _, exact | ranged _ _, number => true
  | quantity d, quantity e => d == e
  | bool, bool | bool, int | bool, exact | bool, number | int, int | int, exact | int, number => true
  | exact, exact | exact, number | number, number => true
  | text, text | codepoint, codepoint | codepoint, text => true
  | _, _ => false

/-- subtyping, decided structurally: ø fits a type that admits it, an optional `t?` fits where both ø and t fit, any
other type fits an optional when it fits the inner type. A preorder: `any?` and `any` fit each other -/
def sub : Ty → Ty → Bool
  | never, _ => true
  | unit, t => admitsUnit t
  | opt a, t => admitsUnit t && sub a t
  | list a, t => match base t with
    | list b => sub a b
    | any => true
    | _ => false
  | fn a, t => match base t with
    | fn b => sub a b
    | any => true
    | _ => false
  | a, t => scalarSub a (base t)


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
  | exact => "rational"
  | text => "text"
  | codepoint => "codepoint"
  | unit => "empty"
  | list t => s!"list of {t.name}"
  | fn t => s!"function to {t.name}"
  | cls p => p.getLastD "object"
  | quantity d => s!"quantity {d}"
  | opt t => s!"{t.name}?"
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

/-- ø or a value of t (`never` gives just ø; `any` and an optional admit ø already) -/
def optional : Ty → Ty
  | never | unit => unit
  | any => any
  | opt t => opt t
  | t => opt t

/-- t, optional when b admits ø -/
def liftUnit (b t : Ty) : Ty := if admitsUnit b then optional t else t

/-- the least upper bound of a type and a type that is no optional, unless they are two lists or two functions -/
def scalarJoin : Ty → Ty → Ty
  | a, never => a
  | cls p, cls q => cls (commonPrefix p q)
  | ranged a b, ranged c d => ranged (min a c) (max b d)
  | bool, ranged _ _ | ranged _ _, bool => int
  | a, b => if sub a b then b else if sub b a then a else any

/-- least upper bound: ø joins into an optional (`int | ø` is `int?`) -/
def join : Ty → Ty → Ty
  | never, b => b
  | unit, b => optional (strip b)
  | opt a, b => optional (join a (strip b))
  | list a, b => liftUnit b (match strip b with
    | list c => list (join a c)
    | s => scalarJoin (list a) s)
  | fn a, b => liftUnit b (match strip b with
    | fn c => fn (join a c)
    | s => scalarJoin (fn a) s)
  | a, b => liftUnit b (scalarJoin a (strip b))


/-- the element type of a list type; `never` (an error in list position) has elements of type `never` -/
def element : Ty → Option Ty
  | never => some never
  | list a => some a
  | _ => none

/-- the result of `+`: an int when both operands are, exact when both are, otherwise a number (a float) -/
def arith (a b : Ty) : Ty := if sub a int && sub b int then int else if sub a exact && sub b exact then exact else number

@[simp] theorem sub_never (t : Ty) : sub never t = true := rfl

@[simp] theorem sub_list (a b : Ty) : sub (list a) (list b) = sub a b := rfl

@[simp] theorem sub_fn (a b : Ty) : sub (fn a) (fn b) = sub a b := rfl

theorem sub_opt_left (a t : Ty) : sub (opt a) t = (admitsUnit t && sub a t) := rfl

@[simp] theorem admitsUnit_opt (t : Ty) : admitsUnit (opt t) = true := rfl

@[simp] theorem base_opt (t : Ty) : base (opt t) = base t := rfl

@[simp] theorem strip_opt (t : Ty) : strip (opt t) = strip t := rfl

theorem base_ne_opt (t x : Ty) : base t ≠ opt x := by induction t <;> simp_all [base]

theorem base_base (t : Ty) : base (base t) = base t := by induction t <;> simp_all [base]

theorem strip_ne_opt (t x : Ty) : strip t ≠ opt x := by induction t <;> simp_all [strip]

theorem strip_ne_unit (t : Ty) : strip t ≠ unit := by induction t <;> simp_all [strip]

/-- a type that admits no ø is its own base and has no ø to strip -/
theorem base_of_not_admits {t : Ty} (h : admitsUnit t = false) : base t = t := by cases t <;> simp_all [admitsUnit, base]

theorem strip_of_not_admits {t : Ty} (h : admitsUnit t = false) : strip t = t := by cases t <;> simp_all [admitsUnit, strip]

theorem admitsUnit_of_base_any : ∀ {t : Ty}, base t = any → admitsUnit t = true := by
  intro t; induction t <;> simp_all [base, admitsUnit]

@[simp] theorem sub_any (t : Ty) : sub t any = true := by
  induction t <;> simp_all [sub, admitsUnit, base, scalarSub]

theorem sub_from_any {t : Ty} (h : sub any t = true) : base t = any := by
  simp only [sub] at h; generalize base t = b at h; cases b <;> simp_all [scalarSub]

/-- below a type whose base is `any`, everything -/
theorem sub_of_base_any {c : Ty} (h : base c = any) : ∀ t, sub t c = true := by
  intro t; induction t <;> simp_all [sub, scalarSub, admitsUnit_of_base_any h]

theorem admitsUnit_up : ∀ {b c : Ty}, sub b c = true → admitsUnit b = true → admitsUnit c = true := by
  intro b c h hb
  cases b with
  | unit => exact h
  | opt _ => simp only [sub, Bool.and_eq_true] at h; exact h.1
  | any => exact admitsUnit_of_base_any (sub_from_any h)
  | _ => simp [admitsUnit] at hb

/-- a type other than ø and an optional fits t when it fits t's base -/
theorem sub_base_right {a t : Ty} (hu : a ≠ unit) (ho : ∀ x, a ≠ opt x) : sub a t = sub a (base t) := by
  cases a <;> simp_all [sub, base_base]

theorem admitsUnit_of_base : ∀ {t : Ty}, admitsUnit (base t) = true → admitsUnit t = true := by
  intro t; induction t <;> simp_all [base, admitsUnit]

theorem sub_of_sub_base : ∀ {a t : Ty}, sub a (base t) = true → sub a t = true := by
  intro a; induction a with
  | unit => intro t h; exact admitsUnit_of_base h
  | opt a ih => intro t h; simp only [sub, Bool.and_eq_true] at h ⊢; exact ⟨admitsUnit_of_base h.1, ih h.2⟩
  | _ => intro t h; simpa [sub, base_base] using h

theorem sub_to_opt : ∀ {a t : Ty}, sub a t = true → sub a (opt t) = true := by
  intro a; induction a with
  | never => intros; rfl
  | unit => intros; rfl
  | opt a ih => intro t h; simp only [sub, Bool.and_eq_true] at h ⊢; exact ⟨rfl, ih h.2⟩
  | _ => intro t h; simpa [sub] using h

theorem sub_refl : ∀ t : Ty, sub t t = true
  | opt a => by simp [sub, sub_to_opt (sub_refl a)]
  | list a => by simp [sub_refl a]
  | fn a => by simp [sub_refl a]
  | cls p => by simp [sub, base, scalarSub]
  | ranged a b => by simp [sub, base, scalarSub]
  | quantity d => by simp [sub, base, scalarSub]
  | never | bool | int | exact | number | text | codepoint | unit | any => rfl

theorem sub_base_left : ∀ {b c : Ty}, sub b c = true → sub (base b) c = true := by
  intro b; induction b with
  | opt b ih => intro c h; simp only [sub, Bool.and_eq_true] at h; exact ih h.2
  | _ => intro c h; exact h

theorem scalarSub_trans {a b c : Ty} (hab : scalarSub a b = true) (hbc : scalarSub b c = true) :
    scalarSub a c = true := by
  cases a <;> cases b <;> cases c <;> simp_all [scalarSub] <;> (try omega)
  exact hbc.trans hab

theorem sub_to_never : ∀ {t : Ty}, sub t never = true → t = never := by
  intro t h; cases t <;> simp_all [sub, base, scalarSub, admitsUnit]

theorem sub_to_list : ∀ {t a : Ty}, sub t (list a) = true → t = never ∨ ∃ b, t = list b ∧ sub b a = true := by
  intro t a h; cases t <;> simp_all [sub, base, scalarSub, admitsUnit]

theorem sub_to_fn : ∀ {t a : Ty}, sub t (fn a) = true → t = never ∨ ∃ b, t = fn b ∧ sub b a = true := by
  intro t a h; cases t <;> simp_all [sub, base, scalarSub, admitsUnit]

theorem sub_trans : ∀ {a b c : Ty}, sub a b = true → sub b c = true → sub a c = true := by
  intro a
  induction a with
  | never => intros; rfl
  | unit => intro b c hab hbc; exact admitsUnit_up hbc hab
  | opt a ih =>
    intro b c hab hbc
    simp only [sub, Bool.and_eq_true] at hab ⊢
    exact ⟨admitsUnit_up hbc hab.1, ih hab.2 hbc⟩
  | any => intro b c hab hbc; have h := sub_base_left hbc; rwa [sub_from_any hab] at h
  | list x ih =>
    intro b c hab hbc
    have hbc := sub_base_left hbc
    simp only [sub] at hab ⊢
    generalize base b = B at hab hbc
    cases B <;> simp at hab
    · simp only [sub] at hbc; generalize base c = C at hbc ⊢; cases C <;> simp_all; exact ih hab hbc
    · rw [sub_from_any hbc]
  | fn x ih =>
    intro b c hab hbc
    have hbc := sub_base_left hbc
    simp only [sub] at hab ⊢
    generalize base b = B at hab hbc
    cases B <;> simp at hab
    · simp only [sub] at hbc; generalize base c = C at hbc ⊢; cases C <;> simp_all; exact ih hab hbc
    · rw [sub_from_any hbc]
  | _ =>
    intro b c hab hbc
    have hbc := sub_base_left hbc
    simp only [sub] at hab ⊢
    generalize base b = B at hab hbc
    cases B <;> first | (simp [scalarSub] at hab; done) | exact scalarSub_trans hab hbc


@[simp] theorem admitsUnit_optional (t : Ty) : admitsUnit (optional t) = true := by cases t <;> rfl

theorem sub_optional {x t : Ty} (h : sub x t = true) : sub x (optional t) = true := by
  cases t <;> simp only [optional] <;> first | exact h | exact sub_to_opt h | (rw [sub_to_never h]; rfl)

theorem optional_least {t c : Ty} (hc : admitsUnit c = true) (h : sub t c = true) : sub (optional t) c = true := by
  cases t <;> simp only [optional] <;> first | exact hc | exact h | (rw [sub_opt_left, hc, h]; rfl)

theorem sub_strip_left : ∀ {x t : Ty}, sub x t = true → sub (strip x) t = true := by
  intro x; induction x with
  | unit => intros; rfl
  | opt x ih => intro t h; simp only [sub, Bool.and_eq_true] at h; exact ih h.2
  | _ => intro t h; exact h

/-- a type fits a type that admits ø when its values other than ø do -/
theorem sub_of_strip : ∀ {b t : Ty}, sub (strip b) t = true → admitsUnit t = true → sub b t = true := by
  intro b; induction b with
  | unit => intro t _ hu; exact hu
  | opt b ih => intro t h hu; simp only [sub, Bool.and_eq_true]; exact ⟨hu, ih h hu⟩
  | _ => intro t h _; exact h

/-- a type other than ø and an optional fits t when it fits t's values other than ø -/
theorem sub_strip_right {a : Ty} (hu : a ≠ unit) (ho : ∀ x, a ≠ opt x) : ∀ {t : Ty}, sub a t = true → sub a (strip t) = true := by
  intro t; induction t with
  | unit => intro h; cases a <;> simp_all [sub, scalarSub]
  | opt t ih => intro h; rw [sub_base_right hu ho, base_opt, ← sub_base_right hu ho] at h; exact ih h
  | _ => intro h; exact h

theorem strip_mono : ∀ {a b : Ty}, sub a b = true → sub (strip a) (strip b) = true := by
  intro a; induction a with
  | never | unit => intros; rfl
  | opt a ih => intro b h; simp only [sub, Bool.and_eq_true] at h; exact ih h.2
  | _ => intro b h; exact sub_strip_right (by simp) (by simp) h

theorem sub_liftUnit_left {x b t : Ty} (h : sub x t = true) : sub x (liftUnit b t) = true := by
  unfold liftUnit; split
  · exact sub_optional h
  · exact h

theorem sub_liftUnit_right {b t : Ty} (h : sub (strip b) t = true) : sub b (liftUnit b t) = true := by
  unfold liftUnit; split
  · exact sub_of_strip (sub_optional h) (admitsUnit_optional _)
  · rwa [strip_of_not_admits (Bool.eq_false_iff.mpr ‹_›)] at h

theorem liftUnit_least {b t c : Ty} (hb : admitsUnit b = true → admitsUnit c = true) (h : sub t c = true) :
    sub (liftUnit b t) c = true := by
  unfold liftUnit; split
  · exact optional_least (hb ‹_›) h
  · exact h

theorem scalarJoin_upper_left (a s : Ty) : sub a (scalarJoin a s) = true := by
  unfold scalarJoin; split <;> (try split) <;> (try split) <;> simp_all [sub_refl, sub, commonPrefix_left] <;> omega

theorem scalarJoin_upper_right (a s : Ty) : sub s (scalarJoin a s) = true := by
  unfold scalarJoin; split <;> (try split) <;> (try split) <;> simp_all [sub_refl, sub, commonPrefix_right] <;> omega

/-- two lists or two functions: their join is no scalar join -/
def sameShape : Ty → Ty → Bool
  | list _, list _ | fn _, fn _ => true
  | _, _ => false

theorem scalarJoin_least {a s c : Ty} (hau : a ≠ unit) (hao : ∀ x, a ≠ opt x) (hsu : s ≠ unit) (hso : ∀ x, s ≠ opt x)
    (shape : sameShape a s = false) (hac : sub a c = true) (hsc : sub s c = true) : sub (scalarJoin a s) c = true := by
  apply sub_of_sub_base
  rw [sub_base_right hau hao] at hac
  rw [sub_base_right hsu hso] at hsc
  have hC := base_ne_opt c
  generalize base c = C at hac hsc hC
  cases C with
  | any => exact sub_any _
  | opt x => exact absurd rfl (hC x)
  | _ =>
    cases a <;> (try simp [sub] at hac) <;> (try simp at hau hao) <;> cases s <;> (try simp [sub] at hsc) <;>
      (try simp at hsu hso shape) <;> simp_all [scalarJoin, sub, sameShape] <;> (try omega)
    all_goals exact commonPrefix_greatest hac hsc

theorem join_upper_left : ∀ a b : Ty, sub a (join a b) = true := by
  intro a
  induction a with
  | never => intro b; rfl
  | unit => intro b; exact admitsUnit_optional _
  | opt a ih => intro b; simp only [join, sub, admitsUnit_optional, Bool.true_and]; exact sub_optional (ih _)
  | list x ih =>
    intro b; simp only [join]; apply sub_liftUnit_left
    generalize strip b = s; cases s <;> first | exact ih _ | exact scalarJoin_upper_left _ _
  | fn x ih =>
    intro b; simp only [join]; apply sub_liftUnit_left
    generalize strip b = s; cases s <;> first | exact ih _ | exact scalarJoin_upper_left _ _
  | _ => intro b; exact sub_liftUnit_left (scalarJoin_upper_left _ _)

theorem join_upper_right : ∀ a b : Ty, sub b (join a b) = true := by
  intro a
  induction a with
  | never => intro b; exact sub_refl b
  | unit => intro b; exact sub_of_strip (sub_optional (sub_refl _)) (admitsUnit_optional _)
  | opt a ih => intro b; exact sub_of_strip (sub_optional (ih _)) (admitsUnit_optional _)
  | list x ih =>
    intro b; simp only [join]; apply sub_liftUnit_right
    generalize strip b = s; cases s <;> first | exact ih _ | exact scalarJoin_upper_right _ _
  | fn x ih =>
    intro b; simp only [join]; apply sub_liftUnit_right
    generalize strip b = s; cases s <;> first | exact ih _ | exact scalarJoin_upper_right _ _
  | _ => intro b; exact sub_liftUnit_right (scalarJoin_upper_right _ _)

theorem join_least : ∀ {a b c : Ty}, sub a c = true → sub b c = true → sub (join a b) c = true := by
  intro a
  induction a with
  | never => intro b c _ hbc; exact hbc
  | unit => intro b c hac hbc; exact optional_least hac (sub_strip_left hbc)
  | opt a ih =>
    intro b c hac hbc
    simp only [sub, Bool.and_eq_true] at hac
    exact optional_least hac.1 (ih hac.2 (sub_strip_left hbc))
  | any => intro b c hac _; exact sub_of_base_any (sub_from_any hac) _
  | list x ih =>
    intro b c hac hbc
    simp only [join]
    refine liftUnit_least (admitsUnit_up hbc) ?_
    have hs := sub_strip_left hbc
    have hsu := strip_ne_unit b
    have hso := strip_ne_opt b
    generalize strip b = s at hs hsu hso ⊢
    cases s
    case list y =>
      simp only [sub] at hac hs ⊢; generalize base c = C at hac hs ⊢; cases C <;> simp_all
    all_goals exact scalarJoin_least (by simp) (by simp) hsu hso rfl hac hs
  | fn x ih =>
    intro b c hac hbc
    simp only [join]
    refine liftUnit_least (admitsUnit_up hbc) ?_
    have hs := sub_strip_left hbc
    have hsu := strip_ne_unit b
    have hso := strip_ne_opt b
    generalize strip b = s at hs hsu hso ⊢
    cases s
    case fn y =>
      simp only [sub] at hac hs ⊢; generalize base c = C at hac hs ⊢; cases C <;> simp_all
    all_goals exact scalarJoin_least (by simp) (by simp) hsu hso rfl hac hs
  | _ =>
    intro b c hac hbc
    exact liftUnit_least (admitsUnit_up hbc)
      (scalarJoin_least (by simp) (by simp) (strip_ne_unit b) (strip_ne_opt b) (by cases strip b <;> rfl) hac
        (sub_strip_left hbc))

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
value may go where a cast checks it, as an int may go to a fixed width whose range a cast checks, and an optional where its values other than ø go -/
def consub : Ty → Ty → Bool
  | any, _ => true
  | opt a, b => consub a b
  | int, ranged _ _ | ranged _ _, ranged _ _ => true
  | list a, list b => consub a b
  | a, b => sub a b

theorem arith_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (arith a' b') (arith a b) = true := by
  unfold arith
  by_cases h : (sub a int && sub b int) = true
  · simp at h
    simp [sub_trans ha h.1, sub_trans hb h.2, h.1, h.2, sub_refl]
  simp only [h, Bool.false_eq_true, ↓reduceIte]
  by_cases e : (sub a exact && sub b exact) = true
  · simp at e
    simp only [sub_trans ha e.1, sub_trans hb e.2, Bool.and_self, ↓reduceIte, e.1, e.2]
    split <;> simp [sub]
  · simp only [e, Bool.false_eq_true, ↓reduceIte]; (repeat' split) <;> simp [sub]

theorem arith_sub_number (a b : Ty) : sub (arith a b) number = true := by
  unfold arith; (repeat' split) <;> simp [sub]

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

/-- an operand that admits ø (ø, an optional, `any`) makes an operation dynamic: checked when it runs (the checker
rejects an optional operand, as warp does: `a may be ø`) -/
def dynamic (a b : Ty) : Bool := admitsUnit a || admitsUnit b

/-- below a static operation, a static one, whose operands admit no ø -/
theorem not_dynamic {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) (h : ¬dynamic a b = true) :
    ¬dynamic a' b' = true ∧ admitsUnit a = false ∧ admitsUnit b = false := by
  simp only [dynamic, Bool.or_eq_true, not_or, Bool.not_eq_true] at h ⊢
  refine ⟨⟨Bool.eq_false_iff.mpr fun h' => ?_, Bool.eq_false_iff.mpr fun h' => ?_⟩, h⟩
  · simp [admitsUnit_up ha h'] at h
  · simp [admitsUnit_up hb h'] at h

/-- a quantity type has no subtypes but itself and `never`, and no supertypes that admit no ø but itself -/
theorem quantity_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : admitsUnit a = false) :
    isQuantity a' = isQuantity a := by
  cases a' <;> cases a <;> simp_all [sub, admitsUnit, isQuantity]

theorem quantity_eq {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (q : isQuantity a = true) : a' = a := by
  cases a' <;> cases a <;> simp_all [sub, admitsUnit, isQuantity]

theorem dimsOf_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : admitsUnit a = false) : dimsOf a' = dimsOf a := by
  cases a' <;> cases a <;> simp_all [sub, admitsUnit, dimsOf]

/-- `+` and `-` of quantities: one dimension on both sides, else a DimensionError -/
def sameQuantity (a b : Ty) : Ty := if a = b then a else never

theorem sameQuantity_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) (na : a' ≠ never)
    (nb : b' ≠ never) (ya : admitsUnit a = false) (yb : admitsUnit b = false) (q : (isQuantity a || isQuantity b) = true) :
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
theorem isText_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : admitsUnit a = false) : isText a' = isText a := by
  cases a' <;> cases a <;> simp_all [sub, admitsUnit, isText]

/-- between `never` and `any`, widening keeps a number a number and a non-number a non-number -/
theorem number_up {a a' : Ty} (h : sub a' a = true) (n : a' ≠ never) (y : admitsUnit a = false) :
    sub a' number = sub a number := by
  cases a' <;> cases a <;> simp_all [sub, admitsUnit]

/-- the result of `+`: `never` when a side raises, dynamic when a side is, two lists' concatenation a list of their
elements' join, a text when a side is a text, a number type otherwise -/
def plus (a b : Ty) : Ty :=
  if a = never ∨ b = never then never else if dynamic a b = true then any
  else if (isQuantity a || isQuantity b) = true then sameQuantity a b
  else if (isListTy a && isListTy b) = true then list (join ((element a).getD any) ((element b).getD any))
  else if (isText a || isText b) = true then text else arith a b

/-- the result of `-` and `*`: dynamic when a side is -/
def arithTy (a b : Ty) : Ty := if dynamic a b = true then any else arith a b

theorem sub_to_text : ∀ {t : Ty}, sub t text = true → t = never ∨ t = text ∨ t = codepoint := by
  intro t h; cases t <;> simp_all [sub, admitsUnit]

theorem arithTy_mono {a b a' b' : Ty} (ha : sub a' a = true) (hb : sub b' b = true) :
    sub (arithTy a' b') (arithTy a b) = true := by
  unfold arithTy
  by_cases y : dynamic a b = true
  · rw [ite_eq_left y]; exact sub_any _
  obtain ⟨y', ya, yb⟩ := not_dynamic ha hb y
  rw [ite_eq_right y, ite_eq_right y']
  exact arith_mono ha hb

/-- the result of `*`: a text times a whole number (either order) repeats the text (P1), numbers multiply -/
def repeatTy (a b : Ty) : Ty :=
  if a = never ∨ b = never then never
  else if dynamic a b = true then any
  else if (isText a && sub b number || sub a number && isText b) = true then text
  else arith a b

/-- above a number other than `never`, below `any`: a number -/
theorem sub_number_up {b b' : Ty} (h : sub b' number = true) (hb : sub b' b = true) (ny : admitsUnit b = false) (nn : b' ≠ never) :
    sub b number = true := by
  cases b' <;> cases b <;> simp_all [sub, admitsUnit]

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
  by_cases y : dynamic a b = true
  · rw [ite_eq_left y]; exact sub_any _
  obtain ⟨y', ya, yb⟩ := not_dynamic ha hb y
  rw [ite_eq_right y, ite_eq_right y']
  simp only [not_or] at n n'
  -- a text side stays a text side when widened (only types that admit ø are above text), and a number side a number
  rw [isText_up ha n'.1 ya, isText_up hb n'.2 yb, number_up ha n'.1 ya, number_up hb n'.2 yb]
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
  by_cases y : dynamic a b = true
  · rw [ite_eq_left y]; exact sub_any _
  obtain ⟨y', ya, yb⟩ := not_dynamic ha hb y
  rw [ite_eq_right y, ite_eq_right y']
  have na : a' ≠ never := fun h => n' (.inl h)
  have nb : b' ≠ never := fun h => n' (.inr h)
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
    have la : isListTy a = true := by cases a <;> simp_all [sub, admitsUnit, isListTy]
    have lb : isListTy b = true := by cases b <;> simp_all [sub, admitsUnit, isListTy]
    simp [la, lb]
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
  | text | codepoint => cases l' <;> simp_all [sub, admitsUnit, elementTy, element, isText]
  | _ => simp [elementTy, element, isText]

end Ty
end Warp
