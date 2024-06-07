use std::{collections::HashMap, fmt::Display, io::Read};
use crate::values::*;

#[derive(Debug)]
pub enum LoadError {
    IoError(String),
    ParserError(String),
}

impl From<std::string::FromUtf8Error> for LoadError {
    fn from(_value: std::string::FromUtf8Error) -> Self {
        Self::ParserError(format!("Could not decode bytes into a String: {}", _value))
    }
}

impl From<std::num::ParseFloatError> for LoadError {
    fn from(_value: std::num::ParseFloatError) -> Self {
        Self::ParserError("Could not parse float from sequence".to_string())
    }
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::ParserError(error) => {
                f.write_str(&format!("Parser Error: {}", error))
            }
            LoadError::IoError(error) => {
                f.write_str(&format!("IO Error: {}", error))
            }
        }
    }
}

pub struct Loader<T: Read> {
    reader: T,
    symbols: Vec<String>,
    objects: Vec<RubyObject>,
}

impl<T: Read> Loader<T> {
    pub fn new(reader: T) -> Self {
        Loader {
            reader,
            symbols: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub fn load(mut self) -> Result<Root, LoadError> {
        let mut buffer: [u8; 2] = [0; 2];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read Marshal version: {}", err)));
        }

        if buffer[0] > 4 || buffer[1] > 8 {
            return Err(LoadError::ParserError("Unsupported Marshal version".to_string()));
        }

        let value = self.read_value()?;

        Ok(Root::new(value, self.symbols, self.objects))
    }

    fn read_value(&mut self) -> Result<RubyValue, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read value type: {}", err)));
        }

        let value = match buffer[0] {
            b'0' => RubyValue::Nil,
            b'T' => RubyValue::Boolean(true),
            b'F' => RubyValue::Boolean(false),
            b'i' => RubyValue::FixNum(self.read_fixnum()?),
            b':' => RubyValue::Symbol(self.read_symbol()?),
            b';' => RubyValue::Symbol(self.read_symbol_link()?),
            b'[' => RubyValue::Array(self.read_array()?),
            b'f' => RubyValue::Float(self.read_float()?),
            b'@' => self.read_object_link()?,
            b'{' => RubyValue::Hash(self.read_hash()?),
            b'}' => RubyValue::HashWithDefault(self.read_hash_with_default()?),
            b'c' => RubyValue::Class(self.read_class()?),
            b'm' => RubyValue::Module(self.read_module()?),
            b'M' => RubyValue::ClassOrModule(self.read_class_or_module()?),
            b'"' => RubyValue::String(self.read_string()?),
            b'I' => self.read_value_with_instance_variables()?,
            b'l' => RubyValue::BigNum(self.read_bignum()?),
            b'/' => RubyValue::RegExp(self.read_regexp()?),
            b'S' => RubyValue::Struct(self.read_struct()?),
            b'o' => RubyValue::Object(self.read_object()?),
            b'C' => RubyValue::UserClass(self.read_user_class()?),
            b'u' => RubyValue::UserDefined(self.read_user_defined()?),
            b'U' => RubyValue::UserMarshal(self.read_user_marshal()?),
            b'd' => return Err(LoadError::ParserError("This parser doesn't support Data objects".to_string())),
            _ => return Err(LoadError::ParserError(format!("Unknown value type: {}", buffer[0]))),
        };

