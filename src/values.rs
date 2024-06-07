use std::{collections::HashMap, fmt::{Display, Write}, ops::{Index, IndexMut}};

use encoding::{label::encoding_from_whatwg_label, DecoderTrap, Encoding};

pub const MARSHAL_MAJOR_VERSION: u8 = 4;
pub const MARSHAL_MINOR_VERSION: u8 = 8;

pub type ObjectID = usize;
pub type SymbolID = usize;

#[derive(Debug)]
pub enum RubyError {
    EncodingError(String)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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

impl RubyValue {
    pub fn as_boolean(&self) -> bool {
        match self {
            RubyValue::Boolean(val) => *val,
            _ => panic!("Not a boolean"),
        }
    }

    pub fn as_fixnum(&self) -> i32 {
        match self {
            RubyValue::FixNum(val) => *val,
            _ => panic!("Not a fixnum"),
        }
    }

    pub fn as_symbol(&self) -> SymbolID {
        match self {
            RubyValue::Symbol(val) => *val,
            _ => panic!("Not a symbol"),
        }
    }

    pub fn as_array(&self) -> ObjectID {
        match self {
            RubyValue::Array(val) => *val,
            _ => panic!("Not an array"),
        }
    }

    pub fn as_bignum(&self) -> ObjectID {
        match self {
            RubyValue::BigNum(val) => *val,
            _ => panic!("Not a bignum"),
        }
    }

    pub fn as_class(&self) -> ObjectID {
        match self {
            RubyValue::Class(val) => *val,
            _ => panic!("Not a class"),
        }
    }

    pub fn as_module(&self) -> ObjectID {
        match self {
            RubyValue::Module(val) => *val,
            _ => panic!("Not a module"),
        }
    }
    
    pub fn as_class_or_module(&self) -> ObjectID {
        match self {
            RubyValue::ClassOrModule(val) => *val,
            _ => panic!("Not a class or module"),
        }
    }

    pub fn as_float(&self) -> ObjectID {
        match self {
            RubyValue::Float(val) => *val,
            _ => panic!("Not a float"),
        }
    }

    pub fn as_hash(&self) -> ObjectID {
        match self {
            RubyValue::Hash(val) => *val,
            _ => panic!("Not a hash"),
        }
    }

    pub fn as_hash_with_default(&self) -> ObjectID {
        match self {
            RubyValue::HashWithDefault(val) => *val,
            _ => panic!("Not a hash with default"),
        }
    }

    pub fn as_object(&self) -> ObjectID {
        match self {
            RubyValue::Object(val) => *val,
            _ => panic!("Not an object"),
        }
    }

    pub fn as_regexp(&self) -> ObjectID {
        match self {
            RubyValue::RegExp(val) => *val,
            _ => panic!("Not a regexp"),
        }
    }

    pub fn as_string(&self) -> ObjectID {
        match self {
            RubyValue::String(val) => *val,
            _ => panic!("Not a string"),
        }
    }

    pub fn as_struct(&self) -> ObjectID {
        match self {
            RubyValue::Struct(val) => *val,
            _ => panic!("Not a struct"),
        }
    }

    pub fn as_user_class(&self) -> ObjectID {
        match self {
            RubyValue::UserClass(val) => *val,
            _ => panic!("Not a user class"),
        }
    }

    pub fn as_user_defined(&self) -> ObjectID {
        match self {
            RubyValue::UserDefined(val) => *val,
            _ => panic!("Not a user defined"),
        }
    }

