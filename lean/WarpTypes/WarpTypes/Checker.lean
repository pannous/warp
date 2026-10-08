import WarpTypes.Soundness

/-! The executable checker: `typeOf` computes the type `HasType` gives (`typeOf_sound`), and `Spec.check` elaborates a
whole program in source order and checks it, so an accepted program is safe (`check_safe`).
This is what the differential test runs on programs exported from warp (src/law/type_model.rs). -/

namespace Warp
open Ty Expr

/-- the checker reads a field only from an error, a class declaring it, or a dynamic value (P201: `s: Shape; s.r` is
a compile error, `s = Circle(…); s.r` is checked when it runs) -/
def strictRead (P : Program) : Ty → String → Bool
  | .never, _ | .any, _ => true
  | .cls p, f => (P.fieldTy p f).isSome
  | _, _ => false

def typeOf (P : Program) (Γ : Ctx) : Expr → Option Ty
  | .bool _ => some .bool
  | .int _ => some .int
  | .num _ => some .number
  | .text _ => some .text
  | .unit => some .unit
  | .nil => some (.list .never)
  | .cons h t =>
    match typeOf P Γ h, typeOf P Γ t with
    | some a, some l => (element l).map fun e => .list (join a e)
    | _, _ => none
  | .glob x => (P.names x).map Prod.snd
  | .loc y => Γ y
  | .add a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if addable ta && addable tb then some (plus ta tb) else none
    | _, _ => none
  | .arith _ a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if numeric ta && numeric tb then some (arithTy ta tb) else none
    | _, _ => none
  | .lt a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb => if numeric ta && numeric tb then some .bool else none
    | _, _ => none
  | .eq a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some _, some _ => some .bool
    | _, _ => none
  | .ite c a b =>
    match typeOf P Γ c, typeOf P Γ a, typeOf P Γ b with
    | some _, some ta, some tb => some (join ta tb)
    | _, _, _ => none
  | .loop c b =>
    match typeOf P Γ c, typeOf P Γ b with
    | some _, some _ => some .unit
    | _, _ => none
  | .seq a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some _, some tb => some tb
    | _, _ => none
  | .index l i =>
    match typeOf P Γ l, typeOf P Γ i with
    | some tl, some ti => if sub ti .int then element tl else none
    | _, _ => none
  | .append a b =>
    match typeOf P Γ a, typeOf P Γ b with
    | some ta, some tb =>
      match element ta, element tb with
      | some ea, some eb => some (.list (join ea eb))
      | _, _ => none
    | _, _ => none
  | .assign x e =>
    match P.names x, typeOf P Γ e with
    | some (.var, t), some te => if sub te t then some t else none
    | _, _ => none
  | .init x e =>
    match P.names x, typeOf P Γ e with
    | some (m, t), some te => if m != .charged && sub te t then some t else none
    | _, _ => none
  | .letIn y t e b =>
    match typeOf P Γ e with
    | some te => if sub te t then typeOf P (Γ.set y t) b else none
    | none => none
  | .call f e =>
    match P.funs f, typeOf P Γ e with
    | some fn, some te => if sub te fn.paramTy then some fn.result else none
    | _, _ => none
  | .error _ => some .never
  | .tryCatch e h =>
    match typeOf P Γ e, typeOf P Γ h with
    | some te, some th => some (join te th)
    | _, _ => none
  | .cast e ts => (typeOf P Γ e).bind fun te => if ts.any (consub te) then some (joinAll ts) else none
  | .broadcast f e =>
    match P.funs f, typeOf P Γ e with
    | some fn, some te =>
      match element te with
      | some a => if sub a fn.paramTy then some (.list fn.result) else none
      | none => none
    | _, _ => none
  | .ref _ p | .new p => some (.cls p)
  | .get e f => (typeOf P Γ e).bind fun te => if strictRead P te f then some (P.readTy te f) else none
  | .set e f v =>
    match typeOf P Γ e, typeOf P Γ v with
    | some te, some tv => (P.writeTy te f).bind fun t => if sub tv t then some tv else none
    | _, _ => none
  | .isA e _ => (typeOf P Γ e).map fun _ => .bool

