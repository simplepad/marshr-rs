use std::{io::Read, string::ParseError};
use crate::ruby_marshal::values::*;

#[derive(Debug)]
enum LoadError {
    IoError,
    ParserError(String),
}

impl From<std::io::Error> for LoadError {
    fn from(_value: std::io::Error) -> Self {
        Self::IoError
    }
}

impl From<std::string::FromUtf8Error> for LoadError {
    fn from(_value: std::string::FromUtf8Error) -> Self {
        Self::ParserError("Could not decode bytes into a String".to_string())
    }
}

struct Loader<T: Read> {
    reader: T,
    arena: Vec<RubyValue>,
    symbols: Vec<String>,
    objects: Vec<RubyObject>,
}

impl<T: Read> Loader<T> {
    pub fn new(reader: T) -> Self {
        let objects = vec![RubyObject::Empty]; // object index starts with 1
        Loader {
            reader,
            arena: Vec::new(),
            symbols: Vec::new(),
            objects,
        }
    }

    pub fn load(mut self) -> Result<Root, LoadError> {
        let mut buffer: [u8; 2] = [0; 2];
        self.reader.read_exact(&mut buffer)?;

        if buffer[0] > 4 || buffer[1] > 8 {
            return Err(LoadError::ParserError("Unsupported Marshal version".to_string()));
        }

        let value = self.read_object()?;

        Ok(Root::new(value, self.symbols, self.objects))
    }

    fn read_object(&mut self) -> Result<RubyValue, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        self.reader.read_exact(&mut buffer)?;

        let value = match buffer[0] {
            b'0' => RubyValue::Nil,
            b'T' => RubyValue::Boolean(true),
            b'F' => RubyValue::Boolean(false),
            b'i' => RubyValue::FixNum(self.read_fixnum()?),
            b':' => RubyValue::Symbol(self.read_symbol()?),
            b';' => RubyValue::Symbol(self.read_symbol_link()?),
            b'[' => RubyValue::Array(self.read_array()?),
            _ => return Err(LoadError::ParserError(format!("Unknown value type: {}", buffer[0]))),
        };

        Ok(value)
    }

    fn read_fixnum(&mut self) -> Result<i32, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        self.reader.read_exact(&mut buffer)?;

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
            self.reader.read_exact(&mut buffer[..int_len.into()])?;

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

    fn read_symbol(&mut self) -> Result<SymbolID, LoadError> {
        let symbol_len = self.read_fixnum()?.try_into().unwrap();
        let mut buffer = vec![0; symbol_len];
        self.reader.read_exact(&mut buffer)?;
        let symbol = String::from_utf8(buffer)?;

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
            Err(error) => return Err(LoadError::ParserError("Could not parse array length (could not convert array length to usize)".to_string())),
        };

        let mut array = Vec::with_capacity(array_len);

        for _ in 0..array_len {
            array.push(self.read_object()?);
        }

        self.objects.push(RubyObject::Array(array));
        Ok(self.objects.len()-1)
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
                assert_eq!(*object_id, 1);
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
        assert_eq!(result.get_objects().len(), 2); // object ids start with 1

        let input = b"\x04\x08[\x07i\x7fi\x7f";
        let reader = BufReader::new(&input[..]);
        let loader = Loader::new(reader);
        let result = loader.load().unwrap();

        match result.get_root() {
            RubyValue::Array(object_id) => {
                assert_eq!(*object_id, 1);
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
        assert_eq!(result.get_objects().len(), 2); // object ids start with 1
    }


}
