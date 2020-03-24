use byteorder::{ByteOrder, LittleEndian};
use wasmer_runtime::{Memory as WASMMemory, WasmPtr};

pub struct Memory<'a> {
    inner: &'a WASMMemory,
}

impl<'a> Memory<'a> {
    pub fn new(inner: &'a WASMMemory) -> Self {
        Self { inner }
    }

    pub fn read_u32(&self, ptr: usize) -> u32 {
        let mut len_buf = [0u8; 4];
        for (i, cell) in self.inner.view()[ptr..ptr + 4].iter().enumerate() {
            len_buf[i] = cell.get();
        }
        LittleEndian::read_u32(&len_buf)
    }

    pub fn get_utf16_string(&self, ptr: usize) -> String {
        if ptr < 4 {
            panic!("Wrong offset, less than 4")
        }

        let len = self.read_u32(ptr - 4) as usize;
        let data_buf = self.get_vec_u8_with_len(ptr, len);

        let mut u16_buffer = vec![0u16; len as usize / 2];
        LittleEndian::read_u16_into(&data_buf, &mut u16_buffer);
        String::from_utf16(&u16_buffer).unwrap()
    }

    pub fn get_utf8_string(&self, ptr: usize) -> String {
        if ptr < 4 {
            panic!("Wrong offset, less than 4")
        }

        let len = self.read_u32(ptr - 4) as usize;
        let data_buf = self.get_vec_u8_with_len(ptr, len);
        String::from_utf8(data_buf).unwrap()
    }

    pub fn get_vec_u8(&self, ptr: usize) -> Vec<u8> {
        if ptr < 4 {
            panic!("Wrong offset, less than 4")
        }

        let len = self.read_u32(ptr - 4) as usize;

        self.get_vec_u8_with_len(ptr, len)
    }

    pub fn set_bytes(&self, offset: usize, data: &[u8]) {
        self.inner.view()[offset..(offset + data.len())]
            .iter()
            .zip(data.iter())
            .for_each(|(cell, v)| cell.set(*v));
    }

    fn get_vec_u8_with_len(&self, ptr: usize, len: usize) -> Vec<u8> {
        let mut data_buf = vec![0u8; len];
        for (i, cell) in self.inner.view()[ptr..(ptr + len)].iter().enumerate() {
            data_buf[i] = cell.get();
        }

        data_buf
    }
}
