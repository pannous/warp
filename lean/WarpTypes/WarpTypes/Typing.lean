import WarpTypes.Syntax

/-! The typing judgment `HasType P Γ e t`: syntax directed, one type per expression, every check an upper bound
(`sub te t`). Each rule names the warp code it stands for (notes/type_theory.md). -/

namespace Warp
open Ty

/-- the type a field read gives: from an error (`never`), `never`; a declared field its type; anything else is checked
when it runs (`any`) -/
def Program.readTy (P : Program) : Ty → String → Ty
  | .never, _ => .never
  | .cls p, f => (P.fieldTy p f).getD .any
  | _, _ => .any

/-- the type a field write takes: into an error (`never`), anything -/
def Program.writeTy (P : Program) : Ty → String → Option Ty
  | .never, _ => some .any
  | .cls p, f => P.fieldTy p f
  | _, _ => none

inductive HasType (P : Program) : Ctx → Expr → Ty → Prop where
  | bool {Γ b} : HasType P Γ (.bool b) .bool
  | int {Γ n} : HasType P Γ (.int n) .int
  | num {Γ n} : HasType P Γ (.num n) .number
  | text {Γ s} : HasType P Γ (.text s) .text
  | unit {Γ} : HasType P Γ .unit .unit
  | nil {Γ} : HasType P Γ .nil (.list .never)
  /-- inference.rs infer_list_type: the elements' join -/
  | cons {Γ h t a l e} : HasType P Γ h a → HasType P Γ t l → element l = some e →
      HasType P Γ (.cons h t) (.list (join a e))
  /-- analyzer Scope::lookup -/
  | glob {Γ x m t} : P.names x = some (m, t) → HasType P Γ (.glob x) t
  | loc {Γ y t} : Γ y = some t → HasType P Γ (.loc y) t
  /-- inference.rs arithmetic_kind; operands of other types raise when it runs (gradual: `HasType` means "cannot get
  stuck", the checker demands addable or numeric operands) -/
  | add {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.add a b) (plus ta tb)
  | arith {Γ op a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.arith op a b) (arithTy ta tb)
  | lt {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.lt a b) .bool
  | eq {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.eq a b) .bool
  /-- any condition (truthiness); inference.rs branches_kind -/
  | ite {Γ c a b tc ta tb} : HasType P Γ c tc → HasType P Γ a ta → HasType P Γ b tb →
      HasType P Γ (.ite c a b) (join ta tb)
  | loop {Γ c b tc tb} : HasType P Γ c tc → HasType P Γ b tb → HasType P Γ (.loop c b) .unit
  | seq {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.seq a b) tb
  /-- inference.rs element_kind -/
  | index {Γ l i tl ti} : HasType P Γ l tl → HasType P Γ i ti → HasType P Γ (.index l i) (elementTy tl)
  | append {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb →
      HasType P Γ (.append a b) (.list (join (listElem ta) (listElem tb)))
  /-- checks.rs check_assignment / check_declared_types; const (P130) and charged (P138) names are not assignable -/
  | assign {Γ x e t te} : P.names x = some (.var, t) → HasType P Γ e te → sub te t = true →
      HasType P Γ (.assign x e) t
  | init {Γ x e m t te} : P.names x = some (m, t) → m ≠ .charged → HasType P Γ e te → sub te t = true →
      HasType P Γ (.init x e) t
  | letIn {Γ y t e b te tb} : HasType P Γ e te → sub te t = true → HasType P (Γ.set y t) b tb →
      HasType P Γ (.letIn y t e b) tb
  /-- user_functions.rs P49, traits.rs admit -/
  | call {Γ f e fn te} : P.funs f = some fn → HasType P Γ e te → sub te fn.paramTy = true →
      HasType P Γ (.call f e) fn.result
  /-- inference.rs raises_error: the bottom kind -/
  | error {Γ m} : HasType P Γ (.error m) .never
  | tryCatch {Γ e h te th} : HasType P Γ e te → HasType P Γ h th → HasType P Γ (.tryCatch e h) (join te th)
  /-- a run-time checked cast: statically any source type, the alternatives' join -/
  | cast {Γ e ts te} : HasType P Γ e te → HasType P Γ (.cast e ts) (joinAll ts)
  /-- broadcasting: f : A → B over a list of A gives a list of B -/
  | broadcast {Γ f e fn te a} : P.funs f = some fn → HasType P Γ e te → element te = some a → sub a fn.paramTy = true →
      HasType P Γ (.broadcast f e) (.list fn.result)
  | ref {Γ a p} : HasType P Γ (.ref a p) (.cls p)
  | new {Γ p} : HasType P Γ (.new p) (.cls p)
  /-- field access: the field type of the static class (or of an ancestor) -/
  | get {Γ e f te} : HasType P Γ e te → HasType P Γ (.get e f) (P.readTy te f)
  /-- field write: fields are invariant, the value must fit the declared field type -/
  | set {Γ e f v te t tv} : HasType P Γ e te → P.writeTy te f = some t → HasType P Γ v tv → sub tv t = true →
      HasType P Γ (.set e f v) tv
  | isA {Γ e c te} : HasType P Γ e te → HasType P Γ (.isA e c) .bool

/-- every function body fits its declared result, given its parameter, and assigns only global names -/
def FunsOk (P : Program) : Prop :=
  ∀ f fn, P.funs f = some fn →
    (∃ tb, HasType P (Ctx.empty.set fn.param fn.paramTy) fn.body tb ∧ sub tb fn.result = true) ∧
    ∀ x ∈ fn.body.assigned, P.globals x = true

end Warp
