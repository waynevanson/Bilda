---
marp: true
theme: catppuccin
paginate: true
header: |
  Rust for programming languages | Wayne Van Son
---

# Rust

The language for languages

<sub>Wayne Van Son</sub>

---

# Excerpt

```
let
  a = 2
  b = True
  d = "Hello, World!"
  e = [a b c d e]
  List = + {

  }
  # Kleisli to lift so I don't repeat myself?
on
  z = [5 3 2 1]
do List::of
  y = z + 2
in
  [y * z]
```

---

## Inspiration

1. Cormack's presentation — Build system tool `buck2`
2. How to make languages better?
3. Monads
4. Rust project way to advanced

---

## Why Rust?

1. Performance - Codebases could 1M LOC
2. Strictness - Do what we know
3. Structures - Sum, Product and Newtypes
4. Ecosystem - Reusing the wheel

---

## Signtaure

```
Text -> Effect
```

Our code needs to do something

---

#### Transformation Pipeline (Program/s)

```
                 ┌─ Text ───────── "let x = 2 in x"
Lexer ═══════════│
                 ├─ Tokens  ────── "let" "x" "=" "2" "in" "x"
Parser ══════════│
                 ├─ Ast ────────── ast(vars (x, expr (3)), expr (identifier (x)))
Compiler ════════│
                 ├─ IR ─────────── Intermediate Representation
Code generation ═│
                 ├─ MachineCode ── Binary, JIT
Execution ═══════│
                 └─ Effect ─────── 2
```

---

# 1. Lexer

---

## Why a lexer?

### Signature

`Text -> Tokens`

### Responsibilities

1. Groups characters
2. Categorizes groups
3. List

---

## How to implement lexer

```rust
use logos::Logos;

#[derive(Logos)]
#[logos(skip(r"\s+"))]
enum Token<'input> {
  #[token("let")]
  Let,

  #[regexp("[a-zA-Z0-9]+")]
  Identifier(&'input str)
}

fn main() {
    let input = "let us";
    let lexer = Token::lexer(input);
    let tokens = lexer.collect().unwrap();
    let expected = vec![Token::Let, Token::Identifier("us")];
    assert_eq(tokens, expected);
}
```

---

## Why logos?

Macro based rust crate that constructs a Lexer.

1. Quick & Simple
2. Compile time distinct patterns
3. Skip patterns - spaces, comments
4. Inline transforms - Coerce `&str` to value
5. Lexer composition - String interpolation

---

# 2. Parser

---

## Parser

### Signature

`Tokens -> AbstractSyntaxTree`

### Responsibilities

1. Group tokens
2. Categorizes groups
3. Tree

---

## Parser - Example

```rust
use token::Token;
use chumsky::prelude::*;

struct Ast {
    identifier: String
}

// returns a parser for Ast
fn ast<'token, 'src: 'token>() -> impl Parser<'token, &'src str, Ast> {
    // parser - match if this token is next
    let identifier = select! {
        Token::Identifier(str) => Ast {
            identifier: identifier.to_string()
        }
    };

    // parser - let keyword
    let let_keyword = just(Token::Let);

    // parser - ignores first value, keeps second
    let_keyword.ignore_then(identifier)
}
```

---

## Parser - Example

```rust
fn main() {
    // after lex
    let tokens = vec![Token::Let, token::Identifier("us")];

    // verification only
    ast().check(tokens).unwrap()

    // verify and create structure
    let parsed: Ast = ast().parse(tokens).unwrap()
}
```

---

### Why a parser combinator library?

1. Allow parser to grammer representation
2. Composition - Join many together
3. Complex error handling managed

---

### Why `chumsky`?

(on Codeberg, not GitHub) is a trait/function based crate that constructs composable parsers with error handling,

1. Performant.
2. Recursive descent.
3. Pratt (precedence).
4. Composition.
5. `check` and `run` mode.

---

## Signature of parser

```rust
<Token, Context, Value, Error>
(tokens: Stream<Token>, context: &mut Context) ->
(Option<Value>, Vec<Error>)
```

## Design

1. Tokens and context.
2. Token cursor moves along.
3. Mutate context.
4. Calculates from tokens taken.
5. Adds errors.

---

## Parsers - Constructors

1. Default `empty()`
   1. increments 0 token/s.
   2. returns `()`.
   3. success.
1. Match 1 `just(i)`
   1. increments 1 token/s.
   2. returns `i`.
   3. success.

---

## Parsers - Combinators

1. `parser_a.then(parser_b) -> (A, B)`
   1. Parse both, keep both.
1. `parser_a.then_ignore(parser_b) -> A`
   1. Parse both, keep first parser value.
1. `parser_a.ignore_then(parser_b) -> B`
   1. Parse both, keep second value.

---

## Parsers - Combinators (cont.1)

1. `parser.delimited_by(parser_a, parser_b) -> T`
   1. `parser_a.ignore_then(parser).then_ignore(parser)`
   2. Returns value from parser.
1. `parser.repeated().at_least(1).at_most(5).collect() -> Vec<T>`
   1. Iterator-like for mulitple of same value.

---

## Parsers - Recursive

1. `recursive(|parser_a| { parser_a })`
   1. Create a parser that relies on itself.
2. `parser.pratt((parser_a, parser_b))`
   1. Parse precedence for infix stuff.

---

# 3. Compiler

---

## Compiler - Side Effects

## Signature

```
> Ast -> ()
  Ast -> IR
```

## Responsibilities

1.  Type checking
2.  Borrow checking
3.  ???

---

## Why Compile?

1. 1 per IR instead of 1 per target `(n + n, n * n)`
2. Developer experience.

---

## Signature

```
  Ast -> ()
> Ast -> IR
```

## Intermediate Representation (IR)

1. Code
2. Indirection
3. Compile to 1 code instead of many codes (architecture targets)
4. Processable by LLVM or cranelift (different formats)
5. Make the implicit explicit (types, drops, jumps, arithmetic)

---

# 4. Code generation

---

## Signature

```
IR -> MachineCode
```

## Responsibilities

1. Generate machine code
2. Target specific
   1. Architecture
   2. OS
3. ~~Creating an executable~~ not always!

---

## Tools

1. LLVM
2. cranelift

---

# 5. Execution

---

## Signature

```
MachineCode -> Effect
```

## Responsibilities

1. Run machine code.
2. Direct from binary or in an intepreter.

---

# How we gon do it fam?

---

## JIT

It generates machine code inside your app, gives you a pointer to the function you can then call to get a response back.

steps

1. Iterate over our AST using cranelift JIT API's (AST -> MachineCode)
2. Return pointer to machine code.
3. Execute it (MachineCode -> Effect).

---

# FIN

# FIN

# FIN

---

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
