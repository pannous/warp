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
  | ref => exact .ref
  | clo hb => exact .clo hb
  | cons _ _ he ih1 ih2 => simp [isValue] at hv; exact .cons (ih1 hv.1) (ih2 hv.2) he
  | _ => simp [isValue] at hv

/-- a value's type is a scalar or a list type, never `never` -/
theorem value_list {Γ v t e} (h : HasType P Γ v t) (hv : v.isValue = true) (he : element t = some e) :
    t = .list e := by
  cases h <;> simp_all [isValue, element]

theorem list_value {Γ v a} (h : HasType P Γ v (.list a)) (hv : v.isValue = true) :
    v = .nil ∨ ∃ hd tl, v = .cons hd tl ∧ hd.isValue = true ∧ tl.isValue = true := by
  cases v <;> simp [isValue] at hv ⊢ <;> first | exact hv | exact ⟨_, _, ⟨rfl, rfl⟩, hv⟩ | cases h

/-- `-` and `*` on values: a number of a type below `arithTy`, or an error -/
theorem arith_typed {Γ op a b ta tb} (ha : HasType P Γ a ta) (hb : HasType P Γ b tb) (va : a.isValue = true)
    (vb : b.isValue = true) : ∃ t', HasType P Γ (arithValues op a b) t' ∧ sub t' (arithTy ta tb) = true := by
  unfold arithValues
  split
  · exact ⟨_, .error, sub_never _⟩
  split
  · exact ⟨_, .error, sub_never _⟩
  · cases ha <;> cases hb <;>
      simp_all [isValue, isNumber, asInt, asNumber, arithTy, Ty.arith, sub] <;>
      first | exact ⟨_, .int, by decide⟩ | exact ⟨_, .num, by decide⟩

theorem lt_typed {Γ a b} : ∃ t', HasType P Γ (ltValues a b) t' ∧ sub t' .bool = true := by
  unfold ltValues
  split
  · exact ⟨_, .bool, sub_refl _⟩
  · split
    · exact ⟨_, .bool, sub_refl _⟩
    · exact ⟨_, .error, sub_never _⟩

theorem nth_typed {Γ} : ∀ {l : Expr} (i : Int) {tl v}, l.isValue = true → HasType P Γ l tl →
    nth l i = some v → ∃ tv, HasType P Γ v tv ∧ sub tv (elementTy tl) = true := by
  intro l
  induction l with
  | cons h t _ iht =>
    intro i tl v hv hl hn
    simp [isValue] at hv
    cases hl with
    | cons hh ht hel =>
      simp only [elementTy, element, reduceCtorEq, ite_false, Option.getD_some]
      simp [nth] at hn
      split at hn
      · cases hn; exact ⟨_, hh, join_upper_left _ _⟩
      · obtain ⟨tv, htv, hs⟩ := iht (i - 1) hv.2 ht hn
        rw [elementTy_of_element hel] at hs
        exact ⟨tv, htv, sub_trans hs (join_upper_right _ _)⟩
  | text s =>
    intro i tl v _ hl hn
    cases hl
    simp only [nth] at hn
    split at hn
    · obtain ⟨c, _, rfl⟩ := Option.map_eq_some_iff.1 hn; exact ⟨_, .text, by simp [elementTy, sub_refl]⟩
    · cases hn
  | _ => intro i tl v _ _ hn; simp [nth] at hn