        Ok(value)
    }

    fn read_fixnum(&mut self) -> Result<i32, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read fixnum's first byte: {}", err)));
        }

        if buffer[0] == 0 {
            return Ok(0);
        }

        let mut is_positive = true;
        let mut int_len = buffer[0];

        if (int_len as i8) < 0 {
            int_len = int_len.wrapping_neg();
            is_positive = false;
        }

        if int_len > 0 && int_len < 5 {
            let mut buffer = [0; 4];
            if let Err(err) = self.reader.read_exact(&mut buffer[..int_len.into()]) {
                return Err(LoadError::IoError(format!("Failed to read fixnum's following bytes: {}", err)));
            }

            if is_positive {
                Ok(i32::from_le_bytes(buffer))
            } else {
                let mut n: i32 = -1;
                for i in 0..int_len {
                    n &= !(0xFF_i32 << (i * 8));
                    n |= i32::from(buffer[i as usize]) << (i * 8);
                }

                Ok(n)
            }
        } else {
            let value = i8::from_le_bytes([int_len]);

            if value > 0 {
                Ok(value as i32 - 5)
            } else {
                Ok(value as i32 + 5)
            }
        }
    }

    fn read_byte_sequence(&mut self) -> Result<Vec<u8>, LoadError> {
        let sequence_len = self.read_fixnum()?.try_into().unwrap();
        let mut buffer = vec![0; sequence_len];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read byte sequence: {}, was expecting {} bytes", err, sequence_len)));
        }
        Ok(buffer)
    }

    fn read_sequence(&mut self) -> Result<String, LoadError> {
        let byte_sequence = self.read_byte_sequence()?;
        let sequence = String::from_utf8(byte_sequence)?;
        Ok(sequence)
    }

    fn read_symbol(&mut self) -> Result<SymbolID, LoadError> {
        let symbol = self.read_sequence()?;

        self.symbols.push(symbol);
        Ok(self.symbols.len()-1)
    }

    fn read_symbol_link(&mut self) -> Result<SymbolID, LoadError> {
        let symbol_id = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse symbol link (could not convert symbol index to usize)".to_string())),
        };

        if symbol_id >= self.symbols.len() {
            Err(LoadError::ParserError("Could not parse symbol link (links to a non-existent symbol)".to_string()))
        } else {
            Ok(symbol_id)
        }
    }

    fn read_array(&mut self) -> Result<ObjectID, LoadError> {
        let array_len = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse array length (could not convert array length to usize)".to_string())),
        };

        self.objects.push(RubyObject::Empty);
        let array_id = self.objects.len()-1;

        let mut array = Vec::with_capacity(array_len);

        for _ in 0..array_len {
            array.push(self.read_value()?);
        }

        self.objects[array_id] = RubyObject::Array(array);
        Ok(array_id)
    }

    fn read_float(&mut self) -> Result<ObjectID, LoadError> {
        let mut float_sequence = self.read_byte_sequence()?;

        let float_val = match float_sequence.as_slice() {
            b"inf" => f64::INFINITY,
            b"-inf" => f64::NEG_INFINITY,
            b"nan" => f64::NAN,
            _ => {
                // replicating ruby's parsing
                float_sequence.push(0); // make sure its null-terminated
                let value: f64 = unsafe { libc::strtod(float_sequence.as_ptr() as *const i8, std::ptr::null_mut()) };
                value
            } 
        };

        self.objects.push(RubyObject::Float(float_val));
        Ok(self.objects.len()-1)
    }

    fn read_object_link(&mut self) -> Result<RubyValue, LoadError> {
        let object_id = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse object link (could not convert object index to usize)".to_string())),
        };

        if let Some(object) = self.objects.get(object_id) {
            let ruby_value = match object {
                RubyObject::Empty => RubyValue::Uninitialized(object_id), // recursion
                RubyObject::Array(_) => RubyValue::Array(object_id),
                RubyObject::Float(_) => RubyValue::Float(object_id),
                RubyObject::Hash(_) => RubyValue::Hash(object_id),
                RubyObject::HashWithDefault(_) => RubyValue::HashWithDefault(object_id),
                RubyObject::Class(_) => RubyValue::Class(object_id),
                RubyObject::Module(_) => RubyValue::Module(object_id),
                RubyObject::ClassOrModule(_) => RubyValue::ClassOrModule(object_id),
                RubyObject::String(_) => RubyValue::String(object_id),
                RubyObject::BigNum(_) => RubyValue::BigNum(object_id),
                RubyObject::RegExp(_) => RubyValue::RegExp(object_id),
                RubyObject::Struct(_) => RubyValue::Struct(object_id),
                RubyObject::Object(_) => RubyValue::Object(object_id),
                RubyObject::UserClass(_) => RubyValue::UserClass(object_id),
                RubyObject::UserDefined(_) => RubyValue::UserDefined(object_id),
                RubyObject::UserMarshal(_) => RubyValue::UserMarshal(object_id),
            };
            Ok(ruby_value)
        } else {
            Err(LoadError::ParserError("Could not parse object link (links to a non-existent object)".to_string()))
        }
    }

    fn read_value_pairs(&mut self) -> Result<HashMap<RubyValue, RubyValue>, LoadError> {
        let num_of_pairs = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse number of key:value pairs (could not convert number of pairs to usize)".to_string())),
        };

        let mut pairs = HashMap::with_capacity(num_of_pairs);

        for _ in 0..num_of_pairs {
            let key = self.read_value()?;
            let value = self.read_value()?;

            pairs.insert(key, value);
        }

        Ok(pairs)
    }

    fn read_value_pairs_symbol_keys(&mut self) -> Result<HashMap<SymbolID, RubyValue>, LoadError> {
        let num_of_pairs = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse number of key:value pairs (could not convert number of pairs to usize)".to_string())),
        };

        let mut pairs = HashMap::with_capacity(num_of_pairs);

        for _ in 0..num_of_pairs {
            let symbol = match self.read_value()? {
                RubyValue::Symbol(symbol_id) => symbol_id,
                other => return Err(LoadError::ParserError(format!("Could not parse key:value pairs, key was not a Symbol: {:?}", other)))
            };
            let value = self.read_value()?;

            pairs.insert(symbol, value);
        }

        Ok(pairs)
    }

    fn read_hash(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let hash_id = self.objects.len()-1;

        let hash = self.read_value_pairs()?;

        self.objects[hash_id] = RubyObject::Hash(hash);
        Ok(hash_id)
    }

    fn read_hash_with_default(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let hash_id = self.objects.len()-1;

        let hash = self.read_value_pairs()?;

        let default = self.read_value()?;

        self.objects[hash_id] = RubyObject::HashWithDefault(HashWithDefault::new(hash, default));
        Ok(hash_id)
    }

    fn read_class(&mut self) -> Result<ObjectID, LoadError> {
        let class = self.read_sequence()?;

        self.objects.push(RubyObject::Class(class));
        Ok(self.objects.len()-1)
    }

    fn read_module(&mut self) -> Result<ObjectID, LoadError> {
        let module = self.read_sequence()?;

        self.objects.push(RubyObject::Module(module));
        Ok(self.objects.len()-1)
    }

    fn read_class_or_module(&mut self) -> Result<ObjectID, LoadError> {
        let class_or_module = self.read_sequence()?;

        self.objects.push(RubyObject::ClassOrModule(class_or_module));
        Ok(self.objects.len()-1)
    }

    fn read_string(&mut self) -> Result<ObjectID, LoadError> {
        let string = self.read_byte_sequence()?;

        self.objects.push(RubyObject::String(RubyString::new(string)));
        Ok(self.objects.len()-1)
    }

    fn read_value_with_instance_variables(&mut self) -> Result<RubyValue, LoadError> {
        let value = self.read_value()?;

        let instance_variables = self.read_value_pairs_symbol_keys()?;
        match value {
            RubyValue::String(object_id) => {
                match &mut self.objects[object_id] {
                    RubyObject::String(string) => {
                        string.set_instance_variables(instance_variables);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            RubyValue::RegExp(object_id) => {
                match &mut self.objects[object_id] {
                    RubyObject::RegExp(regexp) => {
                        regexp.set_instance_variables(instance_variables);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            RubyValue::UserClass(object_id) => {
                match &mut self.objects[object_id] {
                    RubyObject::UserClass(user_class) => {
                        user_class.set_instance_variables(instance_variables);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            RubyValue::UserDefined(object_id) => {
                match &mut self.objects[object_id] {
                    RubyObject::UserDefined(user_defined) => {
                        user_defined.set_instance_variables(instance_variables);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            object => return Err(LoadError::ParserError(format!("Object {:?} doesn't support instance variables", object)))
        }

        Ok(value)
    }

    fn read_bignum(&mut self) -> Result<ObjectID, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read bignum's sign byte: {}", err)));
        }

        let is_positive = match buffer[0] {
            b'+' => true,
            b'-' => false,
            _ => return Err(LoadError::ParserError(format!("Could not parse bignum's sign byte, got \"{}\"", buffer[0]))),
        };

        let length = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val * 2,
            Err(_) => return Err(LoadError::ParserError("Could not parse array length (could not convert array length to usize)".to_string())),
        };

        let mut buffer = vec![0; length];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read bignum: {}, was expecting {} bytes", err, length)));
        }

        let mut value: i64 = 0;

        for (i, byte) in buffer.iter().enumerate() {
            let shift_bits = match u32::try_from(i * 8) {
                Ok(val) => val,
                Err(_) => return Err(LoadError::ParserError("Could not parse bignum, exponent was too big".to_string())),
            };
            value += (*byte as i64) << shift_bits;
        }

        if !is_positive {
            value *= -1;
        }

        self.objects.push(RubyObject::BigNum(value));
        Ok(self.objects.len()-1)
    }

    fn read_regexp(&mut self) -> Result<ObjectID, LoadError> {
        let pattern = self.read_sequence()?;

        let mut buffer: [u8; 1] = [0; 1];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read regexp's options byte: {}", err)));
        }

        let options = buffer[0] as i8;

        self.objects.push(RubyObject::RegExp(RegExp::new(pattern, options)));
        Ok(self.objects.len()-1)
    }

    fn read_struct(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let struct_id = self.objects.len()-1;

        let name = match self.read_value()? {
            RubyValue::Symbol(symbol_id) => symbol_id,
            value => return Err(LoadError::ParserError(format!("Could not parse struct, expected a symbol or a symbol link, got {:?}", value)))
        };

        let struct_members = self.read_value_pairs_symbol_keys()?;

        self.objects[struct_id] = RubyObject::Struct(Struct::new(name, struct_members));
        Ok(struct_id)
    }

    fn read_object(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let object_id = self.objects.len()-1;

        let class_name = match self.read_value()? {
            RubyValue::Symbol(symbol_id) => symbol_id,
            value => return Err(LoadError::ParserError(format!("Could not parse object, expected a symbol or a symbol link, got {:?}", value)))
        };

        let instance_variables = self.read_value_pairs_symbol_keys()?;

        self.objects[object_id] = RubyObject::Object(Object::new(class_name, instance_variables));
        Ok(object_id)
    }

    fn read_user_class(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let user_class_id = self.objects.len()-1;

        let name = match self.read_value()? {
            RubyValue::Symbol(symbol_id) => symbol_id,
            value => return Err(LoadError::ParserError(format!("Could not parse user class, expected a symbol or a symbol link, got {:?}", value)))
        };

        let wrapped_object = self.read_value()?;

        self.objects[user_class_id] = RubyObject::UserClass(UserClass::new(name, wrapped_object));
        Ok(user_class_id)
    }

    fn read_user_defined(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let user_defined_id = self.objects.len()-1;

        let class_name = match self.read_value()? {
            RubyValue::Symbol(symbol_id) => symbol_id,
            value => return Err(LoadError::ParserError(format!("Could not parse user defined, expected a symbol or a symbol link, got {:?}", value)))
        };

        let data = self.read_byte_sequence()?;

        self.objects[user_defined_id] = RubyObject::UserDefined(UserDefined::new(class_name, data));
        Ok(user_defined_id)
    }

    fn read_user_marshal(&mut self) -> Result<ObjectID, LoadError> {
        self.objects.push(RubyObject::Empty);
        let user_marshal_id = self.objects.len()-1;

        let class_name = match self.read_value()? {
            RubyValue::Symbol(symbol_id) => symbol_id,
            value => return Err(LoadError::ParserError(format!("Could not parse user marshal, expected a symbol or a symbol link, got {:?}", value)))
        };

        let wrapped_object = self.read_value()?;

        self.objects[user_marshal_id] = RubyObject::UserMarshal(UserMarshal::new(class_name, wrapped_object));
        Ok(user_marshal_id)
    }
}


#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    #[test]
    fn test_read_nil() {
        let input = b"\x04\x080";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);

        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::Nil);

        let input = b"\x04\x08a";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);

        let result = loader.load();
        assert!(result.is_err());
        if ! matches!(result.unwrap_err(), LoadError::ParserError(_)) {
            panic!("Got wrong error type");
        }
    }

    #[test]
    fn test_read_boolean() {
        let input = b"\x04\x08T";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);

        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::Boolean(true));

        let input = b"\x04\x08F";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);

        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::Boolean(false));
    }

    #[test]
    fn test_read_fixnum() {
        let input = b"\x04\x08i\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(0));

        let input = b"\x04\x08i\x7f";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(122));

        let input = b"\x04\x08i\x80";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(-123));

        let input = b"\x04\x08i\x01\xc8";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(200));

        let input = b"\x04\x08i\xff\x38";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(-200));

        let input = b"\x04\x08i\x02\xe8\x80";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(33000));

        let input = b"\x04\x08i\xfe\x18\x7f";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(-33000));

        let input = b"\x04\x08i\x03\xff\xff\xff";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(16777215));

        let input = b"\x04\x08i\xfd\x01\x00\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(-16777215));

        let input = b"\x04\x08i\x04\xff\xff\xff\x3f";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(1073741823));

        let input = b"\x04\x08i\xfc\x00\x00\x00\xc0";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(-1073741824));

        let input = b"\x04\x08i\x04\x00\x00\x00\x40";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_root(), &RubyValue::FixNum(1073741824));
    }

    #[test]
    fn test_read_symbol() {
        let input = b"\x04\x08:\x0ahello";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);

        let result = loader.load().unwrap();
        let root = result.get_root();
        match root {
            RubyValue::Symbol(symbol_id) => {
                assert_eq!(*symbol_id, 0);
                assert_eq!(result.get_symbol(*symbol_id).unwrap(), "hello");
            },
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_symbol_link() {
        let input = b"\x04\x08[\x07:\x0ahello;\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();
        match result.get_root() {
            RubyValue::Array(object_id) => {
                assert_eq!(*object_id, 0);
                let array = result.get_object(*object_id).unwrap();
                match array {
                    RubyObject::Array(array) => {
                        assert_eq!(array.len(), 2);
                        for val in array {
                            match val {
                                RubyValue::Symbol(symbol_id) => {
                                    assert_eq!(result.get_symbol(*symbol_id).unwrap(), "hello")
                                }
                                _ => panic!("Got wrong value type"),
                            }
                        }
                        assert_eq!(array[0], array[1])
                    },
                    _ => panic!("Got wrong object type"),
                }
            },
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_array() {
        let input = b"\x04\x08[\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Array(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Array(array) => {
                        assert_eq!(array.len(), 0);
                    },
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
        assert_eq!(result.get_objects().len(), 1);

        let input = b"\x04\x08[\x07i\x7fi\x7f";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Array(object_id) => {
                assert_eq!(*object_id, 0);
                let array = result.get_object(*object_id).unwrap();
                match array {
                    RubyObject::Array(array) => {
                        assert_eq!(array.len(), 2);
                        for val in array {
                            match val {
                                RubyValue::FixNum(fixnum) => {
                                    assert_eq!(*fixnum, 122)
                                }
                                _ => panic!("Got wrong value type"),
                            }
                        }
                        assert_eq!(array[0], array[1])
                    },
                    _ => panic!("Got wrong object type"),
                }
            },
            _ => panic!("Got wrong value type"),
        }
        assert_eq!(result.get_objects().len(), 1); // object ids start with 1
    }

    #[test]
    fn test_read_float() {
        let input = b"\x04\x08f\x08inf";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Float(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Float(float_val) => {
                        assert_eq!(*float_val, f64::INFINITY);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08f\x09-inf";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Float(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Float(float_val) => {
                        assert_eq!(*float_val, f64::NEG_INFINITY);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08f\x08nan";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Float(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Float(float_val) => {
                        assert!(float_val.is_nan());
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08f\x092.55";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Float(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Float(float_val) => {
                        assert_eq!(*float_val, 2.55);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08[\x07f\x092.55@\x06";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Array(object_id) => {
                assert_eq!(*object_id, 0);
                let array = result.get_object(*object_id).unwrap();
                match array {
                    RubyObject::Array(array) => {
                        assert_eq!(array.len(), 2);
                        for val in array {
                            match val {
                                RubyValue::Float(object_id) => {
                                    match result.get_object(*object_id).unwrap() {
                                        RubyObject::Float(float_val) => {
                                            assert_eq!(*float_val, 2.55);
                                        }
                                        _ => panic!("Got wrong object type"),
                                    }
                                }
                                _ => panic!("Got wrong value type"),
                            }
                        }
                        assert_eq!(array[0], array[1])
                    },
                    _ => panic!("Got wrong object type"),
                }
            },
            _ => panic!("Got wrong value type"),
        }
        assert_eq!(result.get_objects().len(), 2);

    }

    #[test]
    fn test_read_hash() {
        let input = b"\x04\x08{\x06:\x06ai\x06";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Hash(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Hash(hash) => {
                        assert_eq!(hash.len(), 1);
                        let key = hash.keys().next().unwrap();
                        match key {
                            RubyValue::Symbol(symbol_id) => {
                                assert_eq!(result.get_symbol(*symbol_id).unwrap(), "a");
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        match hash[key] {
                            RubyValue::FixNum(val) => {
                                assert_eq!(val, 1)
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        assert!(hash.get(&RubyValue::FixNum(5)).is_none())
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

    }

    #[test]
    fn test_read_hash_with_default() {
        let input = b"\x04\x08}\x06:\x06ai\x06i\x07";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::HashWithDefault(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::HashWithDefault(hash) => {
                        assert_eq!(hash.len(), 1);
                        let key = hash.keys().next().unwrap();
                        match key {
                            RubyValue::Symbol(symbol_id) => {
                                assert_eq!(result.get_symbol(*symbol_id).unwrap(), "a");
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        match hash[key] {
                            RubyValue::FixNum(val) => {
                                assert_eq!(val, 1)
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        // test default value
                        match hash[&RubyValue::FixNum(5)] {
                            RubyValue::FixNum(val) => {
                                assert_eq!(val, 2)
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

    }

    #[test]
    fn test_read_class() {
        let input = b"\x04\x08c\x09Test";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Class(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Class(class) => {
                        assert_eq!(class, "Test");
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_module() {
        let input = b"\x04\x08m\x09Test";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Module(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Module(module) => {
                        assert_eq!(module, "Test");
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_class_or_module() {
        let input = b"\x04\x08M\x09Test";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::ClassOrModule(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::ClassOrModule(class_or_module) => {
                        assert_eq!(class_or_module, "Test");
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_string() {
        let input = b"\x04\x08\"\x09Test";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::String(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::String(string) => {
                        assert_eq!(string.get_string(), b"Test");
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_instance_variables() {
        let input = b"\x04\x08I\"\x09Test\x06:\x06ET";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::String(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::String(string) => {
                        assert_eq!(result.decode_string(string).unwrap(), "Test");
                        assert_eq!(string.get_instance_variables().as_ref().unwrap().len(), 1);
                        let symbol_id = string.get_instance_variables().as_ref().unwrap().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "E");
                        match string.get_instance_variable(*symbol_id).unwrap() {
                            RubyValue::Boolean(boolean) => {
                                assert!(*boolean);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_bignum() {
        let input = b"\x04\x08l+\x09\xb9\xa3\x38\x97\x22\x26\x36\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::BigNum(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::BigNum(bignum) => {
                        assert_eq!(*bignum, 15241578750190521);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08l-\x09\xb9\xa3\x38\x97\x22\x26\x36\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::BigNum(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::BigNum(bignum) => {
                        assert_eq!(*bignum, -15241578750190521);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

    }

    #[test]
    fn test_read_regexp() {
        let input = b"\x04\x08I/\x08iii\x00\x06:\x06EF";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::RegExp(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::RegExp(regexp) => {
                        assert_eq!(regexp.get_pattern(), "iii");
                        assert_eq!(regexp.get_options(), 0);
                        let symbol_id = regexp.get_instance_variables().as_ref().unwrap().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "E");
                        match regexp.get_instance_variable(*symbol_id).unwrap() {
                            RubyValue::Boolean(boolean) => {
                                assert!(!*boolean);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

        let input = b"\x04\x08l-\x09\xb9\xa3\x38\x97\x22\x26\x36\x00";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::BigNum(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::BigNum(bignum) => {
                        assert_eq!(*bignum, -15241578750190521);
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }

    }

    #[test]
    fn test_read_struct() {
        let input = b"\x04\x08S:\x09Test\x06:\x06ai\x06";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Struct(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Struct(ruby_struct) => {
                        assert_eq!(result.get_symbol(ruby_struct.get_name()).unwrap(), "Test");
                        let symbol_id = ruby_struct.get_members().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "a");
                        match ruby_struct.get_member(*symbol_id).unwrap() {
                            RubyValue::FixNum(fixnum) => {
                                assert_eq!(*fixnum, 1);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_object() {
        let input = b"\x04\x08o:\x09Test\x06:\x07@ai\x06";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Object(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::Object(object) => {
                        assert_eq!(result.get_symbol(object.get_class_name()).unwrap(), "Test");
                        let symbol_id = object.get_instance_variables().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "@a");
                        match object.get_instance_variable(*symbol_id).unwrap() {
                            RubyValue::FixNum(fixnum) => {
                                assert_eq!(*fixnum, 1);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_user_class() {
        let input = b"\x04\x08IC:\x09Test\"\x06a\x06:\x06ET";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::UserClass(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::UserClass(user_class) => {
                        assert_eq!(result.get_symbol(user_class.get_name()).unwrap(), "Test");
                        assert_eq!(user_class.decode_wrapped_string(&result).unwrap(), "a");
                        let symbol_id = user_class.get_instance_variables().as_ref().unwrap().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "E");
                        match user_class.get_instance_variable(*symbol_id).unwrap() {
                            RubyValue::Boolean(boolean) => {
                                assert!(*boolean);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_user_defined() {
        let input = b"\x04\x08Iu:\x09Test\x061\x06:\x06EF";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::UserDefined(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::UserDefined(user_defined) => {
                        assert_eq!(result.get_symbol(user_defined.get_class_name()).unwrap(), "Test");
                        assert_eq!(user_defined.get_data(), &vec![b'1']);
                        let symbol_id = user_defined.get_instance_variables().as_ref().unwrap().keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "E");
                        match user_defined.get_instance_variable(*symbol_id).unwrap() {
                            RubyValue::Boolean(boolean) => {
                                assert!(!*boolean);
                            }
                            _ => panic!("Got wrong value type"),
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }

    #[test]
    fn test_read_user_marshal() {
        let input = b"\x04\x08U:\x09Testi\x06";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::UserMarshal(object_id) => {
                match result.get_object(*object_id).unwrap() {
                    RubyObject::UserMarshal(user_marshal) => {
                        assert_eq!(result.get_symbol(user_marshal.get_class_name()).unwrap(), "Test");
                        if let RubyValue::FixNum(fixnum) = user_marshal.get_wrapped_object() {
                            assert_eq!(*fixnum, 1);
                        } else {
                            panic!("Got wrong value type");
                        }
                    }
                    _ => panic!("Got wrong object type"),
                }
            }
            _ => panic!("Got wrong value type"),
        }
    }
}
