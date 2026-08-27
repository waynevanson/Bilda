#[derive(Debug, PartialEq)]
pub enum Ast<'input> {
    LetIn(LetIn<'input>),
    Expression(Expression<'input>),
}

#[derive(Debug, PartialEq)]
pub struct LetIn<'input> {
    pub assignments: Assignments<'input>,
    pub expression: Box<Expression<'input>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Boolean(pub bool);

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
pub struct Map<'input> {
    pub assignments: Assignments<'input>,
}

#[derive(Debug, PartialEq)]
pub struct Product<'input> {
    pub assignments: Assignments<'input>,
}

#[derive(Debug, PartialEq)]
pub struct Sum<'input> {
    pub assignments: Assignments<'input>,
}

#[derive(Debug, PartialEq)]
pub struct Lambda<'input> {
    pub params: Vec<&'input str>,
    pub body: Box<Ast<'input>>,
}

#[derive(Debug, PartialEq)]
pub struct Call<'input> {
    pub function: Box<Expression<'input>>,
    pub argument: Box<Expression<'input>>,
}

#[derive(Debug, PartialEq)]
pub struct Concat<'input> {
    pub left: Expression<'input>,
    pub right: Expression<'input>,
}

#[derive(Debug, PartialEq)]
pub enum Expression<'input> {
    Map(Map<'input>),
    Product(Product<'input>),
    Sum(Sum<'input>),
    Call(Call<'input>),
    Lambda(Lambda<'input>),
    Int(isize),
    Float(f64),
    String(&'input str),
    Reference(&'input str),
    Math(Math<'input>),
    Boolean(Boolean),
    Not(Boolean),
    List(Vec<Expression<'input>>),
    Concat(Box<Concat<'input>>),
    Unit,
}

#[derive(Debug, PartialEq)]
pub struct Assignment<'input> {
    pub name: &'input str,
    pub value: Box<Ast<'input>>,
}

pub type Assignments<'input> = Vec<Assignment<'input>>;
