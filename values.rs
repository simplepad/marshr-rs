pub const I8_SIZE: usize = std::mem::size_of::<i8>();
pub const I16_SIZE: usize = std::mem::size_of::<i16>();
pub const I24_SIZE: usize = std::mem::size_of::<i32>() - std::mem::size_of::<i8>();
pub const I32_SIZE: usize = std::mem::size_of::<i32>();

#[derive(PartialEq, Debug)]
pub enum RubyObject {
    Nil,
    Boolean(bool),
    FixNum(i32),
    Symbol(String),
    // SymbolLink(u32),
    // ObjectLink(u32),
    // Array(Vec<RubyObject>),
    // BigNum(i64), // TODO pick a better value
    // Class(String),
    // Module(String),
    // Data(Vec<u8>), // TODO pick a better value
    // Float(f64),
    // Hash(HashMap<RubyObject, RubyObject>),
    // Object(RubyObject),
    // RegExp(String),
    // RubyString(String),
    // Struct(),
    // UserClass(),
    // UserDefined(),
    // UserMarshal(),
}

#[derive(Debug)]
pub struct Root {
    // object table
    // symbol table
    value: RubyObject
}

impl Root {
    pub fn new(value: RubyObject) -> Self {
        Self {value}
    }

    pub fn get_value(self) -> RubyObject {
        self.value
    }
}