theorem typeOf_sound {P : Program} : ∀ {e : Expr} {Γ t}, typeOf P Γ e = some t → HasType P Γ e t := by
  intro e
  induction e with
  | bool | int | num | text | unit | nil => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | cons h tl ih1 ih2 =>
    intro Γ t hs; simp only [typeOf] at hs; split at hs
    · rename_i a l ha hl
      simp only [Option.map_eq_some_iff] at hs
      obtain ⟨e, he, rfl⟩ := hs
      exact .cons (ih1 ha) (ih2 hl) he
    · cases hs
  | glob x =>
    intro Γ t h; simp only [typeOf, Option.map_eq_some_iff] at h
    obtain ⟨⟨m, t'⟩, hx, rfl⟩ := h
    exact .glob hx
  | loc y => intro Γ t h; exact .loc h
  | add a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .add (ih1 ha) (ih2 hb)
    · cases h
  | arith op a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .arith (ih1 ha) (ih2 hb)
    · cases h
  | lt a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · cases h; exact .lt (ih1 ha) (ih2 hb)
    · cases h
  | eq a b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i ha hb; cases h; exact .eq (ih1 ha) (ih2 hb)
    · cases h
  | ite c a b ih0 ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i hc ha hb; cases h; exact .ite (ih0 hc) (ih1 ha) (ih2 hb)
    · cases h
  | loop c b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i hc hb; cases h; exact .loop (ih1 hc) (ih2 hb)
    · cases h
  | seq a b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i ha hb; cases h; exact .seq (ih1 ha) (ih2 hb)
    · cases h
  | index l i ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hl hi hs; exact .index (ih1 hl) h (ih2 hi) hs
      · cases h
    · cases h
  | append a b ih1 ih2 =>
    intro Γ t h
    cases ha : typeOf P Γ a <;> cases hb : typeOf P Γ b <;> simp only [typeOf, ha, hb] at h <;> try cases h
    split at h
    · rename_i hea heb; cases h; exact .append (ih1 ha) hea (ih2 hb) heb
    · cases h
  | assign x e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hx he hs; cases h; exact .assign hx (ih he) hs
      · cases h
    · cases h
  | init x e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hx he hc; cases h; simp at hc; exact .init hx hc.1 (ih he) hc.2
      · cases h
    · cases h
  | letIn y t' e b ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i he hs; exact .letIn (ih1 he) hs (ih2 h)
      · cases h
    · cases h
  | call f e ih =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · split at h
      · rename_i hf he hs; cases h; exact .call hf (ih he) hs
      · cases h
    · cases h
  | error => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | tryCatch e h' ih1 ih2 =>
    intro Γ t h; simp only [typeOf] at h; split at h
    · rename_i he hh; cases h; exact .tryCatch (ih1 he) (ih2 hh)
    · cases h
  | cast e ts ih =>
    intro Γ t h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨_, he, hc⟩ := h
    split at hc
    · cases hc; exact .cast (ih he)
    · cases hc
  | broadcast f e ih =>
    intro Γ t h
    cases hf : P.funs f <;> cases he : typeOf P Γ e <;> simp only [typeOf, hf, he] at h <;> try cases h
    split at h
    · split at h
      · rename_i hel hs; cases h; exact .broadcast hf (ih he) hel hs
      · cases h
    · cases h
  | ref | new => intro Γ t h; simp [typeOf] at h; subst h; constructor
  | get e f ih =>
    intro Γ t h; simp only [typeOf, Option.bind_eq_some_iff] at h
    obtain ⟨te, he, hr⟩ := h
    split at hr
    · cases hr; exact .get (ih he)
    · cases hr
  | set e f v ih1 ih2 =>
    intro Γ t h
    cases he : typeOf P Γ e <;> cases hv : typeOf P Γ v <;> simp only [typeOf, he, hv] at h <;> try cases h
    simp only [Option.bind_eq_some_iff] at h
    obtain ⟨tf, hw, h⟩ := h
    split at h
    · rename_i hs; cases h; exact .set (ih1 he) hw (ih2 hv) hs
    · cases h
  | isA e c ih =>
    intro Γ t h; simp only [typeOf, Option.map_eq_some_iff] at h
    obtain ⟨_, he, rfl⟩ := h
    exact .isA (ih he)

