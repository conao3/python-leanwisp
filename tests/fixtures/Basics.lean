/-! Module docs: keep (3: Nat) exactly. -/
namespace Demo
-- Nested comments and Unicode bytes must survive.
/- outer /- inner : := -/ end -/
def double (n:Nat):Nat:=n+n
#check (3: Nat)
#eval (3: Nat) + 4
#eval "colon: inside string := untouched"
def letter : Char := ':'
def quoted : String := "escaped \" : quote"
def «a:b» : Nat := 3
example (a:Nat) : a=a := by
  rfl
example (p q:Prop) (hp:p) (hq:q):p ∧ q := by
  constructor
  · exact hp
  · exact hq
def loop : IO Unit := do
  let n:Nat:=3
  IO.println n
end Demo
