use crate::general::model::*;
use crate::service::handler;
use common::exception::GlobalError;
use common::log::{error, info};
use poem_openapi::param::{Header, Path, Query};
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

pub struct RestApi;

#[OpenApi(prefix_path = "/api/v1")]
impl RestApi {
    #[allow(non_snake_case)]
    #[oai(path = "/stream/start/:device_id/:channel_id", method = "get")]
    /// 点播监控实时画面 transMode 默认0 udp 模式, 1 tcp 被动模式,2 tcp 主动模式
    async fn stream_start(
        &self,
        #[oai(name = "gbs-token")] token: Header<String>,
        #[oai(name = "device_id", validator(min_length = "20", max_length = "20"))] device_id: Path<
            String,
        >,
        #[oai(name = "channel_id", validator(min_length = "20", max_length = "20"))]
        channel_id: Path<String>,
        #[oai(
            name = "trans_mode",
            validator(maximum(value = "2"), minimum(value = "0"))
        )]
        trans_mode: Query<Option<u8>>,
    ) -> Json<ResultMessageData<StreamInfo>> {
        let header = token.0;
        let trans_mode = trans_mode.0;
        let live_model = PlayLiveModel::new(device_id.0, Some(channel_id.0), trans_mode);
        info!("play_live:header = {:?},body = {:?}", &header, &live_model);
        match handler::play_live(live_model, header).await {
            Ok(data) => Json(ResultMessageData::build_success(data)),
            Err(err) => {
                error!("{}", err.to_string());
                match err {
                    GlobalError::BizErr(e) => Json(ResultMessageData::build_failure_msg(e.msg)),
                    GlobalError::SysErr(_e) => Json(ResultMessageData::build_failure()),
                }
            }
        }
    }

    #[allow(non_snake_case)]
    #[oai(path = "/play/live/stream", method = "post")]
    /// 点播监控实时画面 transMode 默认0 udp 模式, 1 tcp 被动模式,2 tcp 主动模式
    async fn play_live(
        &self,
        live: Json<PlayLiveModel>,
        #[oai(name = "gbs-token")] token: Header<String>,
    ) -> Json<ResultMessageData<StreamInfo>> {
        let header = token.0;
        let live_model = live.0;
        info!("play_live:header = {:?},body = {:?}", &header, &live_model);
        match handler::play_live(live_model, header).await {
            Ok(data) => Json(ResultMessageData::build_success(data)),
            Err(err) => {
                error!("{}", err.to_string());
                match err {
                    GlobalError::BizErr(e) => Json(ResultMessageData::build_failure_msg(e.msg)),
                    GlobalError::SysErr(_e) => Json(ResultMessageData::build_failure()),
                }
            }
        }
    }

    #[allow(non_snake_case)]
    #[oai(path = "/play/back/stream", method = "post")]
    /// 点播监控历史画面 transMode 默认0 udp 模式, 1 tcp 被动模式,2 tcp 主动模式
    async fn play_back(
        &self,
        back: Json<PlayBackModel>,
        #[oai(name = "gbs-token")] token: Header<String>,
    ) -> Json<ResultMessageData<StreamInfo>> {
        let header = token.0;
        let back_model = back.0;
        info!("back_model:header = {:?},body = {:?}", &header, &back_model);
        match handler::play_back(back_model, header).await {
            Ok(data) => Json(ResultMessageData::build_success(data)),
            Err(err) => {
                error!("{}", err.to_string());
                match err {
                    GlobalError::BizErr(e) => Json(ResultMessageData::build_failure_msg(e.msg)),
                    GlobalError::SysErr(_e) => Json(ResultMessageData::build_failure()),
                }
            }
        }
    }

    #[allow(non_snake_case)]
    #[oai(path = "/play/back/seek", method = "post")]
    /// 拖动播放录像 seek 拖动秒 [1-86400]
    async fn playback_seek(
        &self,
        seek: Json<PlaySeekModel>,
        #[oai(name = "gbs-token")] token: Header<String>,
    ) -> Json<ResultMessageData<bool>> {
        let header = token.0;
        let seek_model = seek.0;
        info!("back-seek:header = {:?},body = {:?}", &header, &seek_model);
        match handler::seek(seek_model, header).await {
            Err(err) => {
                let err_msg = format!("拖动失败；{}", err);
                error!("{}", &err_msg);
                Json(ResultMessageData::build_failure_msg(err_msg))
            }
            Ok(_) => Json(ResultMessageData::build_success(true)),
        }
    }
    #[allow(non_snake_case)]
    #[oai(path = "/play/back/speed", method = "post")]
    /// 倍速播放历史视频 speed [1,2,4]
    async fn playback_speed(
        &self,
        speed: Json<PlaySpeedModel>,
        #[oai(name = "gbs-token")] token: Header<String>,
    ) -> Json<ResultMessageData<bool>> {
        let header = token.0;
        let speed_model = speed.0;
        info!(
            "back-speed:header = {:?},body = {:?}",
            &header, &speed_model
        );
        match handler::speed(speed_model, header).await {
            Err(err) => {
                let err_msg = format!("倍速播放失败；{}", err);
                error!("{}", &err_msg);
                Json(ResultMessageData::build_failure_msg(err_msg))
            }
            Ok(_) => Json(ResultMessageData::build_success(true)),
        }
    }

    #[allow(non_snake_case)]
    #[oai(path = "/control/ptz", method = "post")]
    /// 云台控制
    async fn control_ptz(
        &self,
        ptz: Json<PtzControlModel>,
        #[oai(name = "gbs-token")] token: Header<String>,
    ) -> Json<ResultMessageData<bool>> {
        let header = token.0;
        let ptz_model = ptz.0;
        info!("control_ptz:header = {:?},body = {:?}", &header, &ptz_model);
        match handler::ptz(ptz_model, header).await {
            Err(err) => {
                let err_msg = format!("云台控制失败；{}", err);
                error!("{}", &err_msg);
                Json(ResultMessageData::build_failure_msg(err_msg))
            }
            Ok(_) => Json(ResultMessageData::build_success(true)),
        }
    }
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/back/speed", method = "get")]
    // /// 倍速播放历史视频 speed [1,2,4]
    // async fn playback_speed(&self,
    //                         #[oai(name = "streamId", validator(min_length = "32", max_length = "32"))] streamId: Query<String>,
    //                         #[oai(name = "speed", validator(maximum(value = "4"), minimum(value = "1")))] speed: Query<u8>) -> Json<ResultMessageData<Option<ResMsg>>> {
    //     match handler::speed(&streamId.0, speed.0).await {
    //         Err(err) => {
    //             error!("倍速播放失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(msg) => { Json(ResultMessageData::build_success(Some(msg))) }
    //     }
    // }

    //
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/save/info", method = "get")]
    // /// 查看录像信息
    // async fn down_info(&self,
    //                    #[oai(name = "deviceId", validator(min_length = "20", max_length = "20"))] deviceId: Query<String>,
    //                    #[oai(name = "channelId", validator(min_length = "20", max_length = "20"))] channelId: Query<String>) -> Json<ResultMessageData<Option<Vec<RecordInfo>>>> {
    //     match handler::query_down_info(&deviceId.0, &channelId.0).await {
    //         Err(err) => {
    //             error!("查看录像信息失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(info) => { Json(ResultMessageData::build_success(Some(info))) }
    //     }
    // }
    //
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/back/save", method = "get")]
    // /// 开启监控历史画面云端录制 transMode 默认0 udp 模式, 1 tcp 被动模式,2 tcp 主动模式， 目前只支持 0 下载速度124   同人同录像机同摄像头只能同时下载一路监控
    // async fn download(&self,
    //                   #[oai(name = "deviceId", validator(min_length = "20", max_length = "20"))] deviceId: Query<String>,
    //                   #[oai(name = "channelId", validator(min_length = "20", max_length = "20"))] channelId: Query<String>,
    //                   #[oai(name = "identity", validator(min_length = "4", max_length = "32"))] _identity: Query<String>,
    //                   #[oai(name = "fileName")] _fileName: Query<String>,
    //                   #[oai(name = "st", validator(minimum(value = "1577808000")))] st: Query<u32>,
    //                   #[oai(name = "et", validator(minimum(value = "1577808001")))] et: Query<u32>,
    //                   #[oai(name = "speed", validator(maximum(value = "4"), minimum(value = "1")))] _speed: Query<u8>,
    //                   #[oai(name = "transMode", validator(maximum(value = "2"), minimum(value = "0")))] _transMode: Query<u8>) -> Json<ResultMessageData<Option<bool>>> {
    //     let dt = Local::now();
    //     match handler::down(&deviceId.0, &channelId.0, 0, st.0, et.0, 4, "twoLevel", dt.timestamp().to_string()).await {
    //         Err(err) => {
    //             error!("下载失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(b) => { Json(ResultMessageData::build_success(Some(b))) }
    //     }
    // }
    //
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/save/break", method = "get")]
    // /// 提前终止云端录像任务
    // async fn save_break(&self,
    //                     #[oai(name = "id", validator(min_length = "32", max_length = "32"))] id: Query<String>) -> Json<ResultMessageData<Option<bool>>> {
    //     match handler::teardown_save(&id.0).await {
    //         Err(err) => {
    //             error!("终止失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(_info) => { Json(ResultMessageData::build_success_none()) }
    //     }
    // }
    //
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/back/stream", method = "get")]
    // /// 点播监控历史画面 transMode 默认0 udp 模式, 1 tcp 被动模式,2 tcp 主动模式， 目前只支持 0  时间跨度不超过24H
    // /// 相机不能同时观看不同的时间段监控？？？是否优化？不同人观看不同？前端设备可以抗住几路并发
    // async fn playback(&self,
    //                   #[oai(name = "deviceId", validator(min_length = "20", max_length = "20"))] deviceId: Query<String>,
    //                   #[oai(name = "channelId", validator(min_length = "20", max_length = "20"))] channelId: Query<String>,
    //                   // #[oai(name = "userId", validator(min_length = "4", max_length = "32"))] _userId: Query<String>,
    //                   #[oai(name = "st", validator(minimum(value = "1577808000")))] st: Query<u32>,
    //                   #[oai(name = "et", validator(minimum(value = "1577808001")))] et: Query<u32>,
    //                   #[oai(name = "transMode", validator(maximum(value = "2"), minimum(value = "0")))] _transMode: Query<u8>) -> Json<ResultMessageData<Option<StreamInfo>>> {
    //     match handler::playback(&deviceId.0, &channelId.0, 0, st.0, et.0, "twoLevel").await {
    //         Err(err) => {
    //             error!("点播失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(info) => { Json(ResultMessageData::build_success(Some(info))) }
    //     }
    // }

    // #[allow(non_snake_case)]
    // #[oai(path = "/play/back/pause", method = "get")]
    // /// 暂停播放历史视频
    // async fn playback_pause(&self,
    //                         #[oai(name = "streamId", validator(min_length = "32", max_length = "32"))] streamId: Query<String>) -> Json<ResultMessageData<Option<ResMsg>>> {
    //     match handler::pause(&streamId.0).await {
    //         Err(err) => {
    //             error!("暂停播放失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(msg) => { Json(ResultMessageData::build_success(Some(msg))) }
    //     }
    // }
    // #[allow(non_snake_case)]
    // #[oai(path = "/play/back/replay", method = "get")]
    // /// 恢复播放历史视频
    // async fn playback_replay(&self,
    //                          #[oai(name = "streamId", validator(min_length = "32", max_length = "32"))] streamId: Query<String>) -> Json<ResultMessageData<Option<ResMsg>>> {
    //     match handler::replay(&streamId.0).await {
    //         Err(err) => {
    //             error!("恢复播放失败；{}",err);
    //             Json(ResultMessageData::build_failure())
    //         }
    //         Ok(msg) => { Json(ResultMessageData::build_success(Some(msg))) }
    //     }
    // }
}
