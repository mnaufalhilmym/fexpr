use std::io::Write;

use crate::error::Error;

pub struct Buffer {
    buffer: Vec<u8>,
}

impl Buffer {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn write_char(&mut self, ch: char) -> Result<(), Error> {
        let mut ch_buf = [0];
        ch.encode_utf8(&mut ch_buf);
        self.buffer
            .write(&ch_buf)
            .map_err(|err| Error::Buffer(err.to_string()))?;
        Ok(())
    }

    pub fn write_string(&mut self, str: &str) -> Result<(), Error> {
        let str_buf = str.as_bytes();
        self.buffer
            .write(&str_buf)
            .map_err(|err| Error::Buffer(err.to_string()))?;
        Ok(())
    }

    pub fn to_string(self) -> Result<String, Error> {
        String::from_utf8(self.buffer).map_err(|err| Error::Buffer(err.to_string()))
    }
}
