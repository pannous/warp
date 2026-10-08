import WarpTypes.Semantics

/-! Lemmas for soundness: values are closed, a smaller context gives smaller types (narrowing), substitution of a
value keeps the type, and the value operations (`+`, `#`, `++`) give values of fitting types. -/

namespace Warp
open Ty Expr

variable {P : Program}

theorem value_ctx {Γ v t} (h : HasType P Γ v t) (hv : v.isValue = true) (Γ' : Ctx) : HasType P Γ' v t := by
  induction h with
  | bool => exact .bool
  | int => exact .int
  | num => exact .num
  | text => exact .text
  | unit => exact .unit
  | nil => exact .nil
  | cons _ _ he ih1 ih2 => simp [isValue] at hv; exact .cons (ih1 hv.1) (ih2 hv.2) he
  | _ => simp [isValue] at hv

/-- a value's type is a scalar or a list type, never `never` -/
theorem value_list {Γ v t e} (h : HasType P Γ v t) (hv : v.isValue = true) (he : element t = some e) :
    t = .list e := by
  cases h <;> simp_all [isValue, element]

theorem list_value {Γ v a} (h : HasType P Γ v (.list a)) (hv : v.isValue = true) :
    v = .nil ∨ ∃ hd tl, v = .cons hd tl ∧ hd.isValue = true ∧ tl.isValue = true := by
  cases v <;> simp [isValue] at hv ⊢ <;> first | exact hv | exact ⟨_, _, ⟨rfl, rfl⟩, hv⟩ | cases h

theorem number_value {Γ v t} (h : HasType P Γ v t) (hv : v.isValue = true) (hs : sub t .number = true) :
    (asInt v ≠ none ∧ sub t .int = true) ∨ (asInt v = none ∧ t = .number) := by
  cases h <;> simp_all [isValue, asInt, sub]

theorem add_typed {Γ a b ta tb} (ha : HasType P Γ a ta) (hb : HasType P Γ b tb) (va : a.isValue = true)
    (vb : b.isValue = true) (sa : addable ta = true) (sb : addable tb = true) :
    ∃ t', HasType P Γ (addValues a b) t' ∧ sub t' (plus ta tb) = true := by
  cases ha <;> cases hb <;> simp_all [isValue, addValues, isText, asInt, asNumber, plus, arith, addable, sub] <;>
    first | exact ⟨_, .int, by decide⟩ | exact ⟨_, .num, by decide⟩ | exact ⟨_, .text, by decide⟩

theorem nth_typed {Γ} : ∀ {l : Expr} (i : Int) {tl e v}, l.isValue = true → HasType P Γ l tl → element tl = some e →
    nth l i = some v → ∃ tv, HasType P Γ v tv ∧ sub tv e = true := by
  intro l
  induction l with
  | cons h t _ iht =>
    intro i tl e v hv hl he hn
    simp [isValue] at hv
    cases hl with
    | cons hh ht hel =>
      simp [element] at he; subst he
      simp [nth] at hn
      split at hn
      · cases hn; exact ⟨_, hh, join_upper_left _ _⟩
      · obtain ⟨tv, htv, hs⟩ := iht (i - 1) hv.2 ht hel hn
        exact ⟨tv, htv, sub_trans hs (join_upper_right _ _)⟩
  | _ => intro i tl e v _ _ _ hn; simp [nth] at hn

theorem append_typed {Γ b tb eb} (hb : HasType P Γ b tb) (vb : b.isValue = true) (eb' : element tb = some eb) :
    ∀ {a : Expr} {ta ea}, a.isValue = true → HasType P Γ a ta → element ta = some ea →
    ∃ t', HasType P Γ (appendValues a b) t' ∧ sub t' (.list (join ea eb)) = true := by
  intro a
  induction a with
  | nil =>
    intro ta ea _ ha hea
    cases ha; simp [element] at hea; subst hea
    rw [value_list hb vb eb'] at hb
    exact ⟨_, hb, by simp [join, sub_refl]⟩
  | cons h t _ iht =>
    intro ta ea va ha hea
    simp [isValue] at va
    cases ha with
    | cons hh ht hel =>
      simp [element] at hea; subst hea
      obtain ⟨t', ht', hs⟩ := iht va.2 ht hel
      obtain ⟨e'', he'', hs''⟩ := element_mono (e := join _ eb) hs rfl
      refine ⟨_, .cons hh ht' he'', ?_⟩
      simp only [sub_list]
      apply join_least
      · exact sub_trans (join_upper_left _ _) (join_upper_left _ _)
      · exact sub_trans hs'' (join_least (sub_trans (join_upper_right _ _) (join_upper_left _ _)) (join_upper_right _ _))
  | _ => intro ta ea va ha hea; cases ha <;> simp_all [isValue, element]

theorem valueType_typed {Γ} : ∀ {v : Expr} {t}, valueType v = some t → HasType P Γ v t := by
  intro v
  induction v with
  | cons h tl ih1 ih2 =>
    intro t hv
    simp only [valueType] at hv
    split at hv
    · rename_i a l ha hl
      split at hv
      · rename_i e he; cases hv; exact .cons (ih1 ha) (ih2 hl) he
      · cases hv
    · cases hv
  | _ => intro t hv; simp [valueType] at hv <;> subst hv <;> constructor

theorem fits_typed {Γ v t} (h : fits v t = true) : ∃ tv, HasType P Γ v tv ∧ sub tv t = true := by
  unfold fits at h
  split at h
  · exact ⟨_, valueType_typed (by assumption), h⟩
  · cases h

/-- Γ' gives every local of Γ a smaller type -/
def CtxSub (Γ' Γ : Ctx) : Prop := ∀ z t, Γ z = some t → ∃ t', Γ' z = some t' ∧ sub t' t = true

theorem CtxSub.set {Γ' Γ : Ctx} (h : CtxSub Γ' Γ) (y : String) (t : Ty) : CtxSub (Γ'.set y t) (Γ.set y t) := by
  intro z tz hz
  simp only [Ctx.set] at hz ⊢
  split at hz
  · cases hz; exact ⟨_, by simp [*], sub_refl _⟩
  · simp [*]; exact h z tz hz

/-- narrowing: smaller local types give a smaller type -/
theorem narrow {Γ e t} (h : HasType P Γ e t) : ∀ {Γ'}, CtxSub Γ' Γ → ∃ t', HasType P Γ' e t' ∧ sub t' t = true := by
  induction h with
  | bool => intros; exact ⟨_, .bool, sub_refl _⟩
  | int => intros; exact ⟨_, .int, sub_refl _⟩
  | num => intros; exact ⟨_, .num, sub_refl _⟩
  | text => intros; exact ⟨_, .text, sub_refl _⟩
  | unit => intros; exact ⟨_, .unit, sub_refl _⟩
  | nil => intros; exact ⟨_, .nil, sub_refl _⟩
  | cons _ _ he ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨l', h2, s2⟩ := ih2 hs
    obtain ⟨e', he', se⟩ := element_mono s2 he
    exact ⟨_, .cons h1 h2 he', by simpa using join_mono s1 se⟩
  | glob hx => intros; exact ⟨_, .glob hx, sub_refl _⟩
  | loc hy =>
    intro Γ' hs
    obtain ⟨t', h', s'⟩ := hs _ _ hy
    exact ⟨t', .loc h', s'⟩
  | add _ _ sa sb ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .add h1 h2 (addable_mono s1 sa) (addable_mono s2 sb), plus_mono s1 s2 sa sb⟩
  | lt _ _ sa sb ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .lt h1 h2 (sub_trans s1 sa) (sub_trans s2 sb), sub_refl _⟩
  | eq _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, _⟩ := ih1 hs
    obtain ⟨b', h2, _⟩ := ih2 hs
    exact ⟨_, .eq h1 h2, sub_refl _⟩
  | ite _ _ _ ih0 ih1 ih2 =>
    intro Γ' hs
    obtain ⟨c', h0, _⟩ := ih0 hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .ite h0 h1 h2, join_mono s1 s2⟩
  | loop _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih1 hs
    obtain ⟨_, h2, _⟩ := ih2 hs
    exact ⟨_, .loop h1 h2, sub_refl _⟩
  | seq _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .seq h1 h2, s2⟩
  | index _ he _ si ih1 ih2 =>
    intro Γ' hs
    obtain ⟨l', h1, s1⟩ := ih1 hs
    obtain ⟨i', h2, s2⟩ := ih2 hs
    obtain ⟨e', he', se⟩ := element_mono s1 he
    exact ⟨_, .index h1 he' h2 (sub_trans s2 si), se⟩
  | append _ hea _ heb ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    obtain ⟨ea', hea', sa⟩ := element_mono s1 hea
    obtain ⟨eb', heb', sb⟩ := element_mono s2 heb
    exact ⟨_, .append h1 hea' h2 heb', by simpa using join_mono sa sb⟩
  | assign hx _ st ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .assign hx h1 (sub_trans s1 st), sub_refl _⟩
  | init hx hm _ st ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .init hx hm h1 (sub_trans s1 st), sub_refl _⟩
  | letIn _ st _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 (hs.set _ _)
    exact ⟨_, .letIn h1 (sub_trans s1 st) h2, s2⟩
  | call hf _ st ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .call hf h1 (sub_trans s1 st), sub_refl _⟩
  | error => intros; exact ⟨_, .error, sub_refl _⟩
  | tryCatch _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .tryCatch h1 h2, join_mono s1 s2⟩
  | cast _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih hs
    exact ⟨_, .cast h1, sub_refl _⟩
  | broadcast hf _ he ha ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    obtain ⟨a', he', sa⟩ := element_mono s1 he
    exact ⟨_, .broadcast hf h1 he' (sub_trans sa ha), sub_refl _⟩

theorem Ctx.set_same (Γ : Ctx) (y : String) (a b : Ty) : (Γ.set y a).set y b = Γ.set y b := by
  funext z; simp only [Ctx.set]; split <;> simp_all

theorem Ctx.set_comm (Γ : Ctx) {y z : String} (hne : z ≠ y) (a b : Ty) :
    (Γ.set y a).set z b = (Γ.set z b).set y a := by
  funext w; simp only [Ctx.set]; split <;> split <;> simp_all

/-- substituting a value of type tv for a local of type tv keeps the type -/
theorem subst_typed {Γ0 e t} (h : HasType P Γ0 e t) :
    ∀ {Γ y tv v}, Γ0 = Γ.set y tv → v.isValue = true → HasType P Ctx.empty v tv →
    HasType P Γ (e.subst y v) t := by
  induction h with
  | bool => intros; simp only [Expr.subst]; exact .bool
  | int => intros; simp only [Expr.subst]; exact .int
  | num => intros; simp only [Expr.subst]; exact .num
  | text => intros; simp only [Expr.subst]; exact .text
  | unit => intros; simp only [Expr.subst]; exact .unit
  | nil => intros; simp only [Expr.subst]; exact .nil
  | cons _ _ he ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .cons (ih1 hΓ hv htv) (ih2 hΓ hv htv) he
  | glob hx => intros; simp only [Expr.subst]; exact .glob hx
  | loc hz =>
    rename_i z _
    intro Γ y tv v hΓ hv htv
    subst hΓ
    simp only [Expr.subst]
    simp only [Ctx.set] at hz
    split at hz
    · cases hz; simp [*]; exact value_ctx htv hv _
    · simp [*]; exact .loc hz
  | add _ _ sa sb ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .add (ih1 hΓ hv htv) (ih2 hΓ hv htv) sa sb
  | lt _ _ sa sb ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .lt (ih1 hΓ hv htv) (ih2 hΓ hv htv) sa sb
  | eq _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .eq (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | ite _ _ _ ih0 ih1 ih2 =>
    intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .ite (ih0 hΓ hv htv) (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | loop _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .loop (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | seq _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .seq (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | index _ he _ si ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .index (ih1 hΓ hv htv) he (ih2 hΓ hv htv) si
  | append _ hea _ heb ih1 ih2 =>
    intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .append (ih1 hΓ hv htv) hea (ih2 hΓ hv htv) heb
  | assign hx _ st ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .assign hx (ih hΓ hv htv) st
  | init hx hm _ st ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .init hx hm (ih hΓ hv htv) st
  | @letIn Γ0 z t' e b te tb he st hb ih1 ih2 =>
    intro Γ y tv v hΓ hv htv
    simp only [Expr.subst]
    subst hΓ
    by_cases hzy : z = y
    · subst hzy
      rw [Ctx.set_same] at hb
      simpa using HasType.letIn (ih1 rfl hv htv) st hb
    · simp only [hzy, ite_false]
      exact .letIn (ih1 rfl hv htv) st (ih2 (Ctx.set_comm Γ hzy tv t') hv htv)
  | call hf _ st ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .call hf (ih hΓ hv htv) st
  | error => intros; simp only [Expr.subst]; exact .error
  | tryCatch _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .tryCatch (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | cast _ ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .cast (ih hΓ hv htv)
  | broadcast hf _ he ha ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .broadcast hf (ih hΓ hv htv) he ha

/-- a value of type tv ≤ t bound to a local of type t: the body keeps (a subtype of) its type -/
theorem let_typed {y t v b tv tb} (hb : HasType P (Ctx.empty.set y t) b tb) (hv : v.isValue = true)
    (htv : HasType P Ctx.empty v tv) (st : sub tv t = true) :
    ∃ t', HasType P Ctx.empty (b.subst y v) t' ∧ sub t' tb = true := by
  have hs : CtxSub (Ctx.empty.set y tv) (Ctx.empty.set y t) := by
    intro z tz hz
    simp only [Ctx.set] at hz ⊢
    split at hz
    · cases hz; simp [*]
    · simp [Ctx.empty] at hz
  obtain ⟨t', hb', s'⟩ := narrow hb hs
  exact ⟨t', subst_typed hb' rfl hv htv, s'⟩

end Warp
