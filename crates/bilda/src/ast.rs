#[derive(Debug, PartialEq)]
pub enum Ast<'input> {
    LetIn {
        assignments: Vec<Assignment<'input>>,
        expression: Box<Expression<'input>>,
    },
    Expression(Expression<'input>),
}

#[derive(Debug, PartialEq)]
pub enum MathSign {
    Subtraction,
    Addition,
    Multiplication,
    Division,
}

#[derive(Debug, PartialEq)]
pub enum MathTarget<'input> {
    Number(isize),
    Reference(&'input str),
    Math(Box<Math<'input>>),
}

#[derive(Debug, PartialEq)]
pub struct Math<'input> {
    pub sign: MathSign,
    pub left: MathTarget<'input>,
    pub right: MathTarget<'input>,
}

#[derive(Debug, PartialEq)]
pub enum Expression<'input> {
    Map {
        assignments: Vec<Assignment<'input>>,
    },
    Product {
        assignments: Vec<Assignment<'input>>,
    },
    Sum {
        assignments: Vec<Assignment<'input>>,
    },
    Call {
        function: &'input str,
        argument: Box<Expression<'input>>,
    },
    Int(isize),
    Float(f64),
    Boolean(bool),
    String(&'input str),
    Reference(&'input str),
    Math(Math<'input>),
}

#[derive(Debug, PartialEq)]
pub struct Assignment<'input> {
    pub name: &'input str,
    pub r#type: Option<()>,
    pub value: Box<Ast<'input>>,
}
