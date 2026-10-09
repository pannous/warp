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
  | arith {Γ op a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.arith op a b) (op.ty ta tb)
  | lt {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.lt a b) .bool
  | eq {Γ s a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.eq s a b) .bool
  /-- any condition (truthiness); inference.rs branches_kind -/
  | ite {Γ c a b tc ta tb} : HasType P Γ c tc → HasType P Γ a ta → HasType P Γ b tb →
      HasType P Γ (.ite c a b) (join ta tb)
  /-- P55: the last body value, or ø when the body never ran (a program's `last` is ø) -/
  | loop {Γ c b d tc tb td} : HasType P Γ c tc → HasType P Γ b tb → HasType P Γ d td →
      HasType P Γ (.loop c b d) (join tb (join td .unit))
  | seq {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.seq a b) tb
  /-- inference.rs element_kind -/
  | index {Γ l i tl ti} : HasType P Γ l tl → HasType P Γ i ti → HasType P Γ (.index l i) (elementTy tl)
  /-- the parameter takes anything (warp's lambdas are unannotated) -/
  | lam {Γ y b tb} : HasType P (Γ.set y .any) b tb → HasType P Γ (.lam y b) (.fn tb)
  | clo {Γ y b tb} : HasType P (Ctx.empty.set y .any) b tb → HasType P Γ (.clo y b) (.fn tb)
  | app {Γ f a tf ta} : HasType P Γ f tf → HasType P Γ a ta → HasType P Γ (.app f a) (resultTy tf)
  /-- the items are ints (number bounds fail when it runs: W0 keeps no float values) -/
  | range {Γ a b ta tb} : HasType P Γ a ta → HasType P Γ b tb → HasType P Γ (.range a b) (.list (arithTy ta tb))
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
  /-- a conversion `e as t`: statically any source type, t (a value that does not convert is an error when it runs) -/
  | conv {Γ e t te} : HasType P Γ e te → HasType P Γ (.conv e t) t
  /-- broadcasting: f : A → B over a list of A gives a list of B -/
  | broadcast {Γ f e fn te a} : P.funs f = some fn → HasType P Γ e te → element te = some a → sub a fn.paramTy = true →
      HasType P Γ (.broadcast f e) (.list fn.result)
  | ref {Γ a p} : HasType P Γ (.ref a p) (.cls p)
  /-- a shared list is typed by the element type it was made with: its items fit that type, which every write checks,
  so a covariant view `list τ` of it (t ≤ τ) reads items that fit τ -/
  | lref {Γ a t} : HasType P Γ (.lref a t) (.list t)
  /-- the items are checked against t when the list is made -/
  | share {Γ e t te} : HasType P Γ e te → HasType P Γ (.share e t) (.list t)
  /-- the item is checked against the list's own element type when it runs -/
  | push {Γ l v tl tv} : HasType P Γ l tl → HasType P Γ v tv → HasType P Γ (.push l v) tl
  | setAt {Γ l i v tl ti tv} : HasType P Γ l tl → HasType P Γ i ti → HasType P Γ v tv → HasType P Γ (.setAt l i v) tv
  | new {Γ p} : HasType P Γ (.new p) (.cls p)
  /-- field access: the field type of the static class (or of an ancestor) -/
  | get {Γ e f te} : HasType P Γ e te → HasType P Γ (.get e f) (P.readTy te f)
  /-- field write: fields are invariant, the value must fit the declared field type -/
  | set {Γ e f v te t tv} : HasType P Γ e te → P.writeTy te f = some t → HasType P Γ v tv → sub tv t = true →
      HasType P Γ (.set e f v) tv
  | isA {Γ e c te} : HasType P Γ e te → HasType P Γ (.isA e c) .bool
  /-- a handler gives at most its event's result type; the payload is dynamic data (`event.level`); the block gives
  its body's value or what a handler's `break` gives -/
  | handle {Γ ev h b th tb R} : P.effects ev = some R → HasType P (Γ.set eventLocal .any) h th → sub th R = true →
      HasType P Γ b tb → HasType P Γ (.handle ev h b) (join tb (P.aborts ev))
  /-- an emit gives its event's result type, or ø when nothing handles it -/
  | emit {Γ ev e te R} : P.effects ev = some R → HasType P Γ e te → HasType P Γ (.emit ev e) (join R .unit)
  | scope {Γ k e te} : HasType P Γ e te → HasType P Γ (.scope k e) te
  /-- `break v` does not return: the bottom type; v goes to a block of ev -/
  | abort {Γ ev k e te} : HasType P Γ e te → sub te (P.aborts ev) = true → HasType P Γ (.abort ev k e) .never
  /-- the loop variable holds the list's items or the text's one-character texts (anything when the type is not
  known: a value of another type fails when it runs) -/
  | forIn {Γ y l b d tl T tb td} : HasType P Γ l tl → sub (elementTy tl) T = true → HasType P (Γ.set y T) b tb →
      HasType P Γ d td → HasType P Γ (.forIn y l b d) (join tb (join td .unit))

/-- a handler of ev, closed but for the payload, gives at most ev's result type -/
def HandlerOk (P : Program) (ev : String) (h : Expr) : Prop :=
  ∃ R th, P.effects ev = some R ∧ HasType P (Ctx.empty.set eventLocal .any) h th ∧ sub th R = true

/-- every function body fits its declared result, given its parameter, and assigns only global names; every
program-wide handler fits its event -/
def ProgramOk (P : Program) : Prop :=
  (∀ f fn, P.funs f = some fn →
    (∃ tb, HasType P (Ctx.empty.set fn.param fn.paramTy) fn.body tb ∧ sub tb fn.result = true) ∧
    ∀ x ∈ fn.body.assigned, P.globals x = true) ∧
  ∀ ev h, P.handlers ev = some h → HandlerOk P ev h

end Warp
