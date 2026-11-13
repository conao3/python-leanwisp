import Lean
open Lean
partial def fingerprint : Syntax → Json
  | .missing => toJson ("missing" : String)
  | .atom _ value => Json.arr #[toJson "atom", toJson value]
  | .ident _ raw value _ => Json.arr #[toJson "ident", toJson raw.toString, toJson value.toString]
  | .node _ kind args => Json.arr #[toJson kind.toString, Json.arr (args.map fingerprint)]
partial def tokens : Syntax → Array Json
  | .node _ _ args => args.foldl (fun out arg => out ++ tokens arg) #[]
  | stx => match stx.getPos?, stx.getTailPos? with
    | some a, some b => #[Json.arr #[toJson a.byteIdx, toJson b.byteIdx, toJson stx.getAtomVal]]
    | _, _ => #[]
partial def supported (stx : Syntax) : Bool :=
  let forbidden := ["syntax", "macro", "elab", "notation", "infix", "infixl", "infixr", "prefix", "postfix", "declare_syntax_cat", "scoped", "attribute", "run_cmd", "initialize", "builtin_initialize"]
  match stx with
  | .atom _ value => !forbidden.contains value
  | .node _ _ args => args.all supported
  | _ => true
unsafe def main : IO Unit := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let env ← importModules (loadExts := true) #[{module := `Init}] {} 0
  let source ← (← IO.getStdin).readToEnd
  let ctx := Parser.mkInputContext source "input.lean"
  let (header, _, messages) ← Parser.parseHeader ctx
  if messages.hasErrors then
    throw (IO.userError "invalid module header")
  unless header.raw[0].getArgs.isEmpty && header.raw[1].getArgs.isEmpty do
    throw (IO.userError "module and prelude headers are unsupported")
  -- Imports may install arbitrary parsers; the initial release handles Init only.
  for imp in header.raw[2].getArgs do
    unless imp[4].getId == `Init do
      throw (IO.userError "only implicit or explicit Init imports are supported")
  let stx ← Parser.testParseModule env "input.lean" source
  unless supported stx do
    throw (IO.userError "syntax extensions and parser-changing commands are unsupported")
  IO.println (Json.mkObj [("Tree", fingerprint stx), ("Tokens", Json.arr (tokens stx))]).compress
