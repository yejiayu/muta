use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use derive_more::{Display, From};
use rocksdb::{Options, WriteBatch, DB};

use common_apm::metrics::storage::{on_storage_get_state, on_storage_put_state};
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

pub struct RocksTrieDB {
    light: bool,
    db:    Arc<DB>,
}

impl RocksTrieDB {
    pub fn new<P: AsRef<Path>>(path: P, light: bool, max_open_files: i32) -> ProtocolResult<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_max_open_files(max_open_files);

        let db = DB::open(&opts, path).map_err(RocksTrieDBError::from)?;

        Ok(RocksTrieDB {
            light,
            db: Arc::new(db),
        })
    }
}

impl cita_trie::DB for RocksTrieDB {
    type Error = RocksTrieDBError;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Self::Error> {
        let inst = Instant::now();

        let res = self.db.get(key).map_err(to_store_err)?.map(|v| v.to_vec());

        on_storage_get_state(inst.elapsed(), 1i64);
        Ok(res)
    }

    fn contains(&self, key: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.db.get(key).map_err(to_store_err)?.is_some())
    }

    fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Self::Error> {
        let inst = Instant::now();
        let size = key.len() + value.len();

        self.db
            .put(Bytes::from(key), Bytes::from(value))
            .map_err(to_store_err)?;

        on_storage_put_state(inst.elapsed(), size as i64);
        Ok(())
    }

    fn insert_batch(&self, keys: Vec<Vec<u8>>, values: Vec<Vec<u8>>) -> Result<(), Self::Error> {
        let inst = Instant::now();

        if keys.len() != values.len() {
            return Err(RocksTrieDBError::BatchLengthMismatch);
        }

        let mut total_size = 0;
        let mut batch = WriteBatch::default();
        for i in 0..keys.len() {
            let key = &keys[i];
            let value = &values[i];

            total_size += key.len();
            total_size += value.len();
            batch.put(key, value);
        }

        self.db.write(batch).map_err(to_store_err)?;

        on_storage_put_state(inst.elapsed(), total_size as i64);
        Ok(())
    }

    fn remove(&self, key: &[u8]) -> Result<(), Self::Error> {
        if self.light {
            self.db.delete(key).map_err(to_store_err)?;
        }
        Ok(())
    }

    fn remove_batch(&self, keys: &[Vec<u8>]) -> Result<(), Self::Error> {
        if self.light {
            let mut batch = WriteBatch::default();
            for key in keys {
                batch.delete(key);
            }

            self.db.write(batch).map_err(to_store_err)?;
        }

        Ok(())
    }

    fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Display, From)]
pub enum RocksTrieDBError {
    #[display(fmt = "store error")]
    Store,

    #[display(fmt = "rocksdb {}", _0)]
    RocksDB(rocksdb::Error),

    #[display(fmt = "parameters do not match")]
    InsertParameter,

    #[display(fmt = "batch length dont match")]
    BatchLengthMismatch,
}

impl std::error::Error for RocksTrieDBError {}

impl From<RocksTrieDBError> for ProtocolError {
    fn from(err: RocksTrieDBError) -> ProtocolError {
        ProtocolError::new(ProtocolErrorKind::Binding, Box::new(err))
    }
}

fn to_store_err(e: rocksdb::Error) -> RocksTrieDBError {
    log::error!("[framework] trie db {:?}", e);
    RocksTrieDBError::Store
}
