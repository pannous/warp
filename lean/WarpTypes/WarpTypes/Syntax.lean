import WarpTypes.Ty

/-! Expressions of W0 (notes/type_theory.md). Main-level names live in the store (`glob`), parameters and lets are
substituted (`loc`). Lists are cons cells, as warp's GC `$Node` lists are. -/

namespace Warp

/-- how a main-level name may change: `x = 1` (var), `const x = 1`, `z := e` (charged: its body runs at each read, P71) -/
inductive Mode where
  | var | const | charged
  deriving DecidableEq, Repr

/-- `-` and `*`: numbers only, unlike `+` -/
inductive ArithOp where
  | sub | mul
  deriving DecidableEq, Repr

def ArithOp.apply : ArithOp → Int → Int → Int
  | .sub, a, b => a - b
  | .mul, a, b => a * b

inductive Expr where
  | bool (b : Bool)
  | int (n : Int)
  /-- an exact or float number; its representation does not matter to the type theory -/
  | num (n : Int)
  | text (s : String)
  | unit
  | nil
  | cons (head tail : Expr)
  | glob (x : String)
  | loc (y : String)
  | add (a b : Expr)
  | arith (op : ArithOp) (a b : Expr)
  | lt (a b : Expr)
  | eq (a b : Expr)
  | ite (c a b : Expr)
  | loop (c body : Expr)
  | seq (a b : Expr)
  /-- `xs#i`, 1-based -/
  | index (l i : Expr)
  /-- `a ++ b`; `xs.add(v)` is `xs = xs ++ [v]` -/
  | append (a b : Expr)
  | assign (x : String) (e : Expr)
  /-- the first binding of a main-level name -/
  | init (x : String) (e : Expr)
  | letIn (y : String) (t : Ty) (e body : Expr)
  | call (f : String) (arg : Expr)
  | error (msg : String)
  | tryCatch (e handler : Expr)
  /-- the run-time check warp inserts where a value of unknown type goes to a declared place (`names = f()` of a
  `names: texts`, card list-element-types; any value given to an inline union `x: int | text` or an optional
  `x: int?`): the value if it fits one of the alternatives ts, else an error -/
  | cast (e : Expr) (ts : List Ty)
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
  deriving DecidableEq, Repr

/-- the local a handler reads the emitted payload from -/
def eventLocal : String := "event"

namespace Expr

def isValue : Expr → Bool
  | bool _ | int _ | num _ | text _ | unit | nil | ref _ _ => true
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
  | eq a b => eq (a.subst y v) (b.subst y v)
  | ite c a b => ite (c.subst y v) (a.subst y v) (b.subst y v)
  | loop c b => loop (c.subst y v) (b.subst y v)
  | seq a b => seq (a.subst y v) (b.subst y v)
  | index a b => index (a.subst y v) (b.subst y v)
  | append a b => append (a.subst y v) (b.subst y v)
  | assign x e => assign x (e.subst y v)
  | init x e => init x (e.subst y v)
  | letIn z t e b => letIn z t (e.subst y v) (if z = y then b else b.subst y v)
  | call f e => call f (e.subst y v)
  | tryCatch e h => tryCatch (e.subst y v) (h.subst y v)
  | cast e ts => cast (e.subst y v) ts
  | broadcast f e => broadcast f (e.subst y v)
  | get e f => get (e.subst y v) f
  | set e f w => set (e.subst y v) f (w.subst y v)
  | isA e c => isA (e.subst y v) c
  | handle ev h b => handle ev (if y = eventLocal then h else h.subst y v) (b.subst y v)
  | emit ev e => emit ev (e.subst y v)
  | scope k e => scope k (e.subst y v)
  | abort ev k e => abort ev k (e.subst y v)
  | e => e

/-- the main-level names an expression assigns or binds -/
def assigned : Expr → List String
  | assign x e | init x e => x :: e.assigned
  | cons a b | add a b | arith _ a b | lt a b | eq a b | loop a b | seq a b | index a b | append a b | tryCatch a b
  | handle _ a b => a.assigned ++ b.assigned
  | ite c a b => c.assigned ++ a.assigned ++ b.assigned
  | letIn _ _ e b => e.assigned ++ b.assigned
  | set a _ b => a.assigned ++ b.assigned
  | call _ e | cast e _ | broadcast _ e | get e _ | isA e _ | emit _ e | scope _ e | abort _ _ e => e.assigned
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
