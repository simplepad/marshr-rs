use std::io::Read;
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

struct Loader {
    symbols_table: Vec<String>
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            symbols_table: Vec::new(),
        }
    }

    pub fn read(&mut self, mut reader: impl Read) -> Result<Root, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        reader.read_exact(&mut buffer)?;

        let value = match buffer[0] {
            b'0' => RubyObject::Nil,
            b'T' => RubyObject::Boolean(true),
            b'F' => RubyObject::Boolean(false),
            b'i' => RubyObject::FixNum(Loader::read_fixnum(reader)?),
            b':' => RubyObject::Symbol(self.read_symbol(reader)?),
            _ => return Err(LoadError::ParserError(format!("Unknown value type: {}", buffer[0]))),
        };

        let root = Root::new(value);

        Ok(root)
    }

    fn read_fixnum(mut reader: impl Read) -> Result<i32, LoadError> {
        let mut buffer: [u8; 1] = [0; 1];
        reader.read_exact(&mut buffer)?;

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
            reader.read_exact(&mut buffer[..int_len.into()])?;

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

    fn read_symbol(&mut self, mut reader: impl Read) -> Result<String, LoadError> {
        let symbol_len = Loader::read_fixnum(&mut reader)?.try_into().unwrap();
        let mut buffer = vec![0; symbol_len];
        reader.read_exact(&mut buffer)?;
        let symbol = String::from_utf8(buffer)?;

        self.symbols_table.push(symbol.clone());
        Ok(symbol)
    }
}


#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::*;

    #[test]
    fn test_read_nil() {
        let mut loader = Loader::new();
        let input = b"0";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::Nil);

        let input = b"a";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_err());
        if ! matches!(result.unwrap_err(), LoadError::ParserError(_)) {
            panic!("Got wrong error type");
        }
    }

    #[test]
    fn test_read_boolean() {
        let mut loader = Loader::new();
        let input = b"T";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::Boolean(true));

        let input = b"F";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::Boolean(false));

        let input = b"a";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_err());
        if ! matches!(result.unwrap_err(), LoadError::ParserError(_)) {
            panic!("Got wrong error type");
        }
    }

    #[test]
    fn test_read_fixnum() {
        let mut loader = Loader::new();

        let input = b"i\x00";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(0));

        let input = b"i\x7f";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(122));

        let input = b"i\x80";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(-123));

        let input = b"i\x01\xc8";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(200));

        let input = b"i\xff\x38";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(-200));

        let input = b"i\x02\xe8\x80";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(33000));

        let input = b"i\xfe\x18\x7f";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(-33000));

        let input = b"i\x03\xff\xff\xff";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(16777215));

        let input = b"i\xfd\x01\x00\x00";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(-16777215));

        let input = b"i\x04\xff\xff\xff\x3f";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(1073741823));

        let input = b"i\xfc\x00\x00\x00\xc0";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(-1073741824));

        let input = b"i\x04\x00\x00\x00\x40";
        let reader = BufReader::new(&input[..]);
        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::FixNum(1073741824));
    }

    #[test]
    fn test_read_symbol() {
        let mut loader = Loader::new();
        let input = b":\x0ahello";
        let reader = BufReader::new(&input[..]);

        let result = loader.read(reader);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_value(), RubyObject::Symbol("hello".to_string()));
        assert!(loader.symbols_table.contains(&"hello".to_string()))
    }

}
