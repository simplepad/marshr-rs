use std::{cell::RefCell, rc::Rc};

pub type Rrc<T> = Rc<RefCell<T>>;

pub type ObjectID = usize;
pub type SymbolID = usize;

#[derive(PartialEq, Debug)]
pub enum RubyValue {
    Nil,
    Boolean(bool),
    FixNum(i32),
    Symbol(SymbolID),
    Array(ObjectID),
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

#[derive(PartialEq, Debug)]
pub enum RubyObject {
    Empty, // for the 0th element (ruby object index starts with 1)
    Array(Vec<RubyValue>),
}

#[derive(Debug)]
pub struct Root {
    symbols: Vec<String>,
    objects: Vec<RubyObject>,
    root: RubyValue,
}

impl Root {
    pub fn new(root: RubyValue, symbols: Vec<String>, objects: Vec<RubyObject>) -> Self {
        Self {root, symbols, objects}
    }

    pub fn get_root(&self) -> &RubyValue {
        &self.root
    }

    pub fn get_symbols(&self) -> &Vec<String> {
        &self.symbols
    }

    pub fn get_objects(&self) -> &Vec<RubyObject> {
        &self.objects
    }

    pub fn get_symbol(&self, id: SymbolID) -> Option<&String> {
        self.symbols.get(id)
    }

    pub fn get_object(&self, id: ObjectID) -> Option<&RubyObject> {
        if id == 0 { return None; } // object ids start with 1
        self.objects.get(id)
    }
}
