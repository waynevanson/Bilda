---
marp: true
theme: catppuccin-mocha
paginate: true
header: |
  Rust for programming languages | Wayne Van Son
transition: fade 0.1s
---

<!--

Alright
Show the language we're making straight away
Explain what the example does
Explain how to build the language
Explain design choices

-->

# Rust

The language for languages

<sub>Wayne Van Son</sub>

---

# Why Rust?

Expected from Rust

1. Performance
2. Low level - memory management

What truly matters

1. Strict
2. Structures
3. Ecosystem

<!--
Ecosystem has great crates for Lexers, Parsers and Compilers (backend + JIT)
-->

---

### Inspiration

1. Cormack's presentation — Build system tool `buck2`
2. Language trade-offs
   - Interesting
   - Best bang for your `buck2`
3. Tim
   - `buck2`
   - ?
   - Timbuktu
   - ...

<!-- Where is TimBuckToo -->

---

## Language design

Build systems.

1. Language goals
2. Existing issues
3. Possible solutions

---

## Language Goal

Make managing build systems easier.

<!-- What's hard? -->

---

#### Issue - Hot Reload

Hot reload/Interactive (dev mode, watch mode, test mode)

Issues

1.  Requires different script execution compared to build mode.
2.  Painful to setup, language dependent, nothing talks the same way.
3.  Rust into JS ecosystem? Easier to to neck yourself.

---

### Analysis of existing use

```yaml
tasks:
  dev:
    persistent: true
  test:
  lint:
  build:
    dependsOn: ["build", "^lint", "^test"] # strings for tasks, ^ syntax
    inputs:
      files: ["./src/**.rs"] # strings for globs
      env: ["$DEFAULTS", "CI"] # rando vars
    outputs: ["./target/{target.platform}/[name].ts"] # rando vars
    scripts:
      - parallel: true # poor concurrency control and explicitness?
        exec: ["./scripts/migrate.sh"] # importing shell scripts is the best?
```

<!-- What can we do better? -->

---

| Problem                                   | Solution                                 | Example                                 |
| :---------------------------------------- | :--------------------------------------- | :-------------------------------------- |
| Strings for globs                         | Primitive attached to lockfile           | `./src/**/*.rs`                         |
| Strings for tasks                         | Reference existing tasks                 | `build.deps.downstream = [tasks.tests]` |
| Strings for tasks                         | Allow glob-like destructure              | `build.deps.downstream = tasks.{tests}` |
| Explicit dependencies express difficultly | Explicit combination syntax for features | Something like a glob!                  |
| `^[task]` syntax for dep reference        | Reference dep in diff prop               | `build.deps.self = self.build`          |
| Rando variables                           | Explicit access via scoped as closure    | `(vars) => vars.defaults{CI}`           |
| Rando variables                           | Range index                              | `(vars) => vars.all{CI}`                |
| Concurency                                | Keywords                                 | `exec (limit = 8)`                      |
| Concurency                                | Defaults                                 | Parallel by default                     |
| Concurency                                | ? Constraints                            |                                         |

---

### Proposal

```
tasks:
  dev:
    persistent: true
  test:
  build:
    depends:
      self: [&tasks.build]
      down: &tasks{lint, test}
    inputs:
      files: ./src/**.rs
      env: &vars
        => vars.defaults{*}
        ++ vars.all{ci}
    outputs:
      - &vars => ./target/{vars.target.platform}/{vars.projectName}
    scripts:
      - parallel: true # poor concurrency control and explicitness?
        exec: ["./scripts/migrate.sh"]
```

---

What about scripting?

Writing scripts sucks balls. Concurrency composition sucks. No way to control or delegate it really.

1. defaults - parallel

new lines are series by default. Imagine if they weren't?

---

What if we make let able to apply n operation as long as it returns?

Top level can even have no binding. assignment assumes let

Like roc, maybe use `main!` the `!` means effect?

```
pubs
  value = 1
  next = 2
  f = [9,8,7,6]
  third = {}
  items = do list::flatmap
    first = [1 2 3 4]
    second = [first * 2 first * 4]
  in
    first * second
```

---

Concurrency for scripts?

library mode vs run mode

```
from "./path/to/file.bilda"
  items

main!() = {
  for item in items
    echo! item.to_str()
  rof
}

```

---

### Ecosystem issues

1. Multi import using globs? Barrel imports without setting up breweries.
2. Lockfile - Local system knows what artifacts to expect and when.
3. Feature explosion. Kaboom. Not just features, but every possible set of inputs/outputs.
4. Hermetic - Same every run.
   1. Same packages and setup every time.
   2. What are my artifacts actually? Remove tracked artifacts before build. Get intellisense?
5. Syntax for concurrency? Gotta be easier than this shit.

---

### Solvable with language design?

Maybe. All solvable without language design but requires tooling.

Could anything be solved with language design?

Languages are complementary to the goal.

---

## A language that is best for plugin systems?

Features

1. Glob as a built in construct.
2. Globs have a lock file that are hashed?
3. Allow composition of like-properties composable. `project-one.dependencies ++= [project-two, project-one.test]`
4. How about `projects-one.dependencies.script.build ++= [project-one.script.test]`

<!--1. You can be anything, like that comedian.-->

---

### Vibes

We've all used programming languages.
It's more than just executable text, it's a feeling.

---

### Star Factor

What makes a language stand out?

1. Tooling & Experience
   1. LSP
   2. Dependencies
   3. Constraints

---

### Breakdown

Let's break down implementing a programming language.

Our signature is essentially `Text -> Effect`.

Transformers required to get between each step.

Constructs Presumably agreed to be valuable after decades of research.

---

#### Interpreted

Transformers.

```
Interpeter -> Effect
(
  Lexer(Text) -> Tokens
  Parser(Tokens) -> AbstractSyntaxTree
  Runtime(AbstractSyntaxTree) -> Effect
)
```

> Note: How you understand this syntax is different from other languages "\n"

---

#### Compiled

Transformers.

```
Compiler -> Executable
(
  Lexer(Text) -> Tokens
  Parser(Tokens) -> AbstractSyntaxTree
  Compiler(AbstractSyntaxTree) -> IntermediateRepresentation
  Compiler(IntermediateRepresentation) -> Executable
)

Executable -> Effect
```

---

### Lexer

Transforms text into tokens.

```
Lexer -> Tokens
(Text)
```

1. Groups characters.
2. Categorizes groups.

`Logos` is a macro-based rust crate that constructs a Lexer, which we can then apply to the Text to create Tokens.

1. Order unimportant.
2. Allows you to skip irrelevant tokens like spaces.
3. Context is minimal - switch between lexers for different cases.

---

### Parser

Transforms tokens into an AST.

```
Parser -> AbstractSyntaxTree
(Tokens)
```

1. Group tokens between other tokens.
2. Categorizes groups.

`Chumsky` (on Codeberg, not GitHub) is a trait/function based crate that constructs composable parsers with error handling,

1. Precedence.
2. Recursive descent.
3. Applies grammar (syntax) rules.
4. Create data so we can understand how to execute.
5. Code execution paths

TODODODOO

two paths, we show compiler and interpreter then we say a thirdish.

---

### Intepreter

The heart and soul, get's things done.

```
Interpreter -> Effect
(AbstractSyntaxTree)
```

`Cranelift` is a code generator & compiler backend, with modules like `cranelift-jit` to transform our AST to platform-agnostic intepreter.

---

### Compiler
