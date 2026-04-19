use crate::error::Error;

pub struct Buffer {
    buffer: Vec<char>,
}

impl Buffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn write_char(&mut self, ch: char) -> Result<(), Error> {
        self.buffer.push(ch);
        Ok(())
    }

    pub fn write_string(&mut self, str: &str) -> Result<(), Error> {
        for c in str.chars() {
            self.buffer.push(c);
        }
        Ok(())
    }

    pub fn into_string(self) -> String {
        self.buffer.iter().collect()
    }
}
