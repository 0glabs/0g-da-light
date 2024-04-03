use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use ethereum_types::H256;
use jsonrpsee::http_client::HttpClient;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use zgs_rpc::ZgsRPCClient;

const MAX_DOWNLOAD_TASK: usize = 5;
const MAX_RETRY: usize = 5;
pub const ENTRY_SIZE: usize = 256;
pub const ENTRIES_PER_SEGMENT: usize = 1024;
const RETRY_WAIT_MS: u64 = 1000;

pub async fn download_segments(
    clients: Vec<HttpClient>,
    data_root: H256,
    segment_indexes: Vec<usize>,
) -> Result<Vec<Vec<u8>>> {
    let mut task_counter = 0;
    let mut task_index = 0;
    let (sender, mut rx) = unbounded_channel();
    while task_index < segment_indexes.len() && task_counter < MAX_DOWNLOAD_TASK {
        tokio::spawn(download_with_proof(
            task_index,
            clients.clone(),
            data_root,
            segment_indexes[task_index],
            sender.clone(),
        ));
        task_index += 1;
        task_counter += 1;
    }
    let mut result = vec![vec![]; segment_indexes.len()];
    let mut failed_tasks = HashMap::new();
    while task_index < segment_indexes.len() || task_counter > 0 {
        if let Some((id, maybe_data)) = rx.recv().await {
            match maybe_data {
                Some(data) => {
                    result[id] = data;
                    if task_index < segment_indexes.len() {
                        tokio::spawn(download_with_proof(
                            task_index,
                            clients.clone(),
                            data_root,
                            segment_indexes[task_index],
                            sender.clone(),
                        ));
                        task_index += 1;
                    } else {
                        task_counter -= 1;
                    }
                }
                None => {
                    match failed_tasks.get_mut(&id) {
                        Some(c) => {
                            if *c == MAX_RETRY {
                                bail!(anyhow!(format!(
                                    "Download segment with index {:?} failed, data root: {:x?}",
                                    segment_indexes[id], data_root,
                                )));
                            }
                            *c += 1;
                        }
                        _ => {
                            failed_tasks.insert(id, 1);
                        }
                    };
                    // TODO: request new file
                    tokio::spawn(download_with_proof(
                        id,
                        clients.clone(),
                        data_root,
                        segment_indexes[id],
                        sender.clone(),
                    ));
                }
            }
        }
    }
    Ok(result)
}

async fn download_with_proof(
    task_index: usize,
    clients: Vec<HttpClient>,
    data_root: H256,
    segment_index: usize,
    sender: UnboundedSender<(usize, Option<Vec<u8>>)>,
) {
    let mut client_index = 0;
    while client_index < clients.len() {
        match clients[client_index]
            .download_segment_with_proof(data_root, segment_index)
            .await
        {
            Ok(Some(segment)) => {
                if segment.data.len() % ENTRY_SIZE != 0 {
                    if let Err(e) = sender.send((task_index, None)) {
                        error!("send error: {:?}", e);
                    }

                    return;
                }

                if segment.root != data_root {
                    if let Err(e) = sender.send((task_index, None)) {
                        error!("send error: {:?}", e);
                    }

                    return;
                }

                if let Err(_) = segment.validate(ENTRIES_PER_SEGMENT) {
                    if let Err(e) = sender.send((task_index, None)) {
                        error!("send error: {:?}", e);
                    }
                    return;
                }

                if let Err(e) = sender.send((task_index, Some(segment.data))) {
                    error!("send error: {:?}", e);
                }

                return;
            }
            Ok(None) => {
                client_index += 1;
                tokio::time::sleep(Duration::from_millis(RETRY_WAIT_MS)).await;
            }
            Err(_e) => {
                client_index += 1;
                tokio::time::sleep(Duration::from_millis(RETRY_WAIT_MS)).await;
            }
        }
    }

    if let Err(e) = sender.send((task_index, None)) {
        error!("send error: {:?}", e);
    }
}
