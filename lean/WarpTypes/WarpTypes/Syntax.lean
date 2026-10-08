import WarpTypes.Ty

/-! Expressions of W0 (notes/type_theory.md). Main-level names live in the store (`glob`), parameters and lets are
substituted (`loc`). Lists are cons cells, as warp's GC `$Node` lists are. -/

namespace Warp

/-- how a main-level name may change: `x = 1` (var), `const x = 1`, `z := e` (charged: its body runs at each read, P71) -/
inductive Mode where
  | var | const | charged
  deriving DecidableEq, Repr

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
  deriving DecidableEq, Repr

namespace Expr

def isValue : Expr → Bool
  | bool _ | int _ | num _ | text _ | unit | nil => true
  | cons h t => h.isValue && t.isValue
  | _ => false

/-- replace the local `y` by the closed value `v` -/
def subst (e : Expr) (y : String) (v : Expr) : Expr :=
  match e with
  | loc z => if z = y then v else loc z
  | cons a b => cons (a.subst y v) (b.subst y v)
  | add a b => add (a.subst y v) (b.subst y v)
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
  | e => e

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

/-- the types of the locals in scope -/
abbrev Ctx := String → Option Ty

def Ctx.empty : Ctx := fun _ => none

def Ctx.set (Γ : Ctx) (y : String) (t : Ty) : Ctx := fun z => if z = y then some t else Γ z

end Warp
