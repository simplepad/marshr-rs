use std::{cell::RefCell, collections::{btree_map::Keys, HashMap}, fmt::{Display, Write}, ops::{Index, IndexMut}, rc::Rc};

pub type Rrc<T> = Rc<RefCell<T>>;

pub type ObjectID = usize;
pub type SymbolID = usize;

#[derive(PartialEq, Eq, Hash, Debug)]
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
    Uninitialized(ObjectID), // for recursion
}

#[derive(PartialEq, Debug)]
pub enum RubyObject {
    Empty, // for the 0th element (ruby object index starts with 1)
    Array(Vec<RubyValue>),
    Hash(HashMap<RubyValue, RubyValue>),
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

impl RubyObject {
    fn as_array(&self) -> &Vec<RubyValue> {
        match self {
            RubyObject::Array(object) => object,
            _ => panic!("Not an array"),
        }
    }

    fn as_hash(&self) -> &HashMap<RubyValue, RubyValue> {
        match self {
            RubyObject::Hash(object) => object,
            _ => panic!("Not a hash"),
        }
    }

    fn as_hash_with_default(&self) -> &HashWithDefault {
        match self {
            RubyObject::HashWithDefault(object) => object,
            _ => panic!("Not a hash with default"),
        }
    }

    fn as_float(&self) -> f64 {
        match self {
            RubyObject::Float(object) => *object,
            _ => panic!("Not a float"),
        }
    }

    fn as_class(&self) -> &String {
        match self {
            RubyObject::Class(object) => object,
            _ => panic!("Not a class"),
        }
    }

    fn as_module(&self) -> &String {
        match self {
            RubyObject::Module(object) => object,
            _ => panic!("Not a module"),
        }
    }

    fn as_class_or_module(&self) -> &String {
        match self {
            RubyObject::ClassOrModule(object) => object,
            _ => panic!("Not a class or module"),
        }
    }

    fn as_string(&self) -> &RubyString {
        match self {
            RubyObject::String(object) => object,
            _ => panic!("Not a string"),
        }
    }

    fn as_bignum(&self) -> i64 {
        match self {
            RubyObject::BigNum(object) => *object,
            _ => panic!("Not a bignum"),
        }
    }

    fn as_regexp(&self) -> &RegExp {
        match self {
            RubyObject::RegExp(object) => object,
            _ => panic!("Not a regexp"),
        }
    }

    fn as_struct(&self) -> &Struct {
        match self {
            RubyObject::Struct(object) => object,
            _ => panic!("Not a struct"),
        }
    }

    fn as_object(&self) -> &Object {
        match self {
            RubyObject::Object(object) => object,
            _ => panic!("Not an object"),
        }
    }

    fn as_user_class(&self) -> &UserClass {
        match self {
            RubyObject::UserClass(object) => object,
            _ => panic!("Not a user class"),
        }
    }

    fn as_user_defined(&self) -> &UserDefined {
        match self {
            RubyObject::UserDefined(object) => object,
            _ => panic!("Not a user defined"),
        }
    }

