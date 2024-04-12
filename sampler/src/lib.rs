#[macro_use]
extern crate tracing;

use std::{collections::HashSet, error::Error, sync::Arc};

use anyhow::{anyhow, bail, Result};
use common::{allocate_rows, types::BlobLocation, COMMITMENT_SIZE};
use data_fetcher::{kv_fetcher::fetch_kv_batch_info, zgs_fetcher::download_segments};
use ethereum_types::H256;
use jsonrpsee::http_client::HttpClient;
use kate::gridgen::{AsBytes, EvaluationGrid};
use kate_recovery::{
    data::Cell,
    matrix::{Dimensions, Position},
    proof,
};
use kv_rpc::build_client;
use rand::{thread_rng, Rng};

pub struct Sampler {
    zgs_clients: Vec<HttpClient>,
    // kv settings
    kv_client: HttpClient,
}

/// Generates random cell positions for sampling
pub fn generate_random_cells(dimensions: Dimensions, cell_count: u32) -> Vec<Position> {
    let max_cells = dimensions.size();
    let count = if max_cells < cell_count {
        debug!("Max cells count {max_cells} is lesser than cell_count {cell_count}");
        max_cells
    } else {
        cell_count
    };
    let mut rng = thread_rng();
    let mut indices = HashSet::new();
    while (indices.len() as u16) < count as u16 {
        let col = rng.gen_range(0..dimensions.cols().into());
        let row = rng.gen_range(0..dimensions.rows().into());
        indices.insert(Position {
            row: row.into(),
            col,
        });
    }

    indices.into_iter().collect::<Vec<_>>()
}

impl Sampler {
    pub fn new(zgs_urls: Vec<String>, kv_url: &String) -> Result<Self> {
        Ok(Self {
            zgs_clients: zgs_urls
                .iter()
                .map(build_client)
                .collect::<Result<Vec<HttpClient>, Box<dyn Error>>>()
                .map_err(|e| anyhow!(e.to_string()))?,
            kv_client: build_client(kv_url).map_err(|e| anyhow!(e.to_string()))?,
        })
    }

    pub async fn sample(
        &self,
        stream_id: H256,
        batch_header_hash: Vec<u8>,
        blob_index: u32,
        times: u32,
    ) -> Result<bool> {
        let mut timer = std::time::Instant::now();
        if let Some(batch_info) =
            fetch_kv_batch_info(self.kv_client.clone(), stream_id, batch_header_hash).await?
        {
            info!(
                "fetch kv batch info used {:?}ms",
                timer.elapsed().as_millis()
            );
            timer = std::time::Instant::now();

            if batch_info.blob_disperse_infos.len() <= blob_index as usize {
                bail!(anyhow!("invalid blob index"));
            }

            let rows = batch_info.blob_disperse_infos[blob_index as usize].rows;
            let cols = batch_info.blob_disperse_infos[blob_index as usize].cols;
            let Some(dimensions) = Dimensions::new(rows as u16, cols as u16) else {
                info!(
                    "Skipping block with invalid dimensions {:?}x{:?}",
                    rows, cols
                );
                return Ok(false);
            };
            let data_root = batch_info.batch_header.data_root;
            let blob_locations = allocate_rows(&batch_info.blob_disperse_infos);
            let positions = generate_random_cells(dimensions, times);

            info!(
                "generate sample positions used {:?}ms, matrix {:?}x{:?}",
                timer.elapsed().as_millis(),
                rows,
                cols
            );

            match self
                .verify_cells(
                    dimensions,
                    &blob_locations[blob_index as usize],
                    data_root,
                    positions,
                )
                .await
            {
                Ok(success) => Ok(success),
                Err(e) => {
                    debug!("sample failed with error {:?}", e.to_string());
                    Ok(false)
                }
            }
        } else {
            bail!(anyhow!("batch not found"));
        }
    }

    pub async fn verify_cells(
        &self,
        dimensions: Dimensions,
        location: &BlobLocation,
        data_root: H256,
        positions: Vec<Position>,
    ) -> Result<bool> {
        let mut timer = std::time::Instant::now();

        let segment_indexes = positions
            .iter()
            .map(|x| location.segment_indexes[x.row as usize] as usize)
            .collect();
        let offsets: Vec<usize> = positions
            .iter()
            .map(|x| location.offsets[x.row as usize] as usize)
            .collect();
        let row_byte_size = dimensions.row_byte_size();
        let segments =
            download_segments(self.zgs_clients.clone(), data_root, segment_indexes).await?;

        info!("download segments used {:?}ms", timer.elapsed().as_millis());

        let cols = u16::from(dimensions.cols()) as usize;
        let pp = Arc::new(kate_recovery::couscous::public_params());
        for ((segment, offset), position) in
            segments.iter().zip(offsets.iter()).zip(positions.iter())
        {
            timer = std::time::Instant::now();
            // generate 1-row matrix
            let evals = EvaluationGrid::from_row_slices(
                1,
                cols,
                segment[*offset..*offset + row_byte_size].to_vec(),
            )
            .map_err(|e| anyhow!(format!("Grid construction failed: {:?}", e)))?;
            // make polynomial
            let polys = evals
                .make_polynomial_grid()
                .map_err(|e| anyhow!(format!("Make polynomial grid failed: {:?}", e)))?;

            let Some(data) = evals.get::<usize, usize>(0, position.col as usize) else {
                bail!(anyhow!(
                    "Invalid position {:?} for dims {:?}",
                    position,
                    evals.dims()
                ));
            };
            let proof = match polys.proof(
                &kate::couscous::multiproof_params(),
                &kate::com::Cell {
                    row: zerog_core::BlockLengthRows(0),
                    col: zerog_core::BlockLengthColumns(position.col.into()),
                },
            ) {
                Ok(x) => x,
                Err(e) => bail!(anyhow!("Unable to make proof: {:?}", e)),
            };

            let data = data.to_bytes().expect("Ser cannot fail").to_vec();
            let proof = proof.to_bytes().expect("Ser cannot fail").to_vec();
            let content = [proof, data].into_iter().flatten().collect::<Vec<_>>();
            let cell = Cell {
                position: Position {
                    row: 0,
                    col: position.col,
                },
                content: content.as_slice().try_into()?,
            };
            let commitment: &[u8; 48] = segment
                [*offset + row_byte_size..*offset + row_byte_size + COMMITMENT_SIZE as usize]
                .try_into()?;
            info!("generate proof used {:?}ms", timer.elapsed().as_millis());
            timer = std::time::Instant::now();

            let verification_success = proof::verify(&pp, dimensions, commitment, &cell)?;
            info!("verification used {:?}ms", timer.elapsed().as_millis());

            if !verification_success {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
