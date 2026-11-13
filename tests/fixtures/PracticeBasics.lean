namespace Basics

def double (n : Nat) : Nat := n + n

def greet (name : String) : String :=
  s!"Hello, {name}!"

example : double 3 = 6 := rfl

end Basics
