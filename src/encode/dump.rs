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

        self.dump_object(root, object)
    }

    fn dump_object(&mut self, root: &Root, object: &RubyValue) -> Result<(), DumpError> {
        match object {
            RubyValue::Nil => self.write_nil(),
            RubyValue::Boolean(boolean) => self.write_boolean(*boolean),
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
}

