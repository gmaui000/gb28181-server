use crate::service;
use common::log::error;
use poem::web::Multipart;
use poem_openapi::{param::Query, OpenApi};
pub struct SecureApi;

#[OpenApi(prefix_path = "/secure")]
impl SecureApi {
    #[allow(non_snake_case)]
    #[oai(path = "/snap/upload", method = "post")]
    ///设备抓图上传 todo
    async fn snap_upload(
        &self,
        #[oai(name = "uk")] uk: Query<String>,
        #[oai(name = "sessionId")] sessionId: Query<Option<String>>,
        #[oai(name = "fileId")] fileId: Query<Option<String>>,
        #[oai(name = "snapShotFileID")] snapShotFileID: Query<Option<String>>,
        mut multipart: Multipart,
    ) {
        loop {
            match multipart.next_field().await {
                Ok(Some(field)) => {
                    let _ = service::control::upload(
                        field,
                        uk.0.clone(),
                        sessionId.0.clone(),
                        fileId.0.clone(),
                        snapShotFileID.0.clone(),
                    )
                    .await;
                }
                Ok(None) => {
                    break;
                }
                Err(err) => {
                    error!("{}", err)
                }
            }
        }
    }
}