    pub fn as_user_marshal(&self) -> ObjectID {
        match self {
            RubyValue::UserMarshal(val) => *val,
            _ => panic!("Not a user marshal"),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
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
    pub fn as_array(&self) -> &Vec<RubyValue> {
        match self {
            RubyObject::Array(object) => object,
            _ => panic!("Not an array"),
        }
    }

    pub fn as_hash(&self) -> &HashMap<RubyValue, RubyValue> {
        match self {
            RubyObject::Hash(object) => object,
            _ => panic!("Not a hash"),
        }
    }

    pub fn as_hash_with_default(&self) -> &HashWithDefault {
        match self {
            RubyObject::HashWithDefault(object) => object,
            _ => panic!("Not a hash with default"),
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            RubyObject::Float(object) => *object,
            _ => panic!("Not a float"),
        }
    }

    pub fn as_class(&self) -> &String {
        match self {
            RubyObject::Class(object) => object,
            _ => panic!("Not a class"),
        }
    }

    pub fn as_module(&self) -> &String {
        match self {
            RubyObject::Module(object) => object,
            _ => panic!("Not a module"),
        }
    }

    pub fn as_class_or_module(&self) -> &String {
        match self {
            RubyObject::ClassOrModule(object) => object,
            _ => panic!("Not a class or module"),
        }
    }

    pub fn as_string(&self) -> &RubyString {
        match self {
            RubyObject::String(object) => object,
            _ => panic!("Not a string"),
        }
    }

    pub fn as_bignum(&self) -> i64 {
        match self {
            RubyObject::BigNum(object) => *object,
            _ => panic!("Not a bignum"),
        }
    }

    pub fn as_regexp(&self) -> &RegExp {
        match self {
            RubyObject::RegExp(object) => object,
            _ => panic!("Not a regexp"),
        }
    }

    pub fn as_struct(&self) -> &Struct {
        match self {
            RubyObject::Struct(object) => object,
            _ => panic!("Not a struct"),
        }
    }

    pub fn as_object(&self) -> &Object {
        match self {
            RubyObject::Object(object) => object,
            _ => panic!("Not an object"),
        }
    }

    pub fn as_user_class(&self) -> &UserClass {
        match self {
            RubyObject::UserClass(object) => object,
            _ => panic!("Not a user class"),
        }
    }

    pub fn as_user_defined(&self) -> &UserDefined {
        match self {
            RubyObject::UserDefined(object) => object,
            _ => panic!("Not a user defined"),
        }
    }

    pub fn as_user_marshal(&self) -> &UserMarshal {
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

    pub fn get_symbol_id(&self, symbol: &str) -> Option<SymbolID> {
        for (i, s) in self.symbols.iter().enumerate() {
            if *s == symbol {
                return Some(i);
            }
        }
        None
    }

    pub fn get_object(&self, id: ObjectID) -> Option<&RubyObject> {
        self.objects.get(id)
    }

    pub fn decode_string(&self, string: &RubyString) -> Result<String, RubyError> {
        if let Some(string_instance_variables) = string.get_instance_variables() {
            return self.decode_string_with_instance_variables(string, string_instance_variables);
        }
        Err(RubyError::EncodingError("Tried to decode a string in a binary encoding".to_string()))
    }

    fn decode_string_with_instance_variables(&self, string: &RubyString, instance_variables: &HashMap<SymbolID, RubyValue>) -> Result<String, RubyError> {
        if string.get_string().is_empty() {
            return Ok(String::new());
        }
        if let Some(encoding_symbol_id) = self.get_symbol_id("E") {
            if let Some(encoding) = instance_variables.get(&encoding_symbol_id) {
                let RubyValue::Boolean(boolean) = encoding else { panic!("Symbol E for string was not boolean")} ;
                if *boolean {
                    return Ok(encoding::all::UTF_8.decode(string.get_string(), DecoderTrap::Strict).unwrap());
                } else {
                    return Ok(encoding::all::ASCII.decode(string.get_string(), DecoderTrap::Strict).unwrap());
                }
            }
        }
        if let Some(encoding_symbol_id) = self.get_symbol_id("encoding") {
            if let Some(encoding) = instance_variables.get(&encoding_symbol_id) {
                let RubyValue::String(encoding) = encoding else { panic!("Symbol encoding for string was not a string") };
                let encoding = self.objects[*encoding].as_string();
                let encoding_string = self.decode_string(encoding).unwrap(); // should be raw encoded
                if let Some(encoding) = encoding_from_whatwg_label(&encoding_string) {
                    return Ok(encoding.decode(string.get_string(), DecoderTrap::Strict).unwrap())
                } else {
                    return Err(RubyError::EncodingError(format!("Could not find encoding {}", encoding_string)))
                }
            }
        }
        Err(RubyError::EncodingError("Tried to decode a string in a binary encoding".to_string()))

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
                for (key, value) in hash.hash.iter() {
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
                for (key, value) in object.instance_variables.iter() {
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
                    for (key, value) in instance_variables.iter() {
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
                f.write_str(&format!("\"{}\"", self.decode_string(string).unwrap()))?;
                Ok(())
            },
            RubyValue::Struct(object_id) => {
                let ruby_struct = self.objects[*object_id].as_struct();
                f.write_str("Stuct { ")?;
                f.write_str(&format!("name: {}", ruby_struct.name))?;
                f.write_str(", members: [ ")?;
                for (key, value) in ruby_struct.members.iter() {
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
                    for (key, value) in instance_variables.iter() {
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
                    for (key, value) in instance_variables.iter() {
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

#[derive(PartialEq, Clone, Debug)]
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

    pub fn is_empty(&self) -> bool {
        self.hash.is_empty()
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

#[derive(PartialEq, Clone, Debug)]
pub struct RubyString {
    string: Vec<u8>,
    instance_variables: Option<HashMap<SymbolID, RubyValue>>,
}

impl RubyString {
    pub fn new(string: Vec<u8>) -> Self {
        Self {string, instance_variables: None}
    }

    pub fn get_string(&self) -> &Vec<u8> {
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

#[derive(PartialEq, Clone, Debug)]
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

#[derive(PartialEq, Clone, Debug)]
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

#[derive(PartialEq, Clone, Debug)]
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

#[derive(PartialEq, Clone, Debug)]
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

    pub fn decode_wrapped_string(&self, root: &Root) -> Result<String, RubyError> {
        if let Some(instance_variables) = &self.instance_variables {
            let inner_string = root.get_object(self.wrapped_object.as_string()).unwrap().as_string();
            root.decode_string_with_instance_variables(inner_string, instance_variables)
        } else {
            Err(RubyError::EncodingError("Tried to decode a string in a binary encoding".to_string()))
        }
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

#[derive(PartialEq, Clone, Debug)]
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

#[derive(PartialEq, Clone, Debug)]
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
