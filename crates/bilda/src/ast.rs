pub enum Ast<'input> {
    LetIn {
        assignments: Vec<Assignment<'input>>,
        expression: Box<Expression<'input>>,
    },
    Expression(Expression<'input>),
}

pub enum MathSign {
    Subtraction,
    Addition,
    Multiplication,
    Division,
}

pub enum MathTarget<'input> {
    Number(&'input isize),
    Reference(&'input str),
}

pub struct Math<'input> {
    pub sign: MathSign,
    pub left: MathTarget<'input>,
    pub right: MathTarget<'input>,
}

pub enum Expression<'input> {
    Map {
        assignments: Vec<Assignment<'input>>,
    },
    Int(&'input isize),
    Float(&'input f64),
    Boolean(bool),
    String(&'input str),
    Reference(&'input str),
    Math(Math<'input>),
}

pub struct Assignment<'input> {
    pub name: &'input str,
    pub r#type: Option<()>,
    pub value: Box<Ast<'input>>,
}
