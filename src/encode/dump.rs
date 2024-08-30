use std::{fmt::Display, io::Write, num::TryFromIntError};
use crate::values::*;

#[derive(Debug)]
pub enum DumpError {
    IoError(String),
    EncoderError(String),
}

impl From<TryFromIntError> for DumpError {
    fn from(value: TryFromIntError) -> Self {
        DumpError::EncoderError(value.to_string())
    }
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

    fn flush(&mut self) -> Result<(), DumpError> {
        if let Err(err) = self.writer.flush() {
            return Err(DumpError::IoError(format!("Could not flush data: {}", err)));
        }
        Ok(())
    }

    pub fn dump(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        self.reset(root.get_symbols().len(), root.get_objects().len());

        self.write(&[MARSHAL_MAJOR_VERSION, MARSHAL_MINOR_VERSION])?;

        self.dump_value(root, object)?;

        self.flush()?;
        Ok(())
    }

    fn dump_value(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        match object {
            RubyValue::Uninitialized(_) => panic!("Tried to dump uninitialized object"),
            RubyValue::Nil => self.write(&[b'0']),
            RubyValue::Boolean(boolean) => if *boolean { self.write(&[b'T']) } else { self.write(&[b'F']) },
            RubyValue::FixNum(fixnum) => { self.write(&[b'i'])?; self.write_fixnum(*fixnum) },
            RubyValue::Symbol(symbol_id) => self.write_symbol(root, *symbol_id),
            RubyValue::Array(object_id) => self.write_array(root, *object_id),
            RubyValue::Float(object_id) => self.write_float(root, *object_id),
            RubyValue::Hash(object_id) => self.write_hash(root, *object_id),
            RubyValue::HashWithDefault(object_id) => self.write_hash_with_default(root, *object_id),
            RubyValue::Class(object_id) => self.write_class(root, *object_id),
            RubyValue::Module(object_id) => self.write_module(root, *object_id),
            RubyValue::ClassOrModule(object_id) => self.write_class_or_module(root, *object_id),
            RubyValue::String(object_id) => self.write_string(root, *object_id),
            RubyValue::BigNum(object_id) => self.write_bignum(root, *object_id),
            RubyValue::RegExp(object_id) => self.write_regexp(root, *object_id),
            RubyValue::Struct(object_id) => self.write_struct(root, *object_id),
            RubyValue::Object(object_id) => self.write_object(root, *object_id),
            RubyValue::UserClass(object_id) => self.write_user_class(root, *object_id),
            RubyValue::UserMarshal(object_id) => self.write_user_marshal(root, *object_id),
            RubyValue::UserDefined(object_id) => self.write_user_defined(root, *object_id),
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
            self.write_fixnum(array.len().try_into()?)?;
            for value in array {
                self.dump_value(root, value)?;
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

    fn write_value_pairs(&mut self, root: &Root, value_pairs: &ValuePairs) -> Result<(), DumpError> {
        self.write_fixnum(value_pairs.len().try_into()?)?;
        for (key, value) in value_pairs {
            self.dump_value(root, key)?;
            self.dump_value(root, value)?;
        }
        Ok(())
    }

    fn write_value_pairs_with_symbol_keys(&mut self, root: &Root, value_pairs: &ValuePairsSymbolKeys) -> Result<(), DumpError> {
        self.write_fixnum(value_pairs.len().try_into()?)?;
        for (key, value) in value_pairs {
            self.write_symbol(root, *key)?;
            self.dump_value(root, value)?;
        }
        Ok(())
    }

    fn write_hash(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // hash has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // hash hasn't been written before, writing an hash
            self.write(&[b'{'])?;
            self.objects[object_id] = true;
            let hash = root.get_object(object_id).unwrap().as_hash();
            self.write_value_pairs(root, hash)?;
        }
        Ok(())
    }

    fn write_hash_with_default(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // hash has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // hash hasn't been written before, writing an hash
            self.write(&[b'}'])?;
            self.objects[object_id] = true;
            let hash = root.get_object(object_id).unwrap().as_hash_with_default();
            self.write_value_pairs(root, hash.hash())?;
            self.dump_value(root, hash.default())?;
        }
        Ok(())
    }

    fn write_class(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // class has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // class hasn't been written before, writing an class
            self.write(&[b'c'])?;
            self.objects[object_id] = true;
            let class = root.get_object(object_id).unwrap().as_class();
            self.write_byte_sequence(class.as_bytes())?;
        }
        Ok(())
    }

