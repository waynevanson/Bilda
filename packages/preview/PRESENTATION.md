---
marp: true
theme: catppuccin-mocha
paginate: true
header: >
  Rust for programming languages
  |
  Wayne Van Son
transition: implode 0.1s
---

# Rust

The language for languages

<sub>Wayne Van Son</sub>

---

### Inspiration

1. Cormack's presentation of build system `buck2`
2. Languages have trade-offs — best bang for your `buck2`
3.

---

## Notes

## Language Goals

Make creating build systems easier.

### Ecosystem issues

1. Hot reload (dev mode, watch mode, test mode)
   1. Requires different script execution compared to build mode.
   2. Painful to setup, language dependent, nothing talks the same way.
   3. Rust into JS ecosystem? Easier to to neck yourself.
2. Multi import using globs? Barrel imports without setting up breweries.
3. Lockfile - Local system knows what artifacts to expect and when.
4. Feature explosion. Kaboom. Not just features, but every possible set of inputs/outputs.
5. Hermetic - Same every run.
   1. Same packages and setup every time.
   2. What are my artifacts actually? Remove tracked artifacts before build. Get intellisense?

### Solvable with language design

Maybe. All solvable without language design but requires tooling.

Could anything be solved with language design?

A language that is best for plugin systems?

```


```

Features

1. Glob as a built in construct.
2. Globs have a lock file that are hashed?
3. Allow composition of like-properties composable. `project-one.dependencies ++= [project-two, project-one.test]`
4. How about `projects-one.dependencies.script.build ++= [project-one.script.test]`

## Presentation

### Todos

1. You can be anything, like that comedian.

### Vibes

We've all used programming languages.
It's more than just executable text, it's a feeling.

### Star Factor

What makes a language stand out?

1.

### Breakdown

Let's break down implementing a programming language.

Our signature is essentially `Text -> Effect`.

Transformers required to get between each step.

Constructs Presumably agreed to be valuable after decades of research.

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

### Intepreter

The heart and soul, get's things done.

```
Interpreter -> Effect
(AbstractSyntaxTree)
```

`Cranelift` is a code generator & compiler backend, with modules like `cranelift-jit` to transform our AST to platform-agnostic intepreter.

### Compiler
