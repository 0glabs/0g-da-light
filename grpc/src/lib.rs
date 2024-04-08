use std::net::SocketAddr;

use sampler::Sampler;
use service::{light::light_server::LightServer, LightService};
use tonic::transport::Server;

#[macro_use]
extern crate tracing;

mod service;

pub async fn run_server(
    addr: SocketAddr,
    sampler: Sampler,
) -> Result<(), Box<dyn std::error::Error>> {
    let encoder_service = LightService::new(sampler);
    Server::builder()
        .add_service(LightServer::new(encoder_service))
        .serve(addr)
        .await?;
    Ok(())
}
