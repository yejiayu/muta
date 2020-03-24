mod binding_proxy;
mod imports;
mod memory;

use std::cell::RefCell;
use std::rc::Rc;

use derive_more::{Display, From};
use wasmer_runtime::{instantiate, Func};

use protocol::traits::{Service, ServiceSDK};
use protocol::types::ServiceContext;
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

use crate::binding_proxy::BindingProxy;
use crate::memory::Memory;

pub struct WasmRuntime<'a, SDK: ServiceSDK> {
    sdk:          Rc<RefCell<SDK>>,
    service_code: &'a [u8],
}

impl<'a, SDK: 'static + ServiceSDK> WasmRuntime<'a, SDK> {
    pub fn new(sdk: SDK, service_code: &'a [u8]) -> Self {
        Self {
            sdk: Rc::new(RefCell::new(sdk)),
            service_code,
        }
    }

    fn run(&self, ctx: ServiceContext, readonly: bool) -> ProtocolResult<String> {
        let mut proxy = BindingProxy::new(readonly, Rc::clone(&self.sdk));
        let import_object = imports::build_imports::<SDK>(proxy.clone());

        let mut instance =
            instantiate(self.service_code, &import_object).map_err(WasmRuntimeError::Wasm)?;

        let ctx_bytes = serde_json::to_vec(&ctx).map_err(WasmRuntimeError::JsonParse)?;
        let ctx_bytes_len = ctx_bytes.len();
        proxy.write_temp_buffer(instance.context_mut(), ctx_bytes);

        let call_fn: Func<u32, u32> = if readonly {
            instance.func("read").unwrap()
        } else {
            instance.func("write").unwrap()
        };
        let result_ptr = call_fn.call(ctx_bytes_len as u32).unwrap();

        let memory = Memory::new(instance.context().memory(0));
        let result_jsonstr = memory.get_utf8_string(result_ptr as usize);
        Ok(result_jsonstr)
    }
}

impl<'a, SDK: 'static + ServiceSDK> Service for WasmRuntime<'a, SDK> {
    fn write_(&mut self, ctx: ServiceContext) -> ProtocolResult<String> {
        self.run(ctx, false)
    }

    fn read_(&self, ctx: ServiceContext) -> ProtocolResult<String> {
        self.run(ctx, true)
    }
}

#[derive(Debug, Display, From)]
pub enum WasmRuntimeError {
    #[display(fmt = "service {:?} method {:?} was not found", service, method)]
    NotFoundMethod { service: String, method: String },

    #[display(fmt = "Parsing payload to json failed {:?}", _0)]
    JsonParse(serde_json::Error),

    #[display(fmt = "wasm {:?}", _0)]
    Wasm(wasmer_runtime::error::Error),
}
impl std::error::Error for WasmRuntimeError {}

impl From<WasmRuntimeError> for ProtocolError {
    fn from(err: WasmRuntimeError) -> ProtocolError {
        ProtocolError::new(ProtocolErrorKind::Executor, Box::new(err))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;

    use async_trait::async_trait;
    use bytes::Bytes;
    use cita_trie::MemoryDB;

    use framework::binding::sdk::{DefalutServiceSDK, DefaultChainQuerier};
    use framework::binding::state::{GeneralServiceState, MPTTrie};
    use protocol::traits::{NoopDispatcher, Storage};
    use protocol::types::{
        Address, Block, Hash, Proof, Receipt, ServiceContext, ServiceContextParams,
        SignedTransaction,
    };
    use protocol::ProtocolResult;

    use super::*;

    #[test]
    fn run_test() {
        let chain_db = DefaultChainQuerier::new(Arc::new(MockStorage {}));
        let trie = MPTTrie::new(Arc::new(MemoryDB::new(false)));
        let state = GeneralServiceState::new(trie);

        let sdk = DefalutServiceSDK::new(
            Rc::new(RefCell::new(state)),
            Rc::new(chain_db),
            NoopDispatcher {},
        );

        let wasm_bytes = include_bytes!("../as-binding/build_examples/storage_service.wasm");
        let mut runtime = WasmRuntime::new(sdk, wasm_bytes);

        let caller = Address::from_hex("0x755cdba6ae4f479f7164792b318b2a06c759833b").unwrap();
        let context = mock_context(
            1024 * 1024 * 1024,
            caller.clone(),
            "set_storage".to_owned(),
            "helloword".to_owned(),
        );

        let res = runtime.write_(context).unwrap();
        println!("res {:?}", res);

        let context = mock_context(
            1024 * 1024 * 1024,
            caller.clone(),
            "get_storage".to_owned(),
            "".to_owned(),
        );
        let res = runtime.read_(context).unwrap();
        println!("res {:?}", res);
    }

    fn mock_context(
        cycles_limit: u64,
        caller: Address,
        method: String,
        payload: String,
    ) -> ServiceContext {
        let params = ServiceContextParams {
            tx_hash: None,
            nonce: None,
            cycles_limit,
            cycles_price: 1,
            cycles_used: Rc::new(RefCell::new(0)),
            caller,
            height: 1,
            timestamp: 0,
            service_name: "service_name".to_owned(),
            service_method: method,
            service_payload: payload,
            extra: None,
            events: Rc::new(RefCell::new(vec![])),
        };

        ServiceContext::new(params)
    }
    struct MockStorage;

    #[async_trait]
    impl Storage for MockStorage {
        async fn insert_transactions(&self, _: Vec<SignedTransaction>) -> ProtocolResult<()> {
            unimplemented!()
        }

        async fn insert_block(&self, _: Block) -> ProtocolResult<()> {
            unimplemented!()
        }

        async fn insert_receipts(&self, _: Vec<Receipt>) -> ProtocolResult<()> {
            unimplemented!()
        }

        async fn update_latest_proof(&self, _: Proof) -> ProtocolResult<()> {
            unimplemented!()
        }

        async fn get_transaction_by_hash(&self, _: Hash) -> ProtocolResult<SignedTransaction> {
            unimplemented!()
        }

        async fn get_transactions(&self, _: Vec<Hash>) -> ProtocolResult<Vec<SignedTransaction>> {
            unimplemented!()
        }

        async fn get_latest_block(&self) -> ProtocolResult<Block> {
            unimplemented!()
        }

        async fn get_block_by_height(&self, _: u64) -> ProtocolResult<Block> {
            unimplemented!()
        }

        async fn get_block_by_hash(&self, _: Hash) -> ProtocolResult<Block> {
            unimplemented!()
        }

        async fn get_receipt(&self, _: Hash) -> ProtocolResult<Receipt> {
            unimplemented!()
        }

        async fn get_receipts(&self, _: Vec<Hash>) -> ProtocolResult<Vec<Receipt>> {
            unimplemented!()
        }

        async fn get_latest_proof(&self) -> ProtocolResult<Proof> {
            unimplemented!()
        }

        async fn update_overlord_wal(&self, _info: Bytes) -> ProtocolResult<()> {
            unimplemented!()
        }

        async fn load_overlord_wal(&self) -> ProtocolResult<Bytes> {
            unimplemented!()
        }
    }
}
