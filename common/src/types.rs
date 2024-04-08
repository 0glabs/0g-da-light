use ethereum_types::H256;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchHeader {
    pub batch_root: Vec<u8>,
    pub data_root: H256,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlobDisperseInfo {
    pub blob_length: u64,
    pub rows: u32,
    pub cols: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KVBatchInfo {
    pub batch_header: BatchHeader,
    pub blob_disperse_infos: Vec<BlobDisperseInfo>,
}

#[derive(Clone, Debug)]
pub struct BlobLocation {
    pub segment_indexes: Vec<u32>,
    pub offsets: Vec<u32>,
}