/-- concatenating list values: a list of the joined element types -/
theorem concat_typed {Γ b tb eb} (hb : HasType P Γ b tb) (vb : b.isValue = true) (eb' : element tb = some eb) :
    ∀ {a : Expr} {ta ea}, a.isValue = true → HasType P Γ a ta → element ta = some ea →
    ∃ t', HasType P Γ (concat a b) t' ∧ sub t' (.list (join ea eb)) = true := by
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

/-- a list value's type has elements -/
theorem list_element {Γ v t} (h : HasType P Γ v t) (hl : isList v = true) : ∃ e, element t = some e := by
  cases h <;> simp_all [isList, element]

/-- `++` on values: a list of the joined element types, or an error -/
theorem append_typed {Γ a b ta tb} (ha : HasType P Γ a ta) (hb : HasType P Γ b tb) (va : a.isValue = true)
    (vb : b.isValue = true) :
    ∃ t', HasType P Γ (appendValues a b) t' ∧ sub t' (.list (join (listElem ta) (listElem tb))) = true := by
  unfold appendValues
  split
  · rename_i hl
    simp only [Bool.and_eq_true] at hl
    obtain ⟨ea, hea⟩ := list_element ha hl.1
    obtain ⟨eb, heb⟩ := list_element hb hl.2
    rw [listElem_of_element hea, listElem_of_element heb]
    exact concat_typed hb vb heb va ha hea
  · exact ⟨_, .error, sub_never _⟩

/-- `+` on values: two lists' concatenation, a number or a text of a type below `plus`, or an error -/
theorem add_typed {Γ a b ta tb} (ha : HasType P Γ a ta) (hb : HasType P Γ b tb) (va : a.isValue = true)
    (vb : b.isValue = true) : ∃ t', HasType P Γ (addValues a b) t' ∧ sub t' (plus ta tb) = true := by
  unfold addValues
  split
  · rename_i hl
    simp only [Bool.and_eq_true] at hl
    obtain ⟨ea, hea⟩ := list_element ha hl.1
    obtain ⟨eb, heb⟩ := list_element hb hl.2
    have hta := value_list ha va hea
    have htb := value_list hb vb heb
    subst hta htb
    simp only [plus, isListTy, Bool.and_self, if_true, element, Option.getD_some]
    simpa using concat_typed hb vb heb va ha hea
  split
  · exact ⟨_, .error, sub_never _⟩
  · cases ha <;> cases hb <;>
      simp_all [isValue, isNumber, isText, isList, asInt, asNumber, plus, isListTy, Ty.arith, sub] <;>
      first | exact ⟨_, .int, by decide⟩ | exact ⟨_, .num, by decide⟩ | exact ⟨_, .text, by decide⟩

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

/-- a smaller object type reads a smaller field type -/
theorem readTy_mono {te te' : Ty} (h : sub te' te = true) (f : String) :
    sub (P.readTy te' f) (P.readTy te f) = true := by
  cases te <;> simp [Program.readTy]
  · rw [sub_to_never h]; simp
  · cases te' <;> simp_all [sub]
    rename_i p q
    cases hf : P.fieldTy p f with
    | none => simp
    | some t => rw [P.fieldTy_prefix h hf]; simp [sub_refl]

/-- a smaller object type takes a larger field type -/
theorem writeTy_anti {te te' : Ty} (h : sub te' te = true) {f t} (ht : P.writeTy te f = some t) :
    ∃ t', P.writeTy te' f = some t' ∧ sub t t' = true := by
  cases te <;> simp [Program.writeTy] at ht
  · subst ht; rw [sub_to_never h]; exact ⟨_, rfl, sub_any _⟩
  · cases te' <;> simp_all [Program.writeTy, sub]
    exact ⟨_, P.fieldTy_prefix h ht, sub_refl _⟩

/-- Γ' gives every local of Γ a smaller type -/
def CtxSub (Γ' Γ : Ctx) : Prop := ∀ z t, Γ z = some t → ∃ t', Γ' z = some t' ∧ sub t' t = true

theorem CtxSub.set {Γ' Γ : Ctx} (h : CtxSub Γ' Γ) (y : String) (t : Ty) : CtxSub (Γ'.set y t) (Γ.set y t) := by
  intro z tz hz
  simp only [Ctx.set] at hz ⊢
  split at hz
  · cases hz; exact ⟨_, by simp [*], sub_refl _⟩
  · simp [*]; exact h z tz hz

theorem CtxSub.refl (Γ : Ctx) : CtxSub Γ Γ := fun _ t h => ⟨t, h, sub_refl t⟩

theorem CtxSub.set_le {Γ' Γ : Ctx} (h : CtxSub Γ' Γ) (y : String) {t' t : Ty} (st : sub t' t = true) :
    CtxSub (Γ'.set y t') (Γ.set y t) := by
  intro z tz hz
  simp only [Ctx.set] at hz ⊢
  split at hz
  · cases hz; exact ⟨t', by simp [*], st⟩
  · simp [*]; exact h z tz hz

/-- the ints of a range are a list of ints -/
theorem intList_typed {Γ} : ∀ (k : Nat) (m : Int), ∃ e, HasType P Γ (intList m k) (.list e) ∧ sub e .int = true
  | 0, _ => ⟨_, .nil, rfl⟩
  | k + 1, m => by
    obtain ⟨e, ht, se⟩ := intList_typed k (m + 1)
    exact ⟨_, .cons .int ht rfl, join_least rfl se⟩

theorem int_sub_arithTy (a b : Ty) : sub .int (arithTy a b) = true := by
  unfold arithTy Ty.arith; split
  · rfl
  · split <;> rfl

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
  | add _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .add h1 h2, plus_mono s1 s2⟩
  | arith _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .arith h1 h2, arithTy_mono s1 s2⟩
  | lt _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, _⟩ := ih1 hs
    obtain ⟨b', h2, _⟩ := ih2 hs
    exact ⟨_, .lt h1 h2, sub_refl _⟩
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
  | loop _ _ _ ih1 ih2 ih3 =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih1 hs
    obtain ⟨_, h2, s2⟩ := ih2 hs
    obtain ⟨_, h3, s3⟩ := ih3 hs
    exact ⟨_, .loop h1 h2 h3, join_mono s2 (join_mono s3 (sub_refl _))⟩
  | seq _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .seq h1 h2, s2⟩
  | range _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .range h1 h2, by simpa [sub] using arithTy_mono s1 s2⟩
  | index _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨l', h1, s1⟩ := ih1 hs
    obtain ⟨i', h2, _⟩ := ih2 hs
    exact ⟨_, .index h1 h2, elementTy_mono s1⟩
  | append _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨a', h1, s1⟩ := ih1 hs
    obtain ⟨b', h2, s2⟩ := ih2 hs
    exact ⟨_, .append h1 h2, by simpa using join_mono (listElem_mono s1) (listElem_mono s2)⟩
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
  | ref => intros; exact ⟨_, .ref, sub_refl _⟩
  | new => intros; exact ⟨_, .new, sub_refl _⟩
  | get _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .get h1, readTy_mono s1 _⟩
  | set _ hw _ st ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih1 hs
    obtain ⟨_, h2, s2⟩ := ih2 hs
    obtain ⟨t', hw', st'⟩ := writeTy_anti s1 hw
    exact ⟨_, .set h1 hw' h2 (sub_trans (sub_trans s2 st) st'), s2⟩
  | isA _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih hs
    exact ⟨_, .isA h1, sub_refl _⟩
  | handle hR _ sh _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih1 (hs.set _ _)
    obtain ⟨_, h2, s2⟩ := ih2 hs
    exact ⟨_, .handle hR h1 (sub_trans s1 sh) h2, join_mono s2 (sub_refl _)⟩
  | emit hR _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, _⟩ := ih hs
    exact ⟨_, .emit hR h1, sub_refl _⟩
  | scope _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .scope h1, s1⟩
  | abort _ st ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih hs
    exact ⟨_, .abort h1 (sub_trans s1 st), sub_refl _⟩
  | forIn _ sT _ _ ih1 ih2 ih3 =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih1 hs
    obtain ⟨_, h2, s2⟩ := ih2 (hs.set _ _)
    obtain ⟨_, h3, s3⟩ := ih3 hs
    exact ⟨_, .forIn h1 (sub_trans (elementTy_mono s1) sT) h2 h3, join_mono s2 (join_mono s3 (sub_refl _))⟩
  | lam _ ih =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih (hs.set _ _)
    exact ⟨_, .lam h1, by simpa using s1⟩
  | clo hb => intros; exact ⟨_, .clo hb, sub_refl _⟩
  | app _ _ ih1 ih2 =>
    intro Γ' hs
    obtain ⟨_, h1, s1⟩ := ih1 hs
    obtain ⟨_, h2, _⟩ := ih2 hs
    exact ⟨_, .app h1 h2, resultTy_mono s1⟩

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
  | add _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .add (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | arith _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .arith (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | lt _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .lt (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | eq _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .eq (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | ite _ _ _ ih0 ih1 ih2 =>
    intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .ite (ih0 hΓ hv htv) (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | loop _ _ _ ih1 ih2 ih3 =>
    intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .loop (ih1 hΓ hv htv) (ih2 hΓ hv htv) (ih3 hΓ hv htv)
  | seq _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .seq (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | index _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .index (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | range _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .range (ih1 hΓ hv htv) (ih2 hΓ hv htv)
  | append _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .append (ih1 hΓ hv htv) (ih2 hΓ hv htv)
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
  | ref => intros; simp only [Expr.subst]; exact .ref
  | new => intros; simp only [Expr.subst]; exact .new
  | get _ ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .get (ih hΓ hv htv)
  | set _ hw _ st ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .set (ih1 hΓ hv htv) hw (ih2 hΓ hv htv) st
  | isA _ ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .isA (ih hΓ hv htv)
  | @handle Γ0 ev h b th tb R hR hh sh hb ih1 ih2 =>
    intro Γ y tv v hΓ hv htv
    simp only [Expr.subst]
    subst hΓ
    by_cases hy : y = eventLocal
    · subst hy
      rw [Ctx.set_same] at hh
      simpa using HasType.handle hR hh sh (ih2 rfl hv htv)
    · simp only [hy, ite_false]
      exact .handle hR (ih1 (Ctx.set_comm Γ (fun h => hy h.symm) tv .any) hv htv) sh (ih2 rfl hv htv)
  | emit hR _ ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .emit hR (ih hΓ hv htv)
  | scope _ ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .scope (ih hΓ hv htv)
  | abort _ st ih => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .abort (ih hΓ hv htv) st
  | @forIn Γ0 z l b d tl T tb td hl sT hb hd ih1 ih2 ih3 =>
    intro Γ y tv v hΓ hv htv
    simp only [Expr.subst]
    subst hΓ
    by_cases hzy : z = y
    · subst hzy
      rw [Ctx.set_same] at hb
      simpa using HasType.forIn (ih1 rfl hv htv) sT hb (ih3 rfl hv htv)
    · simp only [hzy, ite_false]
      exact .forIn (ih1 rfl hv htv) sT (ih2 (Ctx.set_comm Γ hzy tv _) hv htv) (ih3 rfl hv htv)
  | @lam Γ0 z b tb hb ih =>
    intro Γ y tv v hΓ hv htv
    simp only [Expr.subst]
    subst hΓ
    by_cases hzy : z = y
    · subst hzy
      rw [Ctx.set_same] at hb
      simpa using HasType.lam hb
    · simp only [hzy, ite_false]
      exact .lam (ih (Ctx.set_comm Γ hzy tv _) hv htv)
  | clo hb => intros; simp only [Expr.subst]; exact .clo hb
  | app _ _ ih1 ih2 => intro Γ y tv v hΓ hv htv; simp only [Expr.subst]; exact .app (ih1 hΓ hv htv) (ih2 hΓ hv htv)

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
