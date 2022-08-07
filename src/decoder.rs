const JUMP_XOR: u8 = 0x09;

#[derive(Clone)]
pub struct XorDecoder<'a> {
    bytes: &'a [u8],
    key: u8,

    index: usize,
}

impl XorDecoder<'_> {
    pub fn new(bytes: &[u8], key: u8) -> XorDecoder<'_> {
        XorDecoder {
            bytes,
            key,
            index: 0,
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes([self.next()?, self.next()?]))
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes([
            self.next()?,
            self.next()?,
            self.next()?,
            self.next()?,
        ]))
    }

    pub fn peek(&self) -> Option<u8> {
        Some(self.bytes.get(self.index)? ^ self.key)
    }
}

impl Iterator for XorDecoder<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let byte = self.bytes.get(self.index)? ^ self.key;
        self.key = self.key.wrapping_add(JUMP_XOR);
        self.index += 1;
        Some(byte)
    }
}
