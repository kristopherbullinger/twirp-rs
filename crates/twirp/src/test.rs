//! Test helpers and mini twirp api server implementation.
use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Body, Request};
use serde::de::DeserializeOwned;
use url::Url;

use crate::client::{request, TwirpClient, TwirpClientError};
use crate::*;

pub async fn test_api_router() -> Arc<Router> {
    let api = Arc::new(TestAPIServer {});
    let mut router = Router::default();
    // NB: This would be generated
    {
        let api = api.clone();
        router.add_method("/twirp/test.TestAPI/Ping", move |req| {
            let api = api.clone();
            async move { api.ping(req).await }
        });
    }
    {
        router.add_method("/twirp/test.TestAPI/Boom", move |req| {
            let api = api.clone();
            async move { api.boom(req).await }
        });
    }
    Arc::new(router)
}

pub fn gen_ping_request(name: &str) -> Request<hyper::Body> {
    let req = serde_json::to_string(&PingRequest {
        name: name.to_string(),
    })
    .expect("will always be valid json");
    Request::post("/twirp/test.TestAPI/Ping")
        .body(Body::from(req))
        .expect("always a valid twirp request")
}

pub async fn read_string_body(body: Body) -> String {
    let data = hyper::body::to_bytes(body)
        .await
        .expect("invalid body")
        .to_vec();
    String::from_utf8(data).expect("non-utf8 body")
}

pub async fn read_json_body<T>(body: Body) -> T
where
    T: DeserializeOwned,
{
    let data = hyper::body::to_bytes(body)
        .await
        .expect("invalid body")
        .to_vec();
    serde_json::from_slice(&data).expect("twirp response isn't valid JSON")
}

pub async fn read_err_body(body: Body) -> TwirpErrorResponse {
    read_json_body(body).await
}

// Hand written sample test server and client

pub struct TestAPIServer;

#[async_trait]
impl TestAPI for TestAPIServer {
    async fn ping(&self, req: PingRequest) -> Result<PingResponse, TwirpErrorResponse> {
        Ok(PingResponse { name: req.name })
    }

    async fn boom(&self, _: PingRequest) -> Result<PingResponse, TwirpErrorResponse> {
        Err(internal("boom!"))
    }
}

// Small test twirp services (this would usually be generated with twirp-build)

#[async_trait]
pub trait TestAPIClientExt {
    fn ping_url(&self, base_url: &Url) -> Result<Url, TwirpClientError> {
        let url = base_url.join("twirp/test.testAPI/Ping")?;
        Ok(url)
    }
    async fn ping_inner(
        &self,
        url: Url,
        req: PingRequest,
    ) -> Result<PingResponse, TwirpClientError>;
}

#[async_trait]
impl TestAPIClientExt for TwirpClient {
    async fn ping_inner(
        &self,
        url: Url,
        req: PingRequest,
    ) -> Result<PingResponse, TwirpClientError> {
        request(self.client.post(url), req).await
    }
}

#[async_trait]
pub trait TestAPIClient {
    async fn ping(&self, req: PingRequest) -> Result<PingResponse, TwirpClientError>;
    async fn boom(&self, req: PingRequest) -> Result<PingResponse, TwirpClientError>;
}

#[async_trait]
impl TestAPIClient for TwirpClient {
    async fn ping(&self, req: PingRequest) -> Result<PingResponse, TwirpClientError> {
        self.ping_inner(self.ping_url(&self.base_url)?, req).await
    }

    async fn boom(&self, _req: PingRequest) -> Result<PingResponse, TwirpClientError> {
        todo!()
    }
}

#[async_trait]
pub trait TestAPI {
    async fn ping(&self, req: PingRequest) -> Result<PingResponse, TwirpErrorResponse>;
    async fn boom(&self, req: PingRequest) -> Result<PingResponse, TwirpErrorResponse>;
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PingRequest {
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PingResponse {
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
}