/-! ## Whole programs, as warp writes them: top-level items in source order -/

/-- a main-level name and how it was declared -/
structure Decl where
  name : String
  mode : Mode
  type : Ty
  /-- the body of a charged name -/
  charged : Option Expr := none
  isGlobal : Bool := false
  /-- declared with a type (`x: int = 1`), not inferred from its values -/
  annotated : Bool := false

/-- a checked program: declarations, functions and the main expression, after elaboration -/
structure Spec where
  decls : List Decl
  funs : List (String × Fn)
  main : Expr
  /-- each class with the fields it declares itself -/
  classes : List (String × List (String × Ty)) := []

def Spec.program (s : Spec) : Program where
  names x := (s.decls.find? (·.name == x)).map fun d => (d.mode, d.type)
  funs f := (s.funs.find? (·.1 == f)).map (·.2)
  globals x := (s.decls.find? (·.name == x)).any (·.isGlobal)
  fields c f := (s.classes.find? (·.1 == c)).bind fun (_, fields) => fields.lookup f

/-- the store before main runs: charged names hold their body, the others are unset -/
def Spec.store (s : Spec) : Store where
  vars x := (s.decls.find? (·.name == x)).map fun d =>
    match d.charged with
    | some b => .charged b
    | none => .unset

def Decl.ok (P : Program) (d : Decl) : Bool :=
  match d.mode, d.charged with
  | .charged, some b => (typeOf P Ctx.empty b).any (sub · d.type)
  | .charged, none => false
  | _, some _ => false
  | _, none => true

def Fn.ok (P : Program) (fn : Fn) : Bool :=
  (typeOf P (Ctx.empty.set fn.param fn.paramTy) fn.body).any (sub · fn.result) &&
    fn.body.assigned.all P.globals

def Spec.check (s : Spec) : Option Ty :=
  let P := s.program
  if s.decls.all (Decl.ok P) && s.funs.all (fun f => f.2.ok P) then typeOf P Ctx.empty s.main else none

theorem Spec.check_sound {s : Spec} {t} (h : s.check = some t) :
    FunsOk s.program ∧ StoreOk s.program s.store ∧ HasType s.program Ctx.empty s.main t := by
  unfold Spec.check at h
  dsimp only at h
  split at h
  · rename_i hok
    simp only [Bool.and_eq_true, List.all_eq_true] at hok
    obtain ⟨hdecls, hfuns⟩ := hok
    refine ⟨?_, ⟨?_, fun a o ho => by simp [Spec.store] at ho⟩, typeOf_sound h⟩
    · intro f fn hf
      simp only [Spec.program, Option.map_eq_some_iff] at hf
      obtain ⟨⟨g, fn'⟩, hfind, rfl⟩ := hf
      have hfn := hfuns _ (List.mem_of_find?_eq_some hfind)
      simp only [Fn.ok, Bool.and_eq_true, Option.any_eq_true, List.all_eq_true] at hfn
      obtain ⟨⟨tb, htb, hs⟩, hg⟩ := hfn
      exact ⟨⟨tb, typeOf_sound htb, hs⟩, hg⟩
    · intro x m t' hx
      simp only [Spec.program, Option.map_eq_some_iff] at hx
      obtain ⟨d, hfind, hd⟩ := hx
      cases hd
      have hdok := hdecls _ (List.mem_of_find?_eq_some hfind)
      simp only [Spec.store, hfind, Option.map_some]
      unfold Decl.ok at hdok
      split at hdok
      · rename_i h1 h2
        simp only [Option.any_eq_true] at hdok
        obtain ⟨tb, htb, hs⟩ := hdok
        simp only [CellOk, h1, h2]
        first | exact ⟨tb, typeOf_sound htb, hs⟩ | exact ⟨trivial, tb, typeOf_sound htb, hs⟩
      · cases hdok
      · cases hdok
      · rename_i h1 h2
        simp only [CellOk, h1]
        first | exact h1 | exact h2
  · cases h

