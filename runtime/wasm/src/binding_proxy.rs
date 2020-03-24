use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use bytes::Bytes;
use wasmer_runtime::Ctx;

use protocol::traits::{ServiceSDK, StoreMap};

use crate::memory::Memory;

pub struct BindingProxy<SDK: ServiceSDK> {
    temp_buffer:     Rc<RefCell<Vec<u8>>>,
    sdk:             Rc<RefCell<SDK>>,
    readonly:        bool,
    cache_alloc_map: Rc<RefCell<HashMap<String, Box<dyn StoreMap<Bytes, Bytes>>>>>,
}

impl<SDK: ServiceSDK> BindingProxy<SDK> {
    pub fn new(readonly: bool, sdk: Rc<RefCell<SDK>>) -> Self {
        Self {
            temp_buffer: Rc::new(RefCell::new(vec![])),
            readonly,
            sdk,

            cache_alloc_map: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn abort(
        &mut self,
        _: &mut Ctx,
        message_ptr: u32,
        filename_ptr: u32,
        lineNumber: u32,
        columnNumber: u32,
    ) {
        println!(
            "message_ptr {:?} filename_ptr{:?} {:?} {:?}",
            message_ptr, filename_ptr, lineNumber, columnNumber
        );
    }

    pub fn console_log(&mut self, ctx: &mut Ctx, message_ptr: u32) {
        let memory = Memory::new(ctx.memory(0));
        let message = memory.get_utf8_string(message_ptr as usize);

        println!("[wasm-runtime]: {:?}", message)
    }

    pub fn alloc_or_recover_map(&mut self, ctx: &mut Ctx, name_ptr: u32) {
        let memory = Memory::new(ctx.memory(0));
        let name = memory.get_utf8_string(name_ptr as usize);

        if !self.cache_alloc_map.borrow().contains_key(&name) {
            let alloc_map = self
                .sdk
                .borrow_mut()
                .alloc_or_recover_map::<Bytes, Bytes>(&name)
                .unwrap();

            self.cache_alloc_map.borrow_mut().insert(name, alloc_map);
        }
    }

    pub fn get_from_map(&mut self, ctx: &mut Ctx, name_ptr: u32, key_ptr: u32) -> u32 {
        let memory = Memory::new(ctx.memory(0));

        let name = memory.get_utf8_string(name_ptr as usize);
        let key = memory.get_vec_u8(key_ptr as usize);

        let value = match self.cache_alloc_map.borrow().get(&name) {
            Some(map) => map.get(&Bytes::from(key)).unwrap(),
            None => panic!("nerver not null"),
        };

        self.temp_buffer.replace(value.to_vec());
        return self.temp_buffer.borrow().len() as u32;
    }

    pub fn set_to_map(&mut self, ctx: &mut Ctx, name_ptr: u32, key_ptr: u32, value_ptr: u32) {
        if self.readonly {
            panic!("The read method does not allow data to be written")
        }

        let memory = Memory::new(ctx.memory(0));

        let name = memory.get_utf8_string(name_ptr as usize);
        let key = memory.get_vec_u8(key_ptr as usize);
        let value = memory.get_vec_u8(value_ptr as usize);

        match self.cache_alloc_map.borrow_mut().get_mut(&name) {
            Some(map) => map.insert(Bytes::from(key), Bytes::from(value)).unwrap(),
            None => panic!("nerver not null"),
        };
    }

    pub fn read_temp_buffer(&mut self, ctx: &mut Ctx, target_ptr: u32) {
        let memory = Memory::new(ctx.memory(0));

        memory.set_bytes(target_ptr as usize, &self.temp_buffer.borrow());
        self.temp_buffer.borrow_mut().clear();
    }

    pub fn write_temp_buffer(&mut self, ctx: &mut Ctx, data: Vec<u8>) {
        self.temp_buffer.replace(data);
    }
}

impl<SDK: ServiceSDK> Clone for BindingProxy<SDK> {
    fn clone(&self) -> Self {
        Self {
            temp_buffer: Rc::clone(&self.temp_buffer),
            readonly:    self.readonly,
            sdk:         Rc::clone(&self.sdk),

            cache_alloc_map: Rc::clone(&self.cache_alloc_map),
        }
    }
}
