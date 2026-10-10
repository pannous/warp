import WarpTypes.Ty

/-! Expressions of W0 (notes/type_theory.md). Main-level names live in the store (`glob`), parameters and lets are
substituted (`loc`). Lists are cons cells, as warp's GC `$Node` lists are. -/

namespace Warp

/-- how a main-level name may change: `x = 1` (var), `const x = 1`, `z := e` (charged: its body runs at each read, P71) -/
inductive Mode where
  | var | const | charged
  deriving DecidableEq, Repr

/-- `-`, `*`, `%` and `/`: numbers only, unlike `+` -/
inductive ArithOp where
  | sub | mul
  /-- the Euclidean remainder: `-7 % 3` is 2, `7 % -3` is 1, as warp's -/
  | mod
  /-- `6/2` is the int 3, `7/2` the number 3.5 -/
  | div
  /-- `2^10` is the int 1024, `2^-1` the number 0.5 -/
  | pow
  deriving DecidableEq, Repr

def ArithOp.apply : ArithOp → Int → Int → Int
  | .sub, a, b => a - b
  | .mul, a, b => a * b
  | .mod, a, b => a % b
  | .div, a, b => a / b
  | .pow, a, b => a ^ b.toNat

/-- the Euclidean remainder of rationals, as of ints: `7.5 % 2` is 1.5, `-1.5 % 1` is 0.5 -/
def ratMod (a b : Rat) : Rat :=
  let size := if b < 0 then -b else b
  a - size * (a / size).floor

/-- an operation on exact numbers: none when the result is no rational (`2^0.5`), a float in warp -/
def ArithOp.applyExact : ArithOp → Rat → Rat → Option Rat
  | .sub, a, b => some (a - b)
  | .mul, a, b => some (a * b)
  | .mod, a, b => some (ratMod a b)
  | .div, a, b => some (a / b)
  | .pow, a, b => if b.den = 1 then some (a ^ b.num) else none

def ArithOp.applyFloat : ArithOp → Float → Float → Float
  | .sub, a, b => a - b
  | .mul, a, b => a * b
  | .mod, a, b => a - b.abs * (a / b.abs).floor
  | .div, a, b => a / b
  | .pow, a, b => a ^ b

/-- the left side of `/` and `^` counts as exact: two ints divide to a fraction, a negative power is one; a power
whose exponent may be fractional (`2^0.5`) counts as a float -/
def ArithOp.widen : ArithOp → Ty → Ty → Ty
  | .div, t, _ => Ty.join t .exact
  | .pow, t, e => Ty.join t (if Ty.sub e .int then .exact else .number)
  | _, t, _ => t

theorem ArithOp.widen_mono (op : ArithOp) {a b a' b' : Ty} (h : Ty.sub a' a = true) (hb : Ty.sub b' b = true) :
    Ty.sub (op.widen a' b') (op.widen a b) = true := by
  cases op <;> simp only [widen] <;> try first | exact h | exact Ty.join_mono h (Ty.sub_refl _)
  by_cases e : Ty.sub b .int = true
  · rw [if_pos (Ty.sub_trans hb e), if_pos e]; exact Ty.join_mono h (Ty.sub_refl _)
  · rw [if_neg e]; split
    · exact Ty.join_mono h (by decide)
    · exact Ty.join_mono h (Ty.sub_refl _)

/-- the result type of an arithmetic operation on numbers: `*` also repeats a text -/
def ArithOp.numberTy : ArithOp → Ty → Ty → Ty
  | .mul, a, b => Ty.repeatTy a b
  | op, a, b => Ty.arithTy (op.widen a b) b

theorem ArithOp.numberTy_mono (op : ArithOp) {a b a' b' : Ty} (ha : Ty.sub a' a = true) (hb : Ty.sub b' b = true) :
    Ty.sub (op.numberTy a' b') (op.numberTy a b) = true := by
  cases op
  case mul => exact Ty.repeatTy_mono ha hb
  all_goals exact Ty.arithTy_mono (ArithOp.widen_mono _ ha hb) hb

/-- the dimensions of a product (`*`) or quotient (`/`) of quantities of dimensions d and e -/
def ArithOp.dims : ArithOp → Dims → Dims → Option Dims
  | .mul, d, e => some (Dims.times d e)
  | .div, d, e => some (Dims.times d (Dims.inverse e))
  | _, _, _ => none

/-- the result type of an arithmetic operation with a quantity side: `-` keeps one dimension, `*` and `/` combine a
quantity with a quantity or a number, anything else raises a DimensionError -/
def ArithOp.quantityTy (op : ArithOp) (a b : Ty) : Ty :=
  if op = .sub then Ty.sameQuantity a b else
  match a.dimsOf, b.dimsOf with
  | some d, some e => ((op.dims d e).map Ty.ofDims).getD .never
  | _, _ => .never

