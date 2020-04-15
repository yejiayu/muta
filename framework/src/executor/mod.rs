mod factory;
#[cfg(test)]
mod tests;

pub use factory::ServiceExecutorFactory;

use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;

use cita_trie::DB as TrieDB;
use derive_more::{Display, From};

use bytes::BytesMut;
use protocol::traits::{
    Executor, ExecutorParams, ExecutorResp, Service, ServiceMapping, ServiceResponse, ServiceState,
    Storage,
};
use protocol::types::{
    Address, Bloom, BloomInput, Hash, MerkleRoot, Receipt, ReceiptResponse, ServiceContext,
    ServiceContextParams, ServiceParam, SignedTransaction, TransactionRequest,
};
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

use crate::binding::sdk::{DefaultChainQuerier, DefaultSDKFactory};
use crate::binding::state::{GeneralServiceState, MPTTrie};

#[derive(Clone)]
enum HookType {
    Before,
    After,
}

#[derive(Clone)]
enum ExecType {
    Read,
    Write,
}

pub struct ServiceExecutor<S: Storage, DB: TrieDB> {
    querier:    Rc<DefaultChainQuerier<S>>,
    states:     Rc<HashMap<String, Rc<RefCell<GeneralServiceState<DB>>>>>,
    root_state: GeneralServiceState<DB>,

    services: HashMap<String, RefCell<Box<dyn Service>>>,
}

impl<S: 'static + Storage, DB: 'static + TrieDB> ServiceExecutor<S, DB> {
    pub fn create_genesis<Mapping: ServiceMapping>(
        services: Vec<ServiceParam>,
        trie_db: Arc<DB>,
        storage: Arc<S>,
        mapping: Arc<Mapping>,
    ) -> ProtocolResult<MerkleRoot> {
        let querier = Rc::new(DefaultChainQuerier::new(Arc::clone(&storage)));

        let mut states = HashMap::new();
        for name in mapping.list_service_name().into_iter() {
            let trie = MPTTrie::new(Arc::clone(&trie_db));

            states.insert(name, Rc::new(RefCell::new(GeneralServiceState::new(trie))));
        }
        let states = Rc::new(states);
        let sdk_factory = Self::create_sdk_factory(Rc::clone(&states), Rc::clone(&querier));

        for params in services.into_iter() {
            let state = states
                .get(&params.name)
                .ok_or(ExecutorError::NotFoundService {
                    service: params.name.to_owned(),
                })?;

            let mut service = mapping.get_service(&params.name, &sdk_factory)?;
            panic::catch_unwind(AssertUnwindSafe(|| {
                service.genesis_(params.payload.clone())
            }))
            .map_err(|e| ProtocolError::from(ExecutorError::InitService(format!("{:?}", e))))?;

            state.borrow_mut().stash()?;
        }

        let trie = MPTTrie::new(Arc::clone(&trie_db));
        let mut root_state = GeneralServiceState::new(trie);
        for (name, state) in states.iter() {
            let root = state.borrow_mut().commit()?;
            root_state.insert(name.to_owned(), root)?;
        }
        root_state.stash()?;
        root_state.commit()
    }

    pub fn with_root<Mapping: ServiceMapping>(
        root: MerkleRoot,
        trie_db: Arc<DB>,
        storage: Arc<S>,
        service_mapping: Arc<Mapping>,
    ) -> ProtocolResult<Self> {
        let querier = Rc::new(DefaultChainQuerier::new(storage));
        let trie = MPTTrie::from(root, Arc::clone(&trie_db))?;
        let root_state = GeneralServiceState::new(trie);

        let list_service_name = service_mapping.list_service_name();

        let mut states = HashMap::new();
        for name in list_service_name.iter() {
            let trie = match root_state.get(name)? {
                Some(service_root) => MPTTrie::from(service_root, Arc::clone(&trie_db))?,
                None => MPTTrie::new(Arc::clone(&trie_db)),
            };

            let service_state = GeneralServiceState::new(trie);
            states.insert(name.to_owned(), Rc::new(RefCell::new(service_state)));
        }
        let states = Rc::new(states);

        let sdk_factory = Self::create_sdk_factory(Rc::clone(&states), Rc::clone(&querier));

        let mut services: HashMap<String, RefCell<Box<dyn Service>>> = HashMap::new();
        for name in list_service_name.iter() {
            let service = service_mapping.get_service(name, &sdk_factory)?;
            services.insert(name.clone(), RefCell::new(service));
        }

        Ok(Self {
            querier,
            states,
            root_state,
            services,
        })
    }

