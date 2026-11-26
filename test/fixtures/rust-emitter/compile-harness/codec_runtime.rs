// A minimal runtime matching the port contract the generated LE-binary codecs
// declare in `codec_port`. It is only compiled, never published.

pub struct CodecError(pub String);

impl CodecError {
    pub fn new(message: String) -> Self {
        Self(message)
    }
}

pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i32_le(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_f32_le(&mut self, value: f32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_bool(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    pub fn write_string(&mut self, value: &str) {
        self.write_u32_le(value.len() as u32);
        self.bytes.extend_from_slice(value.as_bytes());
    }

    pub fn write_option<T, F>(&mut self, value: &Option<T>, write: F)
    where
        F: FnOnce(&mut Self, &T),
    {
        match value {
            Some(inner) => {
                self.write_bool(true);
                write(self, inner);
            }
            None => self.write_bool(false),
        }
    }

    pub fn write_list<T, F>(&mut self, value: &[T], mut write: F)
    where
        F: FnMut(&mut Self, &T),
    {
        self.write_u32_le(value.len() as u32);
        for item in value {
            write(self, item);
        }
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        if self.bytes.len() < N {
            return Err(CodecError::new("unexpected end of input".to_string()));
        }
        let (head, tail) = self.bytes.split_at(N);
        self.bytes = tail;
        let mut out = [0; N];
        out.copy_from_slice(head);
        Ok(out)
    }

    pub fn read_u32_le(&mut self) -> Result<u32, CodecError> {
        self.take::<4>().map(u32::from_le_bytes)
    }

    pub fn read_i32_le(&mut self) -> Result<i32, CodecError> {
        self.take::<4>().map(i32::from_le_bytes)
    }

    pub fn read_f32_le(&mut self) -> Result<f32, CodecError> {
        self.take::<4>().map(f32::from_le_bytes)
    }

    pub fn read_bool(&mut self) -> Result<bool, CodecError> {
        self.take::<1>().map(|[byte]| byte != 0)
    }

    pub fn read_string(&mut self) -> Result<String, CodecError> {
        let length = self.read_u32_le()? as usize;
        if self.bytes.len() < length {
            return Err(CodecError::new("unexpected end of input".to_string()));
        }
        let (head, tail) = self.bytes.split_at(length);
        self.bytes = tail;
        String::from_utf8(head.to_vec()).map_err(|error| CodecError::new(error.to_string()))
    }

    pub fn read_option<T, F>(&mut self, read: F) -> Result<Option<T>, CodecError>
    where
        F: FnOnce(&mut Self) -> Result<T, CodecError>,
    {
        if self.read_bool()? {
            read(self).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn read_list<T, F>(&mut self, mut read: F) -> Result<Vec<T>, CodecError>
    where
        F: FnMut(&mut Self) -> Result<T, CodecError>,
    {
        let length = self.read_u32_le()?;
        (0..length).map(|_| read(self)).collect()
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len()
    }
}