/-- the result type of an arithmetic operation: `never` when a side raises, dynamic when a side is -/
def ArithOp.ty (op : ArithOp) (a b : Ty) : Ty :=
  if a = .never ∨ b = .never then .never else if a = .any ∨ b = .any then .any
  else if (a.isQuantity || b.isQuantity) = true then op.quantityTy a b else op.numberTy a b

theorem ArithOp.ty_mono (op : ArithOp) {a b a' b' : Ty} (ha : Ty.sub a' a = true) (hb : Ty.sub b' b = true) :
    Ty.sub (op.ty a' b') (op.ty a b) = true := by
  unfold ArithOp.ty
  by_cases n' : a' = .never ∨ b' = .never
  · rw [ite_eq_left n']; exact Ty.sub_never _
  rw [ite_eq_right n']
  have na : a' ≠ .never := fun h => n' (.inl h)
  have nb : b' ≠ .never := fun h => n' (.inr h)
  have n : ¬(a = .never ∨ b = .never) := by
    rintro (rfl | rfl)
    · exact na (Ty.sub_to_never ha)
    · exact nb (Ty.sub_to_never hb)
  rw [ite_eq_right n]
  by_cases y : a = .any ∨ b = .any
  · rw [ite_eq_left y]; exact Ty.sub_any _
  rw [ite_eq_right y]
  have ya : a ≠ .any := fun h => y (.inl h)
  have yb : b ≠ .any := fun h => y (.inr h)
  have y' : ¬(a' = .any ∨ b' = .any) := by
    rintro (rfl | rfl)
    · exact ya (Ty.sub_from_any ha)
    · exact yb (Ty.sub_from_any hb)
  rw [ite_eq_right y', Ty.quantity_up ha na ya, Ty.quantity_up hb nb yb]
  by_cases q : (a.isQuantity || b.isQuantity) = true
  · rw [ite_eq_left q, ite_eq_left q]
    unfold ArithOp.quantityTy
    split
    · exact Ty.sameQuantity_mono ha hb na nb ya yb q
    · rw [Ty.dimsOf_up ha na ya, Ty.dimsOf_up hb nb yb]; exact Ty.sub_refl _
  · rw [ite_eq_right q, ite_eq_right q]; exact op.numberTy_mono ha hb

inductive Expr where
  | bool (b : Bool)
  | int (n : Int)
  /-- an exact number that is no int: `0.5`, `7/2` (a whole result is an int, as in warp) -/
  | num (q : Rat)
  /-- a float, by its bits: `sqrt(2)`, `x as float` -/
  | flt (bits : UInt64)
  /-- a quantity, `2 m`: n counts the smallest step of each base dimension (units.rs `factor`: 0.1 mm, ms, 10 µg),
  so every unit is a whole number of steps -/
  | qty (n : Int) (dims : Dims)
  | text (s : String)
  | unit
  | nil
  | cons (head tail : Expr)
  | glob (x : String)
  | loc (y : String)
  | add (a b : Expr)
  | arith (op : ArithOp) (a b : Expr)
  | lt (a b : Expr)
  /-- `a == b` (loose), or with `same` (`a same b`, `===`) identity on instances (P208) -/
  | eq (same : Bool) (a b : Expr)
  | ite (c a b : Expr)
  /-- `while c { body }` (P55): its value is the last body value, ø when the body never ran; `last` is that value so
  far, ø in a program, or the body running -/
  | loop (c body last : Expr)
  | seq (a b : Expr)
  /-- `xs#i`, 1-based -/
  | index (l i : Expr)
  /-- `a..b`, the ints from a up to b, b excluded (`a to b` is `a..b+1`); none when b ≤ a -/
  | range (a b : Expr)
  /-- `a ++ b`; `xs.add(v)` is `xs = xs ++ [v]` -/
  | append (a b : Expr)
  | assign (x : String) (e : Expr)
  /-- the first binding of a main-level name -/
  | init (x : String) (e : Expr)
  | letIn (y : String) (t : Ty) (e body : Expr)
  | call (f : String) (arg : Expr)
  | error (msg : String)
  | tryCatch (e handler : Expr)
  /-- a stored error, `error("x")` as a value (Decided #1, errors as values): a name, a list item or an argument keeps
  it, `if` takes it as false, `try` and `failed` test for it, any other operation given one raises -/
  | fail (msg : String)
  /-- `e failed`: is the value of e a stored error -/
  | failed (e : Expr)
  /-- the run-time check warp inserts where a value of unknown type goes to a declared place (`names = f()` of a
  `names: texts`, card list-element-types; any value given to an inline union `x: int | text` or an optional
  `x: int?`): the value if it fits one of the alternatives ts, else an error -/
  | cast (e : Expr) (ts : List Ty)
  /-- `e as t`, warp's conversion: a scalar converts to text, int, number or bool (`3.7 as int` is 3, `"4" as int` is
  4, `3 as text` is "3"), a text that is no number fails when it runs; to any other type it is a checked cast. A
  declared result `def f(x) -> int { body }` is `body as int` -/
  | conv (e : Expr) (t : Ty)
  /-- broadcasting: `f(xs)` of a function of A given a list of A applies f to each item (`f(x: int) := x+1; f([1])` is
  [2]); warp decides it at compile time, so it is its own form -/
  | broadcast (f : String) (arg : Expr)
  /-- a reference to the object at heap address a, of the class with ancestor chain `path`: class instances are
  shared, so a mutation through a parameter is visible to the caller (P200) -/
  | ref (a : Nat) (path : List String)
  /-- a fresh instance with its fields unset; a constructor `Circle(2)` is `new` followed by field writes -/
  | new (path : List String)
  /-- `o.f` -/
  | get (e : Expr) (f : String)
  /-- `o.f = v`, giving v -/
  | set (e : Expr) (f : String) (v : Expr)
  /-- `e is C`, the type test of a match arm `Shape::Circle(r)` (P178) -/
  | isA (e : Expr) (c : String)
  /-- `on ev {h} in {body}`: body runs with h answering the event ev; in h the local `event` is the emitted payload
  (effect handlers, notes/effect_handlers.md: block-scoped, dynamically scoped, tail-resumptive) -/
  | handle (ev : String) (h body : Expr)
  /-- `emit ev{payload}`: the innermost active handler of ev runs on the payload, and its value is emit's value; with no
  handler, a program-wide `on ev {…}`; with none, ø (P202) -/
  | emit (ev : String) (payload : Expr)
  /-- a handler body running where its emit was, with only the k handlers outside the one answering: an emit inside
  a handler goes to the next handler outward (run time only) -/
  | scope (k : Nat) (e : Expr)
  /-- `break v` in a block handler of ev (aborting handlers, notes/effect_handlers.md Step 3): the emit does not
  resume, the block `on ev {…} in {…}` whose handler ran ends with v. k is the depth of that block (the handlers
  outside it), unknown (none) until the abort leaves the handler's scope (run time) -/
  | abort (ev : String) (k : Option Nat) (e : Expr)
  /-- a shared list (P200b): the list at address a of the store's lists, made with element type t (declared, else
  any); every name holding it sees an item added through another -/
  | lref (a : Nat) (t : Ty)
  /-- a new shared list of the items of the list value e, of element type t: a list literal, `a..b`, `xs + ys` -/
  | share (e : Expr) (t : Ty)
  /-- `xs.add(v)`: v joins the end of the shared list xs if it fits xs's element type, else a loud error (P215's
  run-time half); gives xs -/
  | push (l v : Expr)
  /-- `xs#i = v`: v replaces the i-th item of the shared list xs if it fits xs's element type, else a loud error; gives v -/
  | setAt (l i v : Expr)
  /-- `for y in l { body }`: body runs once per item of the list l, the local y holding the item; gives ø -/
  | forIn (y : String) (l body last : Expr)
  /-- `y => body`, a lambda: it evaluates to the closure `clo y body` once the locals it captures are substituted -/
  | lam (y : String) (body : Expr)
  /-- a function value: closed but for its parameter y (run time only) -/
  | clo (y : String) (body : Expr)
  /-- `f(a)` of a function value f -/
  | app (f a : Expr)
  deriving DecidableEq, Repr

/-- the local a handler reads the emitted payload from -/
def eventLocal : String := "event"

namespace Expr

def isValue : Expr → Bool
  | bool _ | int _ | num _ | flt _ | qty _ _ | text _ | unit | nil | ref _ _ | clo _ _ | lref _ _ | fail _ => true
  | cons h t => h.isValue && t.isValue
  | _ => false

/-- replace the local `y` by the closed value `v` -/
def subst (e : Expr) (y : String) (v : Expr) : Expr :=
  match e with
  | loc z => if z = y then v else loc z
  | cons a b => cons (a.subst y v) (b.subst y v)
  | add a b => add (a.subst y v) (b.subst y v)
  | arith op a b => arith op (a.subst y v) (b.subst y v)
  | lt a b => lt (a.subst y v) (b.subst y v)
  | eq s a b => eq s (a.subst y v) (b.subst y v)
  | ite c a b => ite (c.subst y v) (a.subst y v) (b.subst y v)
  | loop c b d => loop (c.subst y v) (b.subst y v) (d.subst y v)
  | seq a b => seq (a.subst y v) (b.subst y v)
  | index a b => index (a.subst y v) (b.subst y v)
  | range a b => range (a.subst y v) (b.subst y v)
  | append a b => append (a.subst y v) (b.subst y v)
  | assign x e => assign x (e.subst y v)
  | init x e => init x (e.subst y v)
  | letIn z t e b => letIn z t (e.subst y v) (if z = y then b else b.subst y v)
  | call f e => call f (e.subst y v)
  | tryCatch e h => tryCatch (e.subst y v) (h.subst y v)
  | cast e ts => cast (e.subst y v) ts
  | conv e t => conv (e.subst y v) t
  | broadcast f e => broadcast f (e.subst y v)
  | get e f => get (e.subst y v) f
  | set e f w => set (e.subst y v) f (w.subst y v)
  | isA e c => isA (e.subst y v) c
  | failed e => failed (e.subst y v)
  | handle ev h b => handle ev (if y = eventLocal then h else h.subst y v) (b.subst y v)
  | emit ev e => emit ev (e.subst y v)
  | scope k e => scope k (e.subst y v)
  | abort ev k e => abort ev k (e.subst y v)
  | forIn z l b d => forIn z (l.subst y v) (if z = y then b else b.subst y v) (d.subst y v)
  | lam z b => lam z (if z = y then b else b.subst y v)
  | app f a => app (f.subst y v) (a.subst y v)
  | share e t => share (e.subst y v) t
  | push l w => push (l.subst y v) (w.subst y v)
  | setAt l i w => setAt (l.subst y v) (i.subst y v) (w.subst y v)
  | e => e

/-- the main-level names an expression assigns or binds -/
def assigned : Expr → List String
  | assign x e | init x e => x :: e.assigned
  | cons a b | add a b | arith _ a b | lt a b | eq _ a b | seq a b | index a b | range a b | append a b
  | tryCatch a b | app a b | push a b
  | handle _ a b => a.assigned ++ b.assigned
  | ite c a b | loop c a b | setAt c a b => c.assigned ++ a.assigned ++ b.assigned
  | forIn _ e b d => e.assigned ++ b.assigned ++ d.assigned
  | letIn _ _ e b => e.assigned ++ b.assigned
  | set a _ b => a.assigned ++ b.assigned
  | call _ e | cast e _ | conv e _ | broadcast _ e | get e _ | isA e _ | failed e | emit _ e | scope _ e | abort _ _ e | lam _ e
  | share e _ => e.assigned
  | _ => []

end Expr

/-- a top-level function `f(param: paramTy): result := body` -/
structure Fn where
  param : String
  paramTy : Ty
  result : Ty
  body : Expr

/-- the declarations of a program: main-level names with their mode and type, and the functions -/
structure Program where
  names : String → Option (Mode × Ty)
  funs : String → Option Fn
  /-- names declared `global`: the only main-level names a function may assign (functions2, `global names`) -/
  globals : String → Bool
  /-- the fields a class declares itself, with their types -/
  fields : String → String → Option Ty := fun _ _ => none
  /-- each event's result type: what its handlers give back to an emit -/
  effects : String → Option Ty := fun _ => none
  /-- the program-wide handlers, `on ev {…}` at main level: they answer when no block handler is active -/
  handlers : String → Option Expr := fun _ => none
  /-- each event's abort type: what a `break v` of its block handlers gives the block -/
  aborts : String → Ty := fun _ => .never
  /-- the fields of an instance of the class chain p, for `==` (run time only) -/
  fieldNames : List String → List String := fun _ => []

/-- the type of field f of an instance of the class chain p: the declaration nearest the root wins, so a subclass
keeps the field types of its ancestors -/
def Program.fieldTy (P : Program) (p : List String) (f : String) : Option Ty := p.findSome? (P.fields · f)

theorem Program.fieldTy_prefix (P : Program) {q p : List String} (h : q <+: p) {f t} (hq : P.fieldTy q f = some t) :
    P.fieldTy p f = some t := by
  obtain ⟨r, rfl⟩ := h
  simp [Program.fieldTy, List.findSome?_append] at hq ⊢
  simp [hq]

/-- the types of the locals in scope -/
abbrev Ctx := String → Option Ty

def Ctx.empty : Ctx := fun _ => none

def Ctx.set (Γ : Ctx) (y : String) (t : Ty) : Ctx := fun z => if z = y then some t else Γ z

end Warp
