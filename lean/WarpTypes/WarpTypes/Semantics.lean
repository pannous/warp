import WarpTypes.Typing

/-! Small-step semantics of W0: left to right, call by value. An `error` in any evaluation position propagates,
except under `try`. Main-level names are cells of the store. -/

namespace Warp
open Ty Expr

inductive Cell where
  | unset
  | val (v : Expr)
  /-- the body of a charged name `z := e`, run at each read -/
  | charged (body : Expr)

abbrev Store := String → Option Cell

def Store.set (μ : Store) (x : String) (c : Cell) : Store := fun z => if z = x then some c else μ z

/-- true/false act as 1/0 -/
def asInt : Expr → Option Int
  | .bool b => some (if b then 1 else 0)
  | .int n => some n
  | _ => none

def asNumber : Expr → Int
  | .num n => n
  | e => (asInt e).getD 0

/-- a value as `+` joins it to a text -/
def render : Expr → String
  | .text s => s
  | .bool b => if b then "true" else "false"
  | .int n | .num n => toString n
  | _ => ""

def isText : Expr → Bool
  | .text _ => true
  | _ => false

def addValues (a b : Expr) : Expr :=
  if isText a || isText b then .text (render a ++ render b) else
  match asInt a, asInt b with
  | some x, some y => .int (x + y)
  | _, _ => .num (asNumber a + asNumber b)

/-- false, 0, "", ø and [] are falsy -/
def truthy : Expr → Bool
  | .bool b => b
  | .int n => n != 0
  | .num n => n != 0
  | .text s => !s.isEmpty
  | .unit | .nil => false
  | _ => true

/-- the i-th element, 1-based -/
def nth : Expr → Int → Option Expr
  | .cons h t, i => if i = 1 then some h else nth t (i - 1)
  | _, _ => none

def appendValues : Expr → Expr → Expr
  | .cons h t, b => .cons h (appendValues t b)
  | _, b => b

/-- the type of a value, as `HasType` gives it -/
def valueType : Expr → Option Ty
  | .bool _ => some .bool
  | .int _ => some .int
  | .num _ => some .number
  | .text _ => some .text
  | .unit => some .unit
  | .nil => some (.list .never)
  | .cons h t =>
    match valueType h, valueType t with
    | some a, some l =>
      match element l with
      | some e => some (.list (join a e))
      | none => none
    | _, _ => none
  | _ => none

/-- the run-time type test of a cast -/
def fits (v : Expr) (t : Ty) : Bool :=
  match valueType v with
  | some tv => sub tv t
  | none => false

/-- an evaluation position: the hole is evaluated next once the expressions left of it are values -/
inductive Frame where
  | consL (t : Expr) | consR (h : Expr)
  | addL (b : Expr) | addR (a : Expr)
  | ltL (b : Expr) | ltR (a : Expr)
  | eqL (b : Expr) | eqR (a : Expr)
  | ite (a b : Expr)
  | seq (b : Expr)
  | indexL (i : Expr) | indexR (l : Expr)
  | appendL (b : Expr) | appendR (a : Expr)
  | assign (x : String) | init (x : String)
  | letIn (y : String) (t : Ty) (b : Expr)
  | call (f : String)
  | cast (t : Ty)
  | broadcast (f : String)

namespace Frame

def plug : Frame → Expr → Expr
  | consL t, e => .cons e t
  | consR h, e => .cons h e
  | addL b, e => .add e b
  | addR a, e => .add a e
  | ltL b, e => .lt e b
  | ltR a, e => .lt a e
  | eqL b, e => .eq e b
  | eqR a, e => .eq a e
  | ite a b, e => .ite e a b
  | seq b, e => .seq e b
  | indexL i, e => .index e i
  | indexR l, e => .index l e
  | appendL b, e => .append e b
  | appendR a, e => .append a e
  | assign x, e => .assign x e
  | init x, e => .init x e
  | letIn y t b, e => .letIn y t e b
  | call f, e => .call f e
  | cast t, e => .cast e t
  | broadcast f, e => .broadcast f e

