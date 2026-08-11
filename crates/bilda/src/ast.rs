pub enum Ast<'input> {
    LetIn {
        assignments: Vec<Assignment<'input>>,
        expression: Box<Ast<'input>>,
    },
    Expression(Expression<'input>),
}

pub enum ReservedValue {
    List,
    True,
    False,
    Boolean,
    Int(usize),
    String(String),
}

pub enum ReservedType {
    Boolean,
    Int,
    List,
    String,
}

pub enum Expression<'input> {
    Map {
        assignments: Vec<Assignment<'input>>,
    },
    ReservedValue(ReservedValue),
    Reference(&'input str),
    Add(Box<Ast<'input>>, Box<Ast<'input>>),
}

pub struct Assignment<'input> {
    pub target: AssignmentProperty<'input>,
    pub value: Box<Ast<'input>>,
}

pub struct AssignmentProperty<'input> {
    pub name: &'input str,
    pub r#type: Option<()>,
}
