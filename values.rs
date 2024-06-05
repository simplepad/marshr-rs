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
    BigNum(ObjectID),
    Class(ObjectID),
    Module(ObjectID),
    ClassOrModule(ObjectID),
    // Data(ObjectID),
    Float(ObjectID),
    Hash(ObjectID),
    HashWithDefault(ObjectID),
    Object(ObjectID),
    RegExp(ObjectID),
    String(ObjectID),
    Struct(ObjectID),
    UserClass(ObjectID),
    UserDefined(ObjectID),
    UserMarshal(ObjectID),
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
    String(RubyString),
    BigNum(i64),
    RegExp(RegExp),
    Struct(Struct),
    Object(Object),
    UserClass(UserClass),
    UserDefined(UserDefined),
    UserMarshal(UserMarshal),
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

#[derive(PartialEq, Debug)]
pub struct RubyString {
    string: String,
    instance_variables: Option<HashMap<SymbolID, RubyValue>>,
}

impl RubyString {
    pub fn new(string: String) -> Self {
        Self {string, instance_variables: None}
    }

    pub fn get_string(&self) -> &String {
        &self.string
    }

    pub fn set_instance_variables(&mut self, instance_variables: HashMap<SymbolID, RubyValue>) {
        self.instance_variables = Some(instance_variables);
    }

    pub fn get_instance_variables(&self) -> &Option<HashMap<SymbolID, RubyValue>> {
        &self.instance_variables
    }

    pub fn get_instance_variable(&self, instance_variable: SymbolID) -> Option<&RubyValue> {
        if let Some(instance_variables) = &self.instance_variables {
            instance_variables.get(&instance_variable)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct RegExp {
    pattern: String,
    options: i8,
    instance_variables: Option<HashMap<SymbolID, RubyValue>>,
}

impl RegExp {
    pub fn new(pattern: String, options: i8) -> Self {
        Self {pattern, options, instance_variables: None}
    }

    pub fn set_instance_variables(&mut self, instance_variables: HashMap<SymbolID, RubyValue>) {
        self.instance_variables = Some(instance_variables);
    }

    pub fn get_instance_variables(&self) -> &Option<HashMap<SymbolID, RubyValue>> {
        &self.instance_variables
    }

    pub fn get_instance_variable(&self, instance_variable: SymbolID) -> Option<&RubyValue> {
        if let Some(instance_variables) = &self.instance_variables {
            instance_variables.get(&instance_variable)
        } else {
            None
        }
    }

    pub fn get_pattern(&self) -> &String {
        &self.pattern
    }

    pub fn get_options(&self) -> i8 {
        self.options
    }
}

#[derive(PartialEq, Debug)]
pub struct Struct {
    name: SymbolID,
    members: HashMap<SymbolID, RubyValue>,
}

impl Struct {
    pub fn new(name: SymbolID, members: HashMap<SymbolID, RubyValue>) -> Self {
       Self {name, members} 
    }

    pub fn get_name(&self) -> SymbolID {
        self.name
    }

    pub fn get_members(&self) -> &HashMap<SymbolID, RubyValue> {
        &self.members
    }

    pub fn get_member(&self, symbol_id: SymbolID) -> Option<&RubyValue> {
        self.members.get(&symbol_id)
    }
}

#[derive(PartialEq, Debug)]
pub struct Object {
    class_name: SymbolID,
    instance_variables: HashMap<SymbolID, RubyValue>,
}

impl Object {
    pub fn new(class_name: SymbolID, instance_variables: HashMap<SymbolID, RubyValue>) -> Self {
       Self {class_name, instance_variables} 
    }

    pub fn get_class_name(&self) -> SymbolID {
        self.class_name
    }

    pub fn get_instance_variables(&self) -> &HashMap<SymbolID, RubyValue> {
        &self.instance_variables
    }

    pub fn get_instance_variable(&self, symbol_id: SymbolID) -> Option<&RubyValue> {
        self.instance_variables.get(&symbol_id)
    }
}

#[derive(PartialEq, Debug)]
pub struct UserClass {
    name: SymbolID,
    wrapped_object: RubyValue,
    instance_variables: Option<HashMap<SymbolID, RubyValue>>,
}

impl UserClass {
    pub fn new(name: SymbolID, wrapped_object: RubyValue) -> Self {
       Self {name, wrapped_object, instance_variables: None } 
    }

    pub fn get_name(&self) -> SymbolID {
        self.name
    }

    pub fn get_wrapped_object(&self) -> &RubyValue {
        &self.wrapped_object
    }

    pub fn set_instance_variables(&mut self, instance_variables: HashMap<SymbolID, RubyValue>) {
        self.instance_variables = Some(instance_variables);
    }


    pub fn get_instance_variables(&self) -> &Option<HashMap<SymbolID, RubyValue>> {
        &self.instance_variables
    }

    pub fn get_instance_variable(&self, symbol_id: SymbolID) -> Option<&RubyValue> {
        if let Some(instance_variables) = &self.instance_variables {
            instance_variables.get(&symbol_id)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct UserDefined {
    class_name: SymbolID,
    data: Vec<u8>,
    instance_variables: Option<HashMap<SymbolID, RubyValue>>,
}

impl UserDefined {
    pub fn new(class_name: SymbolID, data: Vec<u8>) -> Self {
       Self {class_name, data, instance_variables: None} 
    }

    pub fn get_class_name(&self) -> SymbolID {
        self.class_name
    }

    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn set_instance_variables(&mut self, instance_variables: HashMap<SymbolID, RubyValue>) {
        self.instance_variables = Some(instance_variables);
    }


    pub fn get_instance_variables(&self) -> &Option<HashMap<SymbolID, RubyValue>> {
        &self.instance_variables
    }

    pub fn get_instance_variable(&self, symbol_id: SymbolID) -> Option<&RubyValue> {
        if let Some(instance_variables) = &self.instance_variables {
            instance_variables.get(&symbol_id)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct UserMarshal {
    class_name: SymbolID,
    wrapped_object: RubyValue,
}

impl UserMarshal {
    pub fn new(class_name: SymbolID, wrapped_object: RubyValue) -> Self {
       Self {class_name, wrapped_object } 
    }

    pub fn get_class_name(&self) -> SymbolID {
        self.class_name
    }

    pub fn get_wrapped_object(&self) -> &RubyValue {
        &self.wrapped_object
    }
}