/-- a right position needs the left operand evaluated -/
def ready : Frame → Bool
  | consR h | addR h | ltR h | eqR h | indexR h | appendR h => h.isValue
  | _ => true

end Frame

inductive Step (P : Program) : Expr × Store → Expr × Store → Prop where
  | frame {F : Frame} {e e' μ μ'} : F.ready = true → Step P (e, μ) (e', μ') → Step P (F.plug e, μ) (F.plug e', μ')
  | raise {F : Frame} {m μ} : F.ready = true → Step P (F.plug (.error m), μ) (.error m, μ)
  | tryStep {e e' h μ μ'} : Step P (e, μ) (e', μ') → Step P (.tryCatch e h, μ) (.tryCatch e' h, μ')
  | tryError {m h μ} : Step P (.tryCatch (.error m) h, μ) (h, μ)
  | tryValue {v h μ} : v.isValue = true → Step P (.tryCatch v h, μ) (v, μ)
  | readValue {x v μ} : μ x = some (.val v) → Step P (.glob x, μ) (v, μ)
  | readUnset {x μ} : μ x = some .unset → Step P (.glob x, μ) (.error "unset", μ)
  | readCharged {x b μ} : μ x = some (.charged b) → Step P (.glob x, μ) (b, μ)
  | add {a b μ} : a.isValue = true → b.isValue = true → Step P (.add a b, μ) (addValues a b, μ)
  | lt {a b μ} : a.isValue = true → b.isValue = true →
      Step P (.lt a b, μ) (.bool (decide (asNumber a < asNumber b)), μ)
  | eq {a b μ} : a.isValue = true → b.isValue = true → Step P (.eq a b, μ) (.bool (decide (a = b)), μ)
  | ite {v a b μ} : v.isValue = true → Step P (.ite v a b, μ) (if truthy v then a else b, μ)
  | loop {c b μ} : Step P (.loop c b, μ) (.ite c (.seq b (.loop c b)) .unit, μ)
  | seq {v b μ} : v.isValue = true → Step P (.seq v b, μ) (b, μ)
  | index {l i μ} : l.isValue = true → i.isValue = true →
      Step P (.index l i, μ) ((nth l ((asInt i).getD 0)).getD (.error "index out of range"), μ)
  | append {a b μ} : a.isValue = true → b.isValue = true → Step P (.append a b, μ) (appendValues a b, μ)
  | assign {x v μ} : v.isValue = true → Step P (.assign x v, μ) (v, μ.set x (.val v))
  | init {x v μ} : v.isValue = true → Step P (.init x v, μ) (v, μ.set x (.val v))
  | letIn {y t v b μ} : v.isValue = true → Step P (.letIn y t v b, μ) (b.subst y v, μ)
  | call {f v fn μ} : v.isValue = true → P.funs f = some fn →
      Step P (.call f v, μ) (.letIn fn.param fn.paramTy v fn.body, μ)
  | broadcastNil {f μ} : Step P (.broadcast f .nil, μ) (.nil, μ)
  | broadcastCons {f h t μ} : h.isValue = true → t.isValue = true →
      Step P (.broadcast f (.cons h t), μ) (.cons (.call f h) (.broadcast f t), μ)
  | cast {v t μ} : v.isValue = true → Step P (.cast v t, μ) (if fits v t then v else .error "type mismatch", μ)

/-- every declared name has a cell that fits its mode and type -/
def CellOk (P : Program) (m : Mode) (t : Ty) : Option Cell → Prop
  | some .unset => m ≠ .charged
  | some (.val v) => m ≠ .charged ∧ v.isValue = true ∧ ∃ tv, HasType P Ctx.empty v tv ∧ sub tv t = true
  | some (.charged b) => m = .charged ∧ ∃ tb, HasType P Ctx.empty b tb ∧ sub tb t = true
  | none => False

def StoreOk (P : Program) (μ : Store) : Prop := ∀ x m t, P.names x = some (m, t) → CellOk P m t (μ x)

end Warp