/-- **An accepted program is safe**: every state it reaches is well typed, and done, failed, or able to step -/
theorem check_safe {s : Spec} {t} (h : s.check = some t) {s'} (hs : Steps s.program (s.main, s.store) s') :
    (∃ t', HasType s.program Ctx.empty s'.1 t' ∧ sub t' t = true) ∧ StoreOk s.program s'.2 ∧
      Progresses s.program s'.2 s'.1 :=
  let ⟨hP, hμ, ht⟩ := Spec.check_sound h
  safety hP hs ht hμ

/-! ## Elaboration: warp's source order, the types warp infers -/

/-- one top-level item of a warp program -/
inductive Item where
  /-- `x = e`, `x: T = e`, `const x = e`, `global x = e`: the first binding of a main-level name -/
  | bind (name : String) (mode : Mode) (annotation : Option Ty) (value : Expr) (isGlobal : Bool)
  /-- `z := e` -/
  | charged (name : String) (body : Expr)
  /-- `f(y: T) := body` (warp hoists functions); an unannotated parameter (`none`) takes the values it is given -/
  | function (name param : String) (paramTy : Option Ty) (body : Expr)
  /-- `class C : Parent { f: T … }` and the variants of `type Color = red | rgb(r: int, …)`: the fields C declares
  itself, an unannotated one (`none`) typed by the values written to it; its ancestor chain is written into the
  types that name it -/
  | classDef (name : String) (fields : List (String × Option Ty))
  /-- any other statement -/
  | statement (e : Expr)

/-- a name given no annotation holds the type of its first value; a list stays a list of anything (list-element-types:
an undeclared list may take items of any type) -/
def widen : Ty → Ty
  | .list _ => .list .any
  | t => t

/-- the values a main-level name is given -/
def valuesOf (x : String) : Expr → List Expr
  | .assign y e | .init y e => (if y = x then [e] else []) ++ valuesOf x e
  | .cons a b | .add a b | .arith _ a b | .lt a b | .eq a b | .loop a b | .seq a b | .index a b | .append a b
  | .tryCatch a b => valuesOf x a ++ valuesOf x b
  | .ite c a b => valuesOf x c ++ valuesOf x a ++ valuesOf x b
  | .letIn _ _ e b => valuesOf x e ++ valuesOf x b
  | .set a _ b => valuesOf x a ++ valuesOf x b
  | .call _ e | .cast e _ | .broadcast _ e | .get e _ | .isA e _ => valuesOf x e
  | _ => []

/-- P45: a name declared by its first value widens over the numbers it is given (`x = 1; x = 2.5` is a number);
a value of another kind (`x = 1; x = "a"`) leaves the type, so its assignment is rejected -/
def widenOver (s : Spec) (d : Decl) : Decl :=
  if d.annotated || d.mode == .charged || d.type == .bool then d else
  let widened := (valuesOf d.name s.main).foldl (init := d.type) fun t value =>
    match typeOf s.program Ctx.empty value with
    | some tv => if sub (join t tv) .number then join t tv else t
    | none => t
  { d with type := widened }

