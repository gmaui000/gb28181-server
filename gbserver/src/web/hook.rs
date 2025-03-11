use crate::general::model::ResultMessageData;
use crate::service::{
    handler, BaseStreamInfo, OnPlayResponse, OnPlayerCountChangeResponse, OnPublishResponse,
    OnRtpServerTimeoutResponse, OnStreamChangedResponse, OnStreamNoneReaderResponse,
    OnStreamNotFoundResponse, PlayRequest, PlayerCountChangeRequest, PublishRequest,
    RtpServerTimeoutRequest, StreamChangedRequest, StreamNoneReaderRequest, StreamNotFoundRequest,
    StreamPlayInfo, StreamState,
};
use common::log::info;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;
pub struct HookApi;

#[OpenApi(prefix_path = "/index/hook")]
impl HookApi {
    ///流媒体发布鉴权事件；
    #[oai(path = "/on_publish", method = "post")]
    async fn on_publish(&self, publish_request: Json<PublishRequest>) -> Json<OnPublishResponse> {
        let request = publish_request.0;
        info!("on_publish = {:?}", &request);
        Json(handler::on_publish(request))
    }
    ///流媒体监测到用户点播流：发送一次用户点播流事件，用于鉴权
    #[oai(path = "/on_play", method = "post")]
    async fn on_play(&self, play_request: Json<PlayRequest>) -> Json<OnPlayResponse> {
        let request = play_request.0;
        info!("on_play = {:?}", &request);
        Json(handler::on_play(request))
    }

    ///流媒体监测到用户点播流：发送一次用户点播流事件，用于鉴权
    #[oai(path = "/on_player_count_change", method = "post")]
    async fn on_player_count_change(
        &self,
        player_count_change_request: Json<PlayerCountChangeRequest>,
    ) -> Json<OnPlayerCountChangeResponse> {
        let request = player_count_change_request.0;
        info!("on_play = {:?}", &request);
        Json(handler::on_player_count_change(request))
    }
    ///流媒体监流注册/注销事件：发送一次流注册/注销事件
    #[oai(path = "/on_stream_changed", method = "post")]
    async fn on_stream_changed(
        &self,
        stream_changed_request: Json<StreamChangedRequest>,
    ) -> Json<OnStreamChangedResponse> {
        let request = stream_changed_request.0;
        info!("on_stream_changed = {:?}", &request);
        // match request {
        //     StreamChangedRequest::Regist(regist) => {
        //         // Handle RegistRequest
        //     }
        //     StreamChangedRequest::UnRegist(unregist) => {
        //         // Handle UpdateRequest
        //     } // Add other variants as needed
        // }
        Json(handler::on_stream_changed(request))
    }
    ///流媒体监测到用户点播流：发送一次用户点播流事件，用于鉴权
    #[oai(path = "/on_stream_none_reader", method = "post")]
    async fn on_stream_none_reader(
        &self,
        stream_none_reader_request: Json<StreamNoneReaderRequest>,
    ) -> Json<OnStreamNoneReaderResponse> {
        let request = stream_none_reader_request.0;
        info!("on_stream_none_reader = {:?}", &request);
        Json(handler::on_stream_none_reader(request))
    }
    ///流媒体监测到用户点播流：发送一次用户点播流事件，用于鉴权
    #[oai(path = "/on_stream_not_found", method = "post")]
    async fn on_stream_not_found(
        &self,
        stream_not_found_request: Json<StreamNotFoundRequest>,
    ) -> Json<OnStreamNotFoundResponse> {
        let request = stream_not_found_request.0;
        info!("on_stream_not_found = {:?}", &request);
        Json(handler::on_stream_not_found(request))
    }
    ///流媒体监测到用户点播流：发送一次用户点播流事件，用于鉴权
    #[oai(path = "/on_rtp_server_timeout", method = "post")]
    async fn on_rtp_server_timeout(
        &self,
        rtp_server_timeout_request: Json<RtpServerTimeoutRequest>,
    ) -> Json<OnRtpServerTimeoutResponse> {
        let request = rtp_server_timeout_request.0;
        info!("on_rtp_server_timeout = {:?}", &request);
        Json(handler::on_rtp_server_timeout(request))
    }
    ///流媒体监听ssrc：接收到流输入，发送一次流注册事件；信令回调/api/play/xxx返回播放流信息
    #[oai(path = "/stream/in", method = "post")]
    async fn stream_in(
        &self,
        base_stream_info: Json<BaseStreamInfo>,
    ) -> Json<ResultMessageData<bool>> {
        let info = base_stream_info.0;
        info!("stream_in = {:?}", &info);
        handler::stream_in(info).await;
        Json(ResultMessageData::build_success_none())
    }
    ///流媒体监听ssrc：等待流8秒，超时未接收到；发送一次接收流超时事件；信令下发设备取消推流，并清理缓存会话；
    /// 【流注册等待超时，信令回调/api/play/xxx返回响应超时信息】
    #[oai(path = "/stream/input/timeout", method = "post")]
    async fn stream_input_timeout(
        &self,
        stream_state: Json<StreamState>,
    ) -> Json<ResultMessageData<bool>> {
        let info = stream_state.0;
        info!("stream_input_timeout = {:?}", &info);
        handler::stream_input_timeout(info);
        Json(ResultMessageData::build_success_none())
    }
    ///流媒体监测到用户断开点播流：发送一次用户关闭流事件：
    #[oai(path = "/off/play", method = "post")]
    async fn off_play(
        &self,
        stream_play_info: Json<StreamPlayInfo>,
    ) -> Json<ResultMessageData<bool>> {
        let info = stream_play_info.0;
        info!("off_play = {:?}", &info);
        Json(ResultMessageData::build_success(
            handler::off_play(info).await,
        ))
    }
    ///流媒体监测到无人连接媒体流：发送一次流空闲事件【配置为不关闭流，则不发送】：信令下发设备关闭推流，并清理缓存会话
    #[oai(path = "/stream/idle", method = "post")]
    async fn stream_idle(
        &self,
        stream_play_info: Json<BaseStreamInfo>,
    ) -> Json<ResultMessageData<bool>> {
        let info = stream_play_info.0;
        info!("stream_idle = {:?}", &info);
        Json(ResultMessageData::build_success(
            handler::stream_idle(info).await,
        ))
    }
}
