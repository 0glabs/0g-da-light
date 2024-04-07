use anyhow::Result;
use common::types::KVBatchInfo;
use ethereum_types::H256;
use jsonrpsee::http_client::HttpClient;
use kv_rpc::KeyValueRpcClient;
use zgs_rpc::types::Segment;

const MAX_QUERY_SIZE: u64 = 256 * 1024; // 256 KB

pub async fn fetch_kv_batch_info(
    client: HttpClient,
    stream_id: H256,
    batch_header_hash: Vec<u8>,
) -> Result<Option<KVBatchInfo>> {
    let mut raw_value = vec![];
    loop {
        if let Some(result) = client
            .get_value(
                stream_id,
                Segment(batch_header_hash.clone()),
                raw_value.len() as u64,
                MAX_QUERY_SIZE,
                None,
            )
            .await?
        {
            raw_value.extend(result.data);
            if raw_value.len() as u64 == result.size {
                break;
            }
        } else {
            return Ok(None);
        }
    }
    Ok(serde_json::from_slice(&raw_value)?)
}
