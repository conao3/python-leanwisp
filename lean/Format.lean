import Lean
open Lean

partial def fingerprint : Syntax → Json
  | .missing => toJson ("missing" : String)
  | .atom _ value => Json.arr #[toJson "atom", toJson value]
  | .ident _ raw value _ => Json.arr #[toJson "ident", toJson raw.toString, toJson value.toString]
  | .node _ kind args => Json.arr #[toJson kind.toString, Json.arr (args.map fingerprint)]

partial def document : Std.Format → Json
  | .nil => Json.arr #[toJson "nil"]
  | .line => Json.arr #[toJson "line"]
  | .align force => Json.arr #[toJson "align", toJson force]
  | .text value => Json.arr #[toJson "text", toJson value]
  | .nest indent child => Json.arr #[toJson "nest", toJson indent, document child]
  | .append a b => Json.arr #[toJson "append", document a, document b]
  | .group child behavior => Json.arr #[toJson "group", document child,
      toJson (behavior == .fill)]
  | .tag _ child => document child

partial def trivia : Syntax → Array Json
  | .node _ _ args => args.foldl (fun acc s => acc ++ trivia s) #[]
  | stx => match stx.getHeadInfo with
    | .original leading _ trailing _ =>
      #[toJson leading.toString, toJson trailing.toString]
    | _ => #[]

partial def changesParser : Syntax → Bool
  | .node _ `Lean.Parser.Command.declaration _ => false
  | .node _ `Lean.Parser.Command.in _ => false
  | .node _ `Lean.Parser.Command.mutual _ => false
  | .atom _ value => ["syntax", "macro", "macro_rules", "notation", "infix", "infixl", "infixr",
      "prefix", "postfix", "declare_syntax_cat", "namespace", "section", "end", "open",
      "set_option"].contains value
  | .node _ _ args => args.any changesParser
  | _ => false

open Elab Frontend in
partial def collect : FrontendM (Array Syntax × Array Std.Format) := do
  let ctx ← getInputContext
  let cs ← getCommandState
  let ps ← getParserState
  let scope := cs.scopes.head!
  let (cmd, ps, messages) := Parser.parseCommand ctx
    { env := cs.env, options := scope.opts, currNamespace := scope.currNamespace,
      openDecls := scope.openDecls } ps cs.messages
  setParserState ps
  setMessages messages
  if messages.hasErrors then
    messages.forM fun msg => do IO.eprintln (← msg.toString)
    throw (IO.userError "invalid Lean syntax")
  let fmt ← runCommandElabM <| Command.liftCoreM <| PrettyPrinter.formatCategory `command cmd
  if cmd.isOfKind `Lean.Parser.Command.eoi then return (#[cmd], #[fmt])
  if changesParser cmd then
    elabCommandAtFrontend cmd
    if (← getCommandState).messages.hasErrors then
      throw (IO.userError "cannot elaborate parser-changing command")
  let (commands, docs) ← collect
  return (#[cmd] ++ commands, #[fmt] ++ docs)

unsafe def main : IO Unit := do
  initSearchPath (← findSysroot)
  enableInitializersExecution
  let source ← (← IO.getStdin).readToEnd
  let ctx := Parser.mkInputContext source "input.lean"
  let (header, ps, messages) ← Parser.parseHeader ctx
  if messages.hasErrors then throw (IO.userError "invalid module header")
  let env ← importModules (loadExts := true) (Elab.headerToImports header) {} 0
  let (headerFmt, _) ← (PrettyPrinter.format Parser.Module.header.formatter header).toIO
    { fileName := "input.lean", fileMap := ctx.fileMap } { env }
  let ((commands, docs), _) ← collect.run { inputCtx := ctx } |>.run
    { commandState := Elab.Command.mkState env, parserState := ps, cmdPos := ps.pos }
  let stx := mkNode `Lean.Parser.Module.module #[header, mkListNode commands]
  let fmt := headerFmt ++ Std.Format.joinSep docs.toList (Std.Format.line ++ Std.Format.line)
  IO.println (Json.mkObj [("Tree", fingerprint stx), ("Document", document fmt),
    ("Trivia", Json.arr (trivia stx))]).compress
