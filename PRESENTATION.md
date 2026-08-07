## Notes

## Presentation

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

`Lexer(Text) -> Tokens`

1. Groups characters.
2. Categorizes groups.

`Logos` is a macro-based rust crate that constructs a Lexer, which we can then apply to the Text to create Tokens.

1. Order unimportant.
2. Allows you to skip irrelevant tokens like spaces.
3. Context is minimal - switch between lexers for different cases.

### Parser

Transforms tokens into an AST.

`Parser(Tokens) -> AbstractSyntaxTree`

1. Precedence.
2. Recursive descent.
3.
