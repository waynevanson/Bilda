pub enum Ast {
    LetIn {
        assignments: Vec<Assignment>,
        expression: Box<Ast>,
    },
    Expression(Expression),
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

pub enum Expression {
    Map { assignments: Vec<Assignment> },
    ReservedValue(ReservedValue),
    ReservedType(ReservedType),
}

pub struct Assignment {
    target: AssignmentProperty,
    value: Box<Ast>,
}

pub struct AssignmentProperty {
    name: String,
    r#type: Option<()>,
}

pub enum PropertyKind {
    Static(String),
    // something that can be a key
    Computed(Computable),
}

pub enum Computable {}
