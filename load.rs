use std::{collections::HashMap, io::Read, string::ParseError};
use crate::ruby_marshal::values::*;

#[derive(Debug)]
enum LoadError {
    IoError(String),
    ParserError(String),
}

impl From<std::string::FromUtf8Error> for LoadError {
    fn from(_value: std::string::FromUtf8Error) -> Self {
        Self::ParserError("Could not decode bytes into a String".to_string())
    }
}

impl From<std::num::ParseFloatError> for LoadError {
    fn from(_value: std::num::ParseFloatError) -> Self {
        Self::ParserError("Could not parse float from sequence".to_string())
    }
}

struct Loader<T: Read> {
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

    fn read_sequence(&mut self) -> Result<String, LoadError> {
        let sequence_len = self.read_fixnum()?.try_into().unwrap();
        let mut buffer = vec![0; sequence_len];
        if let Err(err) = self.reader.read_exact(&mut buffer) {
            return Err(LoadError::IoError(format!("Failed to read sequence: {}, was expecting {} bytes", err, sequence_len)));
        }
        let sequence = String::from_utf8(buffer)?;
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
            Err(error) => return Err(LoadError::ParserError("Could not parse symbol link (could not convert symbol index to usize)".to_string())),
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
        let float_sequence = self.read_sequence()?;

        let float_val = match float_sequence.as_str() {
            "inf" => f64::INFINITY,
            "-inf" => f64::NEG_INFINITY,
            "nan" => f64::NAN,
            float_val => {
                float_val.parse()?
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
                RubyObject::Empty => return Err(LoadError::ParserError("Could not parse object link (links to a non-existent object)".to_string())),
                RubyObject::Array(_) => RubyValue::Array(object_id),
                RubyObject::Float(_) => RubyValue::Float(object_id),
                RubyObject::Hash(_) => RubyValue::Hash(object_id),
                RubyObject::HashWithDefault(_) => RubyValue::HashWithDefault(object_id),

            };
            Ok(ruby_value)
        } else {
            Err(LoadError::ParserError("Could not parse object link (links to a non-existent object)".to_string()))
        }
    }

    fn read_value_pairs(&mut self) -> Result<HashMap<SymbolID, RubyValue>, LoadError> {
        let num_of_pairs = match usize::try_from(self.read_fixnum()?) {
            Ok(val) => val,
            Err(_) => return Err(LoadError::ParserError("Could not parse number of key:value pairs (could not convert number of pairs to usize)".to_string())),
        };

        let mut pairs = HashMap::with_capacity(num_of_pairs);

        for _ in 0..num_of_pairs {
            let symbol = if let RubyValue::Symbol(symbol_id) = self.read_value()? {
                symbol_id
            } else {
                return Err(LoadError::ParserError("Could not parse key:value pairs, key was not a Symbol".to_string()))
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
                        for i in 0..2 {
                            match array[i] {
                                RubyValue::Symbol(symbol_id) => {
                                    assert_eq!(result.get_symbol(symbol_id).unwrap(), "hello")
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
                        for i in 0..2 {
                            match array[i] {
                                RubyValue::FixNum(fixnum) => {
                                    assert_eq!(fixnum, 122)
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
                        for i in 0..2 {
                            match array[i] {
                                RubyValue::Float(object_id) => {
                                    match result.get_object(object_id).unwrap() {
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
                        let symbol_id = hash.keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "a");
                        match hash[symbol_id] {
                            RubyValue::FixNum(val) => {
                                assert_eq!(val, 1)
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        assert!(hash.get(&(symbol_id+1)).is_none())
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
                        let symbol_id = hash.keys().next().unwrap();
                        assert_eq!(result.get_symbol(*symbol_id).unwrap(), "a");
                        match hash[*symbol_id] {
                            RubyValue::FixNum(val) => {
                                assert_eq!(val, 1)
                            }
                            _ => panic!("Got wrong value type"),
                        }
                        // test default value
                        match hash[symbol_id+1] {
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
}