    fn write_module(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // module has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // module hasn't been written before, writing an module
            self.write(&[b'm'])?;
            self.objects[object_id] = true;
            let module = root.get_object(object_id).unwrap().as_module();
            self.write_byte_sequence(module.as_bytes())?;
        }
        Ok(())
    }

    fn write_class_or_module(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // class_or_module has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // class_or_module hasn't been written before, writing an class_or_module
            self.write(&[b'M'])?;
            self.objects[object_id] = true;
            let class_or_module = root.get_object(object_id).unwrap().as_class_or_module();
            self.write_byte_sequence(class_or_module.as_bytes())?;
        }
        Ok(())
    }

    fn write_string(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // string has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // string hasn't been written before, writing an string
            self.objects[object_id] = true;
            let string = root.get_object(object_id).unwrap().as_string();
            let has_instance_variables = string.get_instance_variables().is_some();
            if has_instance_variables {
                self.write(&[b'I'])?;
            }
            self.write(&[b'"'])?;
            self.write_byte_sequence(string.get_string())?;
            if has_instance_variables {
                self.write_value_pairs_with_symbol_keys(root, string.get_instance_variables().as_ref().unwrap())?;
            }
        }
        Ok(())
    }

    fn write_bignum(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // bignum has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // bignum hasn't been written before, writing an bignum
            self.objects[object_id] = true;
            self.write(b"l")?;
            let bignum = root.get_object(object_id).unwrap().as_bignum();
            if bignum.is_positive() {
                self.write(b"+")?;
            } else {
                self.write(b"-")?; // will write 0 as -0, although 0 shouldn't be encoded as bignum
            }
            let bignum = bignum.abs();
            let bignum_bytes = bignum.to_le_bytes();
            let mut first_non_zero_byte = 0;
            while bignum_bytes[first_non_zero_byte] == 0 {
                first_non_zero_byte += 1;
            }
            if first_non_zero_byte % 2 == 1 {
                first_non_zero_byte -= 1;
            }
            self.write_fixnum(((std::mem::size_of::<RubyBignum>() - first_non_zero_byte) / 2).try_into()?)?;
            self.write(&bignum_bytes[first_non_zero_byte..])?;
        }
        Ok(())
    }

    fn write_regexp(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // regexp has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // regexp hasn't been written before, writing an regexp
            self.objects[object_id] = true;
            let regexp = root.get_object(object_id).unwrap().as_regexp();
            let has_instance_variables = regexp.get_instance_variables().is_some();
            if has_instance_variables {
                self.write(&[b'I'])?;
            }
            self.write(&[b'/'])?;
            self.write_byte_sequence(regexp.get_pattern().as_bytes())?;
            self.write(&[regexp.get_options() as u8])?;
            if has_instance_variables {
                self.write_value_pairs_with_symbol_keys(root, regexp.get_instance_variables().as_ref().unwrap())?;
            }
        }
        Ok(())
    }

    fn write_struct(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // struct has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // struct hasn't been written before, writing an struct
            self.objects[object_id] = true;
            let ruby_struct = root.get_object(object_id).unwrap().as_struct();
            self.write(&[b'S'])?;
            self.write_symbol(root, ruby_struct.get_name())?;
            self.write_value_pairs_with_symbol_keys(root, ruby_struct.get_members())?;
        }
        Ok(())
    }

    fn write_object(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // object has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // object hasn't been written before, writing an object
            self.objects[object_id] = true;
            let object = root.get_object(object_id).unwrap().as_object();
            self.write(&[b'o'])?;
            self.write_symbol(root, object.get_class_name())?;
            self.write_value_pairs_with_symbol_keys(root, object.get_instance_variables())?;
        }
        Ok(())
    }

    fn write_user_class(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // user_class has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // user_class hasn't been written before, writing an user_class
            self.objects[object_id] = true;
            let user_class = root.get_object(object_id).unwrap().as_user_class();
            let has_instance_variables = user_class.get_instance_variables().is_some();
            if has_instance_variables {
                self.write(&[b'I'])?;
            }
            self.write(&[b'C'])?;
            self.write_symbol(root, user_class.get_name())?;
            self.dump_value(root, user_class.get_wrapped_object())?;
            if has_instance_variables {
                self.write_value_pairs_with_symbol_keys(root, user_class.get_instance_variables().as_ref().unwrap())?;
            }
        }
        Ok(())
    }

    fn write_user_defined(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // user_defined has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // user_defined hasn't been written before, writing an user_defined
            self.objects[object_id] = true;
            let user_defined = root.get_object(object_id).unwrap().as_user_defined();
            let has_instance_variables = user_defined.get_instance_variables().is_some();
            if has_instance_variables {
                self.write(&[b'I'])?;
            }
            self.write(&[b'u'])?;
            self.write_symbol(root, user_defined.get_class_name())?;
            self.write_byte_sequence(user_defined.get_data())?;
            if has_instance_variables {
                self.write_value_pairs_with_symbol_keys(root, user_defined.get_instance_variables().as_ref().unwrap())?;
            }
        }
        Ok(())
    }

    fn write_user_marshal(&mut self, root: &Root, object_id: ObjectID) -> Result<(), DumpError> {
        if self.objects[object_id] {
            // user_marshal has been written before, writing an object link
            self.write_object_link(object_id)?;
        } else {
            // user_marshal hasn't been written before, writing an user_marshal
            self.objects[object_id] = true;
            let user_marshal = root.get_object(object_id).unwrap().as_user_marshal();
            self.write(&[b'U'])?;
            self.write_symbol(root, user_marshal.get_class_name())?;
            self.dump_value(root, user_marshal.get_wrapped_object())?;
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

    macro_rules! assert_output_is_concat {
        ($i:literal) => {
            let mut output = Vec::<u8>::new();
            let mut dumper = Dumper::new(&mut output);

            let input = $i;
            let mut reader = BufReader::new(&input[..]);
            let mut loader = Loader::new(&mut reader);

            let root = loader.load().unwrap();
            dumper.dump(&root, root.get_root()).unwrap();

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

    #[test]
    fn test_write_hash() {
        assert_output_is!(b"\x04\x08}\x06:\x06ai\x06i\x07");
    }

    #[test]
    fn test_write_class() {
        assert_output_is!(b"\x04\x08c\x09Test");
    }

    #[test]
    fn test_write_module() {
        assert_output_is!(b"\x04\x08m\x09Test");
    }

    #[test]
    fn test_write_class_or_module() {
        assert_output_is!(b"\x04\x08M\x09Test");
    }

    #[test]
    fn test_write_string() {
        assert_output_is!(b"\x04\x08\"\x09Test");
        assert_output_is!(b"\x04\x08I\"\x09Test\x06:\x06ET");
    }

    #[test]
    fn test_write_bignum() {
        assert_output_is!(b"\x04\x08l+\x09\xb9\xa3\x38\x97\x22\x26\x36\x00");
        assert_output_is!(b"\x04\x08l-\x09\xb9\xa3\x38\x97\x22\x26\x36\x00");
    }

    #[test]
    fn test_write_regexp() {
        assert_output_is!(b"\x04\x08I/\x08iii\x00\x06:\x06EF");
    }

    #[test]
    fn test_write_struct() {
        assert_output_is!(b"\x04\x08S:\x09Test\x06:\x06ai\x06");
    }

    #[test]
    fn test_write_object() {
        assert_output_is!(b"\x04\x08o:\x09Test\x06:\x07@ai\x06");
    }

    #[test]
    fn test_write_user_class() {
        assert_output_is!(b"\x04\x08IC:\x09Test\"\x06a\x06:\x06ET");
    }

    #[test]
    fn test_write_user_defined() {
        assert_output_is!(b"\x04\x08Iu:\x09Test\x061\x06:\x06EF");
    }

    #[test]
    fn test_write_user_marshal() {
        assert_output_is!(b"\x04\x08U:\x09Testi\x06");
    }

    #[test]
    fn test_write_concat() {
        assert_output_is_concat!(b"\x04\x08i\x06\x04\x08i\x07");
        assert_output_is_concat!(b"\x04\x08o:\x09Test\x00\x04\x08o:\x09Test\x00");
    }
}

