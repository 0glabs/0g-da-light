use sampler::Sampler;
use tonic::{Code, Request, Response, Status};

use self::light::{
    light_server::Light, RetrieveReply, RetrieveRequest, SampleReply, SampleRequest,
};

pub mod light {
    tonic::include_proto!("light");
}

pub struct LightService {
    sampler: Sampler,
}

impl LightService {
    pub fn new(sampler: Sampler) -> Self {
        Self { sampler }
    }
}

#[tonic::async_trait]
impl Light for LightService {
    async fn sample(
        &self,
        request: Request<SampleRequest>,
    ) -> Result<Response<SampleReply>, Status> {
        let remote_addr = request.remote_addr();
        let request_content = request.into_inner();
        info!(
            "Received request from {:?}, blob_header_hash: {:x?}, blob_index: {:?}, times: {:?}",
            remote_addr,
            request_content.batch_header_hash,
            request_content.blob_index,
            request_content.times,
        );
        match self
            .sampler
            .sample(
                request_content.batch_header_hash,
                request_content.blob_index,
                request_content.times,
            )
            .await
        {
            Ok(result) => Ok(Response::new(SampleReply { success: result })),
            Err(msg) => Err(Status::new(Code::Internal, msg.to_string())),
        }
    }

    async fn retrieve(
        &self,
        _request: Request<RetrieveRequest>,
    ) -> Result<Response<RetrieveReply>, Status> {
        todo!()
    }
}
