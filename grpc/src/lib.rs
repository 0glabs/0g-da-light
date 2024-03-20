use std::net::SocketAddr;

use service::{light::light_server::LightServer, LightService};
use tonic::transport::Server;

#[macro_use]
extern crate tracing;

mod service;

pub async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let encoder_service = LightService::new();
    Server::builder()
        .add_service(LightServer::new(encoder_service))
        .serve(addr)
        .await?;
    Ok(())
}
