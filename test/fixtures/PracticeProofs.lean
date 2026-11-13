namespace Proofs

theorem identity (p : Prop) (h : p) : p := by
  exact h

theorem swap (p q : Prop) (h : p ∧ q) : q ∧ p := by
  constructor
  · exact h.2
  · exact h.1

theorem addZero (n : Nat) : n + 0 = n := by
  rfl

end Proofs
