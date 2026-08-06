#[derive(Clone, Debug, PartialEq)]
pub struct Ast(pub Vec<Block>);

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Let(LetBlock),
    ExprWhere(ExprWhereBlock),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LetBlock {
    pub bindings: Vec<Binding>,
    pub body: Box<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprWhereBlock {
    pub body: Box<Expr>,
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Ident(String),
    Number(String),
    Apply {
        func: Box<Expr>,
        arg: Box<Expr>,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    Access {
        target: Box<Expr>,
        field: String,
        kind: AccessKind,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Infix {
        op: InfixOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Record(Vec<Binding>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AccessKind {
    Type,
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrefixOp {
    Annotation,
    Sum,
    Product,
    Difference,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InfixOp {
    Add,
    Sub,
    Mul,
}
