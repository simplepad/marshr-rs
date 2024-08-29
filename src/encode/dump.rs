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
    /// length is equal to the number of objects + 1 (0th object is the root), `objects[i]` holds `true` if the object with id `i` has already been written
    objects: Vec<bool>,
}

impl<'a, T: Write> Dumper<'a, T> {
    pub fn new(writer: &'a mut T) -> Self {
        Self {
            writer,
            symbols: Vec::new(),
            objects: Vec::new(),
        }
    }

    fn reset(&mut self, number_of_symbols: usize, number_of_objects: usize) {
        // TODO: use reserve()
        self.symbols = vec![false; number_of_symbols];
        self.objects = vec![false; number_of_objects+1];
    }

    fn write(&mut self, data: &[u8]) -> Result<(), DumpError> {
        if let Err(err) = self.writer.write_all(data) {
            return Err(DumpError::IoError(format!("Could not write data: {}", err)));
        }
        Ok(())
    }

    pub fn dump(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        self.reset(root.get_symbols().len(), root.get_objects().len());

        self.write(&[MARSHAL_MAJOR_VERSION, MARSHAL_MINOR_VERSION])?;

        self.dump_value(root, object)
    }

    fn dump_value(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        match object {
            RubyValue::Nil => self.write(&[b'0']),
            RubyValue::Boolean(boolean) => if *boolean { self.write(&[b'T']) } else { self.write(&[b'F']) },
            RubyValue::FixNum(fixnum) => { self.write(&[b'i'])?; self.write_fixnum(*fixnum) },
            RubyValue::Symbol(symbol_id) => self.write_symbol(root, *symbol_id),
            RubyValue::Array(object_id) => self.write_array(root, *object_id),
            RubyValue::Float(object_id) => self.write_float(root, *object_id),
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
            self.symbols[symbol_id] = true;
            self.write(&[b':'])?;
            self.write_byte_sequence(root.get_symbol(symbol_id).unwrap().as_bytes())?;
        }

        Ok(())
    }

    fn write_object_link(&mut self, object_id: ObjectID) -> Result<(), DumpError> {
        self.write(&[b'@'])?;
        self.write_fixnum(object_id.try_into().unwrap())
    }

    fn write_array(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // array has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // array hasn't been written before, writing an array
            self.write(&[b'['])?;
            self.objects[object_id] = true;
            let array = root.get_object(object_id).unwrap().as_array();
            if let Ok(array_len) = array.len().try_into() {
                self.write_fixnum(array_len)?;
                for value in array {
                    self.dump_value(root, value)?;
                }
            } else {
                return Err(DumpError::EncoderError("Could not write array length, the length doesn't fit into an i32".to_string()));
            }
        }
        Ok(())
    }

    fn write_float(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // float has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // float hasn't been written before, writing an float
            self.write(&[b'f'])?;
            self.objects[object_id] = true;
            let float = root.get_object(object_id).unwrap().as_float();
            if float.is_nan() {
                self.write_byte_sequence(b"nan")?; // float.to_string() returns NaN
            } else {
                self.write_byte_sequence(float.to_string().as_bytes())?;
            }
        }
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::decode::load::Loader;

    use super::*;

    macro_rules! assert_output_is {
        ($i:literal) => {
            let mut output = Vec::<u8>::new();
            let mut dumper = Dumper::new(&mut output);

            let input = $i;
            let mut reader = BufReader::new(&input[..]);
            let mut loader = Loader::new(&mut reader);

            let root = loader.load().unwrap();
            dumper.dump(&root, root.get_root()).unwrap();

            assert_eq!(input[..], output);
        };
    }

    #[test]
    fn test_write_nil() {
        assert_output_is!(b"\x04\x080");
    }

    #[test]
    fn test_write_boolean() {
        assert_output_is!(b"\x04\x08T");
        assert_output_is!(b"\x04\x08F");
    }

    #[test]
    fn test_write_fixnum() {
        assert_output_is!(b"\x04\x08i\x00");
        assert_output_is!(b"\x04\x08i\x7f");
        assert_output_is!(b"\x04\x08i\x80");
        assert_output_is!(b"\x04\x08i\x01\xc8");
        assert_output_is!(b"\x04\x08i\xff\x38");
        assert_output_is!(b"\x04\x08i\x02\xe8\x80");
        assert_output_is!(b"\x04\x08i\xfe\x18\x7f");
        assert_output_is!(b"\x04\x08i\x03\xff\xff\xff");
        assert_output_is!(b"\x04\x08i\xfd\x01\x00\x00");
        assert_output_is!(b"\x04\x08i\x04\xff\xff\xff\x3f");
        assert_output_is!(b"\x04\x08i\xfc\x00\x00\x00\xc0");
        assert_output_is!(b"\x04\x08i\x04\x00\x00\x00\x40");
    }

    #[test]
    fn test_write_symbol() {
        assert_output_is!(b"\x04\x08:\x0ahello");
        assert_output_is!(b"\x04\x08[\x07:\x0ahello;\x00");
    }

    #[test]
    fn test_write_array() {
        assert_output_is!(b"\x04\x08[\x00");
        assert_output_is!(b"\x04\x08[\x07i\x7fi\x7f");
    }

    #[test]
    fn test_write_float() {
        assert_output_is!(b"\x04\x08f\x08inf");
        assert_output_is!(b"\x04\x08f\x09-inf");
        assert_output_is!(b"\x04\x08f\x08nan");
        assert_output_is!(b"\x04\x08f\x092.55");
        assert_output_is!(b"\x04\x08[\x07f\x092.55@\x06");
    }
}