    fn create_sdk_factory(
        states: Rc<HashMap<String, Rc<RefCell<GeneralServiceState<DB>>>>>,
        querier: Rc<DefaultChainQuerier<S>>,
    ) -> DefaultSDKFactory<GeneralServiceState<DB>, DefaultChainQuerier<S>> {
        DefaultSDKFactory::new(states, querier)
    }

    fn commit(&mut self) -> ProtocolResult<MerkleRoot> {
        for (name, state) in self.states.iter() {
            let root = state.borrow_mut().commit()?;
            self.root_state.insert(name.to_owned(), root)?;
        }
        self.root_state.stash()?;
        self.root_state.commit()
    }

    fn stash(&mut self) -> ProtocolResult<()> {
        for state in self.states.values() {
            state.borrow_mut().stash()?;
        }

        Ok(())
    }

    fn revert_cache(&mut self) -> ProtocolResult<()> {
        for state in self.states.values() {
            state.borrow_mut().revert_cache()?;
        }

        Ok(())
    }

    fn hook(&mut self, hook: HookType, exec_params: &ExecutorParams) -> ProtocolResult<()> {
        for name in self.list_service_name().iter() {
            let service = self.get_service(name)?;

            let hook_ret = match hook {
                HookType::Before => panic::catch_unwind(AssertUnwindSafe(|| {
                    service.borrow_mut().hook_before_(exec_params)
                })),
                HookType::After => panic::catch_unwind(AssertUnwindSafe(|| {
                    service.borrow_mut().hook_after_(exec_params)
                })),
            };

            if hook_ret.is_err() {
                self.revert_cache()?;
            } else {
                self.stash()?;
            }
        }
        Ok(())
    }

    fn get_service(&self, name: &str) -> ProtocolResult<&RefCell<Box<dyn Service>>> {
        self.services.get(name).ok_or(
            ExecutorError::NotFoundService {
                service: name.to_owned(),
            }
            .into(),
        )
    }

    fn list_service_name(&self) -> Vec<String> {
        self.services.keys().map(Clone::clone).collect()
    }

    fn get_context(
        &self,
        tx_hash: Option<Hash>,
        nonce: Option<Hash>,
        caller: &Address,
        cycles_price: u64,
        cycles_limit: u64,
        params: &ExecutorParams,
        request: &TransactionRequest,
    ) -> ProtocolResult<ServiceContext> {
        let ctx_params = ServiceContextParams {
            tx_hash,
            nonce,
            cycles_limit,
            cycles_price,
            cycles_used: Rc::new(RefCell::new(0)),
            caller: caller.clone(),
            height: params.height,
            timestamp: params.timestamp,
            service_name: request.service_name.to_owned(),
            service_method: request.method.to_owned(),
            service_payload: request.payload.to_owned(),
            extra: None,
            events: Rc::new(RefCell::new(vec![])),
        };

        Ok(ServiceContext::new(ctx_params))
    }

    fn catch_call(
        &mut self,
        context: ServiceContext,
        exec_type: ExecType,
    ) -> ProtocolResult<ServiceResponse<String>> {
        let result = match exec_type {
            ExecType::Read => panic::catch_unwind(AssertUnwindSafe(|| {
                self.call(context.clone(), exec_type.clone())
            })),
            ExecType::Write => panic::catch_unwind(AssertUnwindSafe(|| {
                self.call_with_tx_hooks(context.clone(), exec_type.clone())
            })),
        };
        match result {
            Ok(r) => {
                self.stash()?;
                Ok(r)
            }
            Err(e) => {
                self.revert_cache()?;
                log::error!("inner chain error occurred when calling service: {:?}", e);
                Err(ExecutorError::CallService(format!("{:?}", e)).into())
            }
        }
    }

    fn call_with_tx_hooks(
        &self,
        context: ServiceContext,
        exec_type: ExecType,
    ) -> ServiceResponse<String> {
        let list_service_name = self.list_service_name();

        // TODO: If tx_hook_before_ failed, we should not exec the tx.
        // Need a mechanism for this.
        for name in list_service_name.iter() {
            let service = self
                .get_service(name)
                .unwrap_or_else(|e| panic!("get target service failed: {}", e));
            service.borrow_mut().tx_hook_before_(context.clone());
        }
        let original_res = self.call(context.clone(), exec_type);
        // TODO: If the tx fails, status tx_hook_after_ changes will also be reverted.
        // It may not be what the developer want.
        // Need a new mechanism for this.
        for name in list_service_name.iter() {
            let service = self
                .get_service(name)
                .unwrap_or_else(|e| panic!("get target service failed: {}", e));
            service.borrow_mut().tx_hook_after_(context.clone());
        }
        original_res
    }

