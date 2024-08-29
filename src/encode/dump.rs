use std::{fmt::Display, io::Write};
use crate::values::*;

#[derive(Debug)]
pub enum DumpError {
    IoError(String),
    EncoderError(String),
}

impl Display for DumpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpError::IoError(error) => {
                f.write_str(&format!("IO Error: {}", error))
            }
            DumpError::EncoderError(error) => {
                f.write_str(&format!("Encoder Error: {}", error))
            }
        }
    }
}

pub struct Dumper<'a, T: Write> {
    writer: &'a mut T,
    /// length is equal to the number of symbols, `symbols[i]` holds `true` if the symbol with id `i` has already been written
    symbols: Vec<bool>, 
    objects: Vec<RubyObject>,
}

impl<'a, T: Write> Dumper<'a, T> {
    pub fn new(writer: &'a mut T, number_of_symbols: usize) -> Self {
        Self {
            writer,
            symbols: vec![false; number_of_symbols],
            objects: Vec::new(),
        }
    }

    fn reset(&mut self, number_of_symbols: usize) {
        self.symbols = vec![false; number_of_symbols];
        self.objects.clear();
    }

    fn write(&mut self, data: &[u8]) -> Result<(), DumpError> {
        if let Err(err) = self.writer.write_all(data) {
            return Err(DumpError::IoError(format!("Could not write data: {}", err)));
        }
        Ok(())
    }

    pub fn dump(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        self.reset(root.get_symbols().len());

        self.write(&[MARSHAL_MAJOR_VERSION, MARSHAL_MINOR_VERSION])?;

        self.dump_value(root, object)
    }

    fn dump_value(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        match object {
            RubyValue::Nil => self.write(&[b'0']),
            RubyValue::Boolean(boolean) => if *boolean { self.write(&[b'T']) } else { self.write(&[b'F']) },
            RubyValue::FixNum(fixnum) => { self.write(&[b'i'])?; self.write_fixnum(*fixnum) },
            RubyValue::Symbol(symbol_id) => self.write_symbol(root, *symbol_id),
            _ => todo!(),
        }

    }

    fn write_fixnum(&mut self, mut number: i32) -> Result<(), DumpError> {
        let mut output = [0; std::mem::size_of::<i32>() + 1];
        let mut bytes_written = 0;

        match number {
            0 => {
                output[0] = 0x00;
                bytes_written += 1;
            },
            1 ..= 122 => {
                output[0] = (number as i8 + 5).to_le_bytes()[0];
                bytes_written += 1;
            },
            -123 ..= -1 => {
                output[0] = (number as i8 - 5).to_le_bytes()[0];
                bytes_written += 1;
            },
            _ => {
                bytes_written += 1; // for fixnum size
                for i in 1..(std::mem::size_of::<i32>() + 1) {
                    output[i] = u8::try_from(number & 0xFF).unwrap();
                    bytes_written += 1;

                    number >>= 8;
                    if number == 0 {
                        output[0] = u8::try_from(i).unwrap();
                        break;
                    }
                    if number == -1 {
                        output[0] = (-i8::try_from(i).unwrap()) as u8;
                        break;
                    }
                }
            }
        }

        self.write(&output[..bytes_written])
    }

    fn write_byte_sequence(&mut self, sequence: &[u8]) -> Result<(), DumpError> {
        if let Ok(sequence_len) = i32::try_from(sequence.len()) {
            self.write_fixnum(sequence_len)?;
        } else {
            return Err(DumpError::EncoderError("Could not write byte sequence length, the length doesn't fit into an i32".to_string()));
        }

        self.write(sequence)
    }

    fn write_symbol(&mut self, root: &Root, symbol_id: SymbolID) -> Result<(), DumpError> {
        if self.symbols[symbol_id] {
            // symbol has been written before, writing a symbol link
            self.write(&[b';'])?;
            self.write_fixnum(symbol_id.try_into().unwrap())?;
        } else {
            // symbol hasn't been written before, writing a symbol
            self.write(&[b':'])?;
            self.write_byte_sequence(root.get_symbol(symbol_id).unwrap().as_bytes())?;
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

