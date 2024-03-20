use tonic::{Request, Response, Status};

use self::light::{light_server::Light, ConfidenceReply, ConfidenceRequest};

pub mod light {
    tonic::include_proto!("light");
}

pub struct LightService {
}

impl LightService {
    pub fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl Light for LightService {
    async fn confidence(&self, request: Request<ConfidenceRequest>) -> Result<Response<ConfidenceReply>, Status> {
        todo!()
    }
}