    fn call(&self, context: ServiceContext, exec_type: ExecType) -> ServiceResponse<String> {
        let service = self
            .get_service(context.get_service_name())
            .unwrap_or_else(|e| panic!("get target service failed: {}", e));

        match exec_type {
            ExecType::Read => service.borrow().read_(context),
            ExecType::Write => service.borrow_mut().write_(context),
        }
    }

    fn logs_bloom(&self, receipts: &[Receipt]) -> Bloom {
        let mut bloom = Bloom::default();
        for receipt in receipts {
            for event in receipt.events.iter() {
                let bytes =
                    BytesMut::from((event.service.clone() + &event.data).as_bytes()).freeze();
                let hash = Hash::digest(bytes).as_bytes();

                let input = BloomInput::Raw(hash.as_ref());
                bloom.accrue(input)
            }
        }

        bloom
    }
}

impl<S: 'static + Storage, DB: 'static + TrieDB> Executor for ServiceExecutor<S, DB> {
    fn exec(
        &mut self,
        params: &ExecutorParams,
        txs: &[SignedTransaction],
    ) -> ProtocolResult<ExecutorResp> {
        self.hook(HookType::Before, params)?;

        let mut receipts = txs
            .iter()
            .map(|stx| {
                let caller = Address::from_pubkey_bytes(stx.pubkey.clone())?;
                let context = self.get_context(
                    Some(stx.tx_hash.clone()),
                    Some(stx.raw.nonce.clone()),
                    &caller,
                    stx.raw.cycles_price,
                    stx.raw.cycles_limit,
                    params,
                    &stx.raw.request,
                )?;

                let exec_resp = self.catch_call(context.clone(), ExecType::Write)?;

                Ok(Receipt {
                    state_root:  MerkleRoot::from_empty(),
                    height:      context.get_current_height(),
                    tx_hash:     stx.tx_hash.clone(),
                    cycles_used: context.get_cycles_used(),
                    events:      context.get_events(),
                    response:    ReceiptResponse {
                        service_name: context.get_service_name().to_owned(),
                        method:       context.get_service_method().to_owned(),
                        response:     exec_resp,
                    },
                })
            })
            .collect::<Result<Vec<Receipt>, ProtocolError>>()?;

        self.hook(HookType::After, params)?;

        let state_root = self.commit()?;
        let mut all_cycles_used = 0;

        for receipt in receipts.iter_mut() {
            receipt.state_root = state_root.clone();
            all_cycles_used += receipt.cycles_used;
        }
        let logs_bloom = self.logs_bloom(&receipts);

        Ok(ExecutorResp {
            receipts,
            all_cycles_used,
            state_root,
            logs_bloom,
        })
    }

    fn read(
        &self,
        params: &ExecutorParams,
        caller: &Address,
        cycles_price: u64,
        request: &TransactionRequest,
    ) -> ProtocolResult<ServiceResponse<String>> {
        let context = self.get_context(
            None,
            None,
            caller,
            cycles_price,
            std::u64::MAX,
            params,
            request,
        )?;
        panic::catch_unwind(AssertUnwindSafe(|| self.call(context, ExecType::Read)))
            .map_err(|e| ProtocolError::from(ExecutorError::QueryService(format!("{:?}", e))))
    }
}

#[derive(Debug, Display, From)]
pub enum ExecutorError {
    #[display(fmt = "service {:?} was not found", service)]
    NotFoundService { service: String },
    #[display(fmt = "service {:?} method {:?} was not found", service, method)]
    NotFoundMethod { service: String, method: String },
    #[display(fmt = "Parsing payload to json failed {:?}", _0)]
    JsonParse(serde_json::Error),

    #[display(fmt = "Init service genesis failed: {:?}", _0)]
    InitService(String),
    #[display(fmt = "Query service failed: {:?}", _0)]
    QueryService(String),
    #[display(fmt = "Call service failed: {:?}", _0)]
    CallService(String),
}

impl std::error::Error for ExecutorError {}

impl From<ExecutorError> for ProtocolError {
    fn from(err: ExecutorError) -> ProtocolError {
        ProtocolError::new(ProtocolErrorKind::Executor, Box::new(err))
    }
}
