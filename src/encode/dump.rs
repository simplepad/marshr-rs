use std::{fmt::Display, io::Write};
use crate::values::*;

#[derive(Debug)]
pub enum DumpError {
    IoError(String),
}

impl Display for DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpError::IoError(error) => {
                f.write_str(&format!("IO Error: {}", error))
            }
        }
    }
}

pub struct Dumper<'a, T: Write> {
    writer: &'a mut T,
    symbols: Vec<String>,
    objects: Vec<RubyObject>,
}

impl<'a, T: Write> Dumper<'a, T> {
    pub fn new(writer: &'a mut T) -> Self {
        Self {
            writer,
            symbols: Vec::new(),
            objects: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.symbols.clear();
        self.objects.clear();
    }

    pub fn dump(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        self.reset();

        if let Err(err) = self.writer.write_all(&[MARSHAL_MAJOR_VERSION, MARSHAL_MINOR_VERSION]) {
            return Err(DumpError::IoError(format!("Could not write Marshal version: {}", err)));
        }

        self.dump_value(root, object)
    }

    fn dump_value(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        match object {
            RubyValue::Nil => self.write_nil(),
            RubyValue::Boolean(boolean) => self.write_boolean(*boolean),
            RubyValue::FixNum(fixnum) => self.write_fixnum(*fixnum),
            _ => todo!(),
        }

    }

    fn write_nil(&mut self) -> Result<(), DumpError> {
        if let Err(err) = self.writer.write_all(&[b'0']) {
            return Err(DumpError::IoError(format!("Could not write nil value: {}", err)));
        }
        Ok(())
    }

    fn write_boolean(&mut self, boolean: bool) -> Result<(), DumpError> {
        let mut output = [b'F'];

        if boolean {
            output[0] = b'T';
        }

        if let Err(err) = self.writer.write_all(&output) {
            return Err(DumpError::IoError(format!("Could not write boolean value: {}", err)));
        }
        Ok(())
    }

    fn write_fixnum(&mut self, mut number: i32) -> Result<(), DumpError> {
        let mut output = [0; std::mem::size_of::<i32>() + 2];
        output[0] = b'i';
        let mut bytes_written = 1;

        match number {
            0 => {
                output[1] = 0x00;
                bytes_written += 1;
            },
            1 ..= 122 => {
                output[1] = (number as i8 + 5).to_le_bytes()[0];
                bytes_written += 1;
            },
            -123 ..= -1 => {
                output[1] = (number as i8 - 5).to_le_bytes()[0];
                bytes_written += 1;
            },
            _ => {
                bytes_written += 1; // for fixnum size
                for i in 2..(std::mem::size_of::<i32>() + 2) {
                    output[i] = u8::try_from(number & 0xFF).unwrap();
                    bytes_written += 1;

                    number >>= 8;
                    if number == 0 {
                        output[1] = u8::try_from(i-1).unwrap();
                        break;
                    }
                    if number == -1 {
                        output[1] = (-i8::try_from(i-1).unwrap()) as u8;
                        break;
                    }
                }
            }
        }

        if let Err(err) = self.writer.write_all(&output[..bytes_written]) {
            return Err(DumpError::IoError(format!("Could not write fixnum value: {}", err)));
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::decode::load::Loader;

    use super::*;

    #[test]
    fn test_write_nil() {
        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x080";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);

        let root = loader.load().unwrap();
        dumper.dump(&root, root.get_root()).unwrap();

        assert_eq!(input[..], output);
    }

    #[test]
    fn test_write_boolean() {
        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08T";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);

        let root = loader.load().unwrap();
        dumper.dump(&root, root.get_root()).unwrap();

        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08F";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);

        let root = loader.load().unwrap();
        dumper.dump(&root, root.get_root()).unwrap();

        assert_eq!(input[..], output);
    }

    #[test]
    fn test_write_fixnum() {
        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x00";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x7f";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x80";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x01\xc8";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\xff\x38";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x02\xe8\x80";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\xfe\x18\x7f";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x03\xff\xff\xff";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\xfd\x01\x00\x00";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x04\xff\xff\xff\x3f";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\xfc\x00\x00\x00\xc0";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);

        let mut output = Vec::<u8>::new();
        let mut dumper = Dumper::new(&mut output);

        let input = b"\x04\x08i\x04\x00\x00\x00\x40";
        let mut reader = BufReader::new(&input[..]);
        let mut loader = Loader::new(&mut reader);
        let result = loader.load().unwrap();

        dumper.dump(&result, result.get_root()).unwrap();
        assert_eq!(input[..], output);
    }
}

