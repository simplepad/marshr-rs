use std::{cell::RefCell, collections::{btree_map::Keys, HashMap}, ops::{Index, IndexMut}, rc::Rc};

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
    Class(ObjectID),
    Module(ObjectID),
    ClassOrModule(ObjectID),
    // Data(Vec<u8>), // TODO pick a better value
    Float(ObjectID),
    Hash(ObjectID),
    HashWithDefault(ObjectID),
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
    Hash(HashMap<SymbolID, RubyValue>),
    HashWithDefault(HashWithDefault),
    Float(f64),
    Class(String),
    Module(String),
    ClassOrModule(String),
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
        self.objects.get(id)
    }
}

#[derive(PartialEq, Debug)]
pub struct HashWithDefault {
    hash: HashMap<SymbolID, RubyValue>,
    default: RubyValue,
}

impl HashWithDefault {
    pub fn new(hash: HashMap<SymbolID, RubyValue>, default: RubyValue) -> Self {
        Self { hash, default }
    }

    pub fn len(&self) -> usize {
        self.hash.len()
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = &'a SymbolID> {
        self.hash.keys()
    }
}

impl Index<usize> for HashWithDefault {
    type Output = RubyValue;
    fn index(&self, index: usize) -> &Self::Output {
        self.hash.get(&index).unwrap_or(&self.default)
    }
}

impl IndexMut<usize> for HashWithDefault {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.hash.get_mut(&index).unwrap_or(&mut self.default)
    }
}
