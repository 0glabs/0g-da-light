use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BatchHeader {
    #[serde(with = "base64")]
    pub batch_root: Vec<u8>,
    #[serde(with = "base64")]
    pub data_root: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BlobDisperseInfo {
    pub blob_length: u64,
    pub rows: u32,
    pub cols: u32,
}

#[derive(Serialize, Deserialize)]
pub struct KVBatchInfo {
    pub batch_header: BatchHeader,
    pub blob_disperse_infos: Vec<BlobDisperseInfo>,
}

#[derive(Clone)]
pub struct BlobLocation {
    pub segment_indexes: Vec<u32>,
    pub offsets: Vec<u32>,
}

mod base64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = base64::encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let base64 = String::deserialize(d)?;
        base64::decode(base64.as_bytes()).map_err(serde::de::Error::custom)
    }
}