    fn as_user_marshal(&self) -> &UserMarshal {
        match self {
            RubyObject::UserMarshal(object) => object,
            _ => panic!("Not a user marshal"),
        }
    }
}

impl Display for RubyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RubyValue::Nil => f.write_str("nil"),
            RubyValue::Boolean(boolean) => f.write_str(&format!("{}", boolean)),
            RubyValue::FixNum(num) => f.write_str(&format!("{}", num)),
            _ => panic!("Display not implemented for this RubyValue type, use Display on corresponding RubyObject"),
        }
    }
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

    pub fn print(&self, value: &RubyValue, f: &mut impl Write) -> Result<(), std::fmt::Error> {
        match value {
            RubyValue::Nil | RubyValue::FixNum(_) | RubyValue::Boolean(_) => f.write_str(&format!("{}", value)),
            RubyValue::Symbol(symbol_id) => f.write_str(&self.symbols[*symbol_id]),
            RubyValue::Array(object_id) => {
                let array = self.objects[*object_id].as_array();
                if !array.is_empty() {
                    f.write_str("Array [ ")?;
                    for (i, obj) in array.iter().enumerate() {
                        self.print(obj, f)?;
                        if i != array.len() - 1 {
                            f.write_str(", ")?;
                        }
                    }
                    f.write_str(" ]")?;
                } else {
                    f.write_str("Array []")?;
                }
                Ok(())
            },
            RubyValue::BigNum(object_id) => f.write_str(&self.objects[*object_id].as_bignum().to_string()),
            RubyValue::Class(object_id) => f.write_str(&format!("Class {}", self.objects[*object_id].as_class())),
            RubyValue::Module(object_id) => f.write_str(&format!("Module {}", self.objects[*object_id].as_module())),
            RubyValue::ClassOrModule(object_id) => f.write_str(&format!("ClassOrModule {}", self.objects[*object_id].as_class_or_module())),
            RubyValue::Float(object_id) => f.write_str(&self.objects[*object_id].as_float().to_string()),
            RubyValue::Hash(object_id) => {
                let hash = self.objects[*object_id].as_hash();
                f.write_str("Hash { ")?;
                for (i, (key, value)) in hash.iter().enumerate() {
                    self.print(key, f)?;
                    f.write_str(": ")?;
                    self.print(value, f)?;
                    if i != hash.len() - 1 {
                        f.write_str(", ")?;
                    }
                }
                f.write_str(" }")?;
                Ok(())
            },
            RubyValue::HashWithDefault(object_id) => {
                let hash = self.objects[*object_id].as_hash_with_default();
                f.write_str("HashWithDefault { ")?;
                for (i, (key, value)) in hash.hash.iter().enumerate() {
                    self.print(key, f)?;
                    f.write_str(": ")?;
                    self.print(value, f)?;
                    f.write_str(", ")?;
                }
                f.write_str("default: ")?;
                self.print(&hash.default, f)?;
                f.write_str(" }")?;
                Ok(())
            },
            RubyValue::Object(object_id) => {
                let object = self.objects[*object_id].as_object();
                f.write_str("Object { ")?;
                f.write_str("class_name: ")?;
                self.print(&RubyValue::Symbol(object.class_name), f)?;
                f.write_str(", instance_variables: [ ")?;
                for (i, (key, value)) in object.instance_variables.iter().enumerate() {
                    self.print(&RubyValue::Symbol(*key), f)?;
                    f.write_str(": ")?;
                    self.print(value, f)?;
                    f.write_str(", ")?;
                }
                f.write_str(" ] }")?;
                Ok(())
            },
            RubyValue::RegExp(object_id) => {
                let regexp = self.objects[*object_id].as_regexp();
                f.write_str("RegExp { ")?;
                f.write_str("pattern: ")?;
                f.write_str(&regexp.pattern)?;
                f.write_str(", options: ")?;
                f.write_str(&regexp.options.to_string())?;
                if let Some(instance_variables) = &regexp.instance_variables {
                    f.write_str(", instance_variables: [ ")?;
                    for (i, (key, value)) in instance_variables.iter().enumerate() {
                        self.print(&RubyValue::Symbol(*key), f)?;
                        f.write_str(": ")?;
                        self.print(value, f)?;
                        f.write_str(", ")?;
                    }
                    f.write_str(" ] }")?;
                } else {
                    f.write_str(" }")?;
                }
                Ok(())
            },
            RubyValue::String(object_id) => {
                let string = self.objects[*object_id].as_string();
                f.write_str(&format!("\"{}\"", string.string))?;
                Ok(())
            },
            RubyValue::Struct(object_id) => {
                let ruby_struct = self.objects[*object_id].as_struct();
                f.write_str("Stuct { ")?;
                f.write_str(&format!("name: {}", ruby_struct.name))?;
                f.write_str(", members: [ ")?;
                for (i, (key, value)) in ruby_struct.members.iter().enumerate() {
                    self.print(&RubyValue::Symbol(*key), f)?;
                    f.write_str(": ")?;
                    self.print(value, f)?;
                    f.write_str(", ")?;
                }
                f.write_str(" ] }")?;
                Ok(())
            },
            RubyValue::UserClass(object_id) => {
                let user_class = self.objects[*object_id].as_user_class();
                f.write_str("UserClass { ")?;
                f.write_str("name: ")?;
                self.print(&RubyValue::Symbol(user_class.name), f)?;
                f.write_str(", wrapped_object: ")?;
                self.print(&user_class.wrapped_object, f)?;
                if let Some(instance_variables) = &user_class.instance_variables {
                    f.write_str(", instance_variables: [ ")?;
                    for (i, (key, value)) in instance_variables.iter().enumerate() {
                        self.print(&RubyValue::Symbol(*key), f)?;
                        f.write_str(": ")?;
                        self.print(value, f)?;
                        f.write_str(", ")?;
                    }
                    f.write_str(" ] }")?;
                } else {
                    f.write_str(" }")?;
                }
                Ok(())
            },
            RubyValue::UserDefined(object_id) => {
                let user_defined = self.objects[*object_id].as_user_defined();
                f.write_str("UserDefined { ")?;
                f.write_str("class_name: ")?;
                self.print(&RubyValue::Symbol(user_defined.class_name), f)?;
                f.write_str(&format!(", data: {:?}", user_defined.data))?;
                if let Some(instance_variables) = &user_defined.instance_variables {
                    f.write_str(", instance_variables: [ ")?;
                    for (i, (key, value)) in instance_variables.iter().enumerate() {
                        self.print(&RubyValue::Symbol(*key), f)?;
                        f.write_str(": ")?;
                        self.print(value, f)?;
                        f.write_str(", ")?;
                    }
                    f.write_str(" ] }")?;
                } else {
                    f.write_str(" }")?;
                }
                Ok(())
            },
            RubyValue::UserMarshal(object_id) => {
                let user_marshal = self.objects[*object_id].as_user_marshal();
                f.write_str("UserMarshal { ")?;
                f.write_str("class_name: ")?;
                self.print(&RubyValue::Symbol(user_marshal.class_name), f)?;
                f.write_str(", wrapped_object: ")?;
                self.print(&user_marshal.wrapped_object, f)?;
                f.write_str(" }")?;
                Ok(())
            },
            RubyValue::Uninitialized(_object_id) => {
                f.write_str("RECURSION")
            },
        }
    }
}

impl Display for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.print(&self.root, f)
    }
}

#[derive(PartialEq, Debug)]
pub struct HashWithDefault {
    hash: HashMap<RubyValue, RubyValue>,
    default: RubyValue,
}

impl HashWithDefault {
    pub fn new(hash: HashMap<RubyValue, RubyValue>, default: RubyValue) -> Self {
        Self { hash, default }
    }

    pub fn len(&self) -> usize {
        self.hash.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &RubyValue> {
        self.hash.keys()
    }
}

impl<'a> Index<&'a RubyValue> for HashWithDefault {
    type Output = RubyValue;
    fn index(&self, index: &'a RubyValue) -> &Self::Output {
        self.hash.get(index).unwrap_or(&self.default)
    }
}

impl<'a> IndexMut<&'a RubyValue> for HashWithDefault {
    fn index_mut(&mut self, index: &'a RubyValue) -> &mut Self::Output {
        self.hash.get_mut(index).unwrap_or(&mut self.default)
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
