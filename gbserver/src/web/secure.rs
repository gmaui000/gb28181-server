use crate::service;
use crate::utils::se_token;
use common::log::error;
use poem::web::Multipart;
use poem::Body;
use poem_openapi::payload::Binary;
use poem_openapi::{param::Query, OpenApi};
pub struct SecureApi;

#[OpenApi(prefix_path = "/secure")]
impl SecureApi {
    #[allow(non_snake_case)]
    #[oai(path = "/snap/upload", method = "post")]
    ///设备抓图上传 todo
    async fn snap_upload(
        &self,
        #[oai(name = "token")] token: Query<String>,
        #[oai(name = "SessionID")] SessionID: Query<String>,
        #[oai(name = "FileID")] FileID: Query<Option<String>>,
        #[oai(name = "SnapShotFileID")] SnapShotFileID: Query<Option<String>>,
        data: Binary<Body>,
    ) {
        if se_token::check_token(SessionID.0.as_str(), token.0.as_str()).is_ok() {
            let _ = service::control::upload(
                data,
                SessionID.0.clone(),
                FileID.0.clone(),
                SnapShotFileID.0.clone(),
            )
            .await;
        }
    }
}