/-- warp decides broadcasting at compile time: a call whose argument is a list of what the function takes -/
def resolveCalls (P : Program) (Γ : Ctx) : Expr → Expr
  | .call f e =>
    let e := resolveCalls P Γ e
    match P.funs f, typeOf P Γ e with
    | some fn, some te =>
      if sub te fn.paramTy then .call f e
      else if (element te).any (sub · fn.paramTy) then .broadcast f e else .call f e
    | _, _ => .call f e
  | .cons a b => .cons (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .add a b => .add (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .arith op a b => .arith op (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .lt a b => .lt (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .eq a b => .eq (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .ite c a b => .ite (resolveCalls P Γ c) (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .loop c b => .loop (resolveCalls P Γ c) (resolveCalls P Γ b)
  | .seq a b => .seq (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .index a b => .index (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .append a b => .append (resolveCalls P Γ a) (resolveCalls P Γ b)
  | .assign x e => .assign x (resolveCalls P Γ e)
  | .init x e => .init x (resolveCalls P Γ e)
  | .letIn y t e b => .letIn y t (resolveCalls P Γ e) (resolveCalls P (Γ.set y t) b)
  | .tryCatch e h => .tryCatch (resolveCalls P Γ e) (resolveCalls P Γ h)
  | .cast e ts => .cast (resolveCalls P Γ e) ts
  | .get e f => .get (resolveCalls P Γ e) f
  | .set e f v => .set (resolveCalls P Γ e) f (resolveCalls P Γ v)
  | .isA e c => .isA (resolveCalls P Γ e) c
  | e => e

/-- the function with a provisional result; its calls resolved (a recursive call sees that result) -/
def draftFunction (s : Spec) (name param : String) (paramTy result : Ty) (body : Expr) : Fn :=
  let draft : Fn := { param, paramTy, result, body }
  let s' := { s with funs := (name, draft) :: s.funs }
  { draft with body := resolveCalls s'.program (Ctx.empty.set param paramTy) body }

/-- a function's result: iterate from `never` so a recursive call sees its own result type -/
def inferFunction (s : Spec) (name param : String) (paramTy : Ty) (body : Expr) : Nat → Ty → Fn
  | 0, r => draftFunction s name param paramTy r body
  | n + 1, r =>
    let fn := draftFunction s name param paramTy r body
    let s' := { s with funs := (name, fn) :: s.funs }
    match typeOf s'.program (Ctx.empty.set param paramTy) fn.body with
    | some r' => if r' == r then fn else inferFunction s name param paramTy body n (join r r')
    | none => fn

/-- statements in order; the last one gives the program's value -/
def sequence : List Expr → Expr
  | [] => .unit
  | [e] => e
  | e :: rest => .seq e (sequence rest)

def RESULT_ROUNDS : Nat := 4

/-- the types guessed for unannotated parameters (by function) and fields (by class and field) -/
structure Guesses where
  params : List (String × Ty) := []
  fields : List ((String × String) × Ty) := []

/-- a value given to an unannotated place: an argument of f, or a write to field f of a class chain -/
inductive Observation where
  | argument (f : String) (t : Ty)
  | field (path : List String) (f : String) (t : Ty)

/-- the arguments and field writes of an expression, with their types -/
def observe (P : Program) (Γ : Ctx) : Expr → List Observation
  | .call f e => (typeOf P Γ e).toList.map (.argument f) ++ observe P Γ e
  | .set o f v =>
    (match typeOf P Γ o, typeOf P Γ v with
      | some (.cls p), some tv => [.field p f tv]
      | _, _ => []) ++ observe P Γ o ++ observe P Γ v
  | .letIn y t e b => observe P Γ e ++ observe P (Γ.set y t) b
  | .cons a b | .add a b | .arith _ a b | .lt a b | .eq a b | .loop a b | .seq a b | .index a b | .append a b
  | .tryCatch a b => observe P Γ a ++ observe P Γ b
  | .ite c a b => observe P Γ c ++ observe P Γ a ++ observe P Γ b
  | .assign _ e | .init _ e | .cast e _ | .broadcast _ e | .get e _ | .isA e _ => observe P Γ e
  | _ => []

/-- the items with every unannotated place typed by its guess; one never given a value holds anything -/
def Guesses.fill (g : Guesses) : Item → Item
  | .function f y none b => .function f y (some (((g.params.lookup f).filter (· != .never)).getD .any)) b
  | .classDef c fields =>
    .classDef c (fields.map fun (f, t) => (f, some (t.getD (((g.fields.lookup (c, f)).filter (· != .never)).getD .any))))
  | item => item

def Guesses.add (g : Guesses) (classes : List (String × List (String × Option Ty))) : Observation → Guesses
  | .argument f t => { g with params := (f, join ((g.params.lookup f).getD .never) t) :: g.params.filter (·.1 != f) }
  | .field p f t =>
    -- the class of the chain that declares f
    match p.find? fun c => ((classes.lookup c).getD []).any (·.1 == f) with
    | some c =>
      { g with fields := ((c, f), join ((g.fields.lookup (c, f)).getD .never) t) :: g.fields.filter (·.1 != (c, f)) }
    | none => g

def elaborateTyped (items : List Item) : Spec :=
  let functions := items.filterMap fun | .function f y t b => some (f, y, t.getD .any, b) | _ => none
  let rest := items.filter fun | .function .. | .classDef .. => false | _ => true
  let classes := items.filterMap fun | .classDef c fields => some (c, fields.map fun (f, t) => (f, t.getD .any)) | _ => none
  let withFunctions := functions.foldl (init := ({ decls := [], funs := [], main := .unit, classes } : Spec))
    fun s (f, y, t, b) =>
      { s with funs := s.funs ++ [(f, inferFunction s f y t b RESULT_ROUNDS .never)] }
  let step (s : Spec) (statements : List Expr) : Item → Spec × List Expr
    | .bind x m annotation value g =>
      let value := resolveCalls s.program Ctx.empty value
      let t := annotation.getD (widen ((typeOf s.program Ctx.empty value).getD .any))
      ({ s with decls := s.decls ++ [{ name := x, mode := m, type := t, isGlobal := g, annotated := annotation.isSome }] },
        statements ++ [.init x value])
    | .charged x body =>
      let body := resolveCalls s.program Ctx.empty body
      let t := (typeOf s.program Ctx.empty body).getD .any
      ({ s with decls := s.decls ++ [{ name := x, mode := .charged, type := t, charged := some body }] }, statements)
    | .statement e => (s, statements ++ [resolveCalls s.program Ctx.empty e])
    | .function .. | .classDef .. => (s, statements)
  let (s, statements) := rest.foldl (init := (withFunctions, [])) fun (s, st) item => step s st item
  let s := { s with main := sequence statements }
  (List.range RESULT_ROUNDS).foldl (init := s) fun s _ => { s with decls := s.decls.map (widenOver s) }

/-- one round of inference: elaborate with the guesses so far, then join in the values the program gives -/
def Guesses.refine (items : List Item) (g : Guesses) : Guesses :=
  let s := elaborateTyped (items.map g.fill)
  let classes := items.filterMap fun | .classDef c fields => some (c, fields) | _ => none
  let bodies := s.funs.map fun (_, fn) => observe s.program (Ctx.empty.set fn.param fn.paramTy) fn.body
  (observe s.program Ctx.empty s.main ++ bodies.flatten).foldl (fun g o => g.add classes o) g

/-- warp's elaboration: unannotated parameters and fields are typed by the values the program gives them (P173: a
parameter given several kinds holds anything), then the program is elaborated in source order -/
def elaborate (items : List Item) : Spec :=
  let guesses := (List.range RESULT_ROUNDS).foldl (init := ({} : Guesses)) fun g _ => g.refine items
  elaborateTyped (items.map guesses.fill)

/-- the verdict line the differential test reads: `ok <type>` or `rejected` -/
def verdict (items : List Item) : String :=
  match (elaborate items).check with
  | some t => s!"ok {t.name}"
  | none => "rejected"

end Warp
