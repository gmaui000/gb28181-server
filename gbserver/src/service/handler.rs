use crate::gb::handler::cmd::{CmdControl, CmdStream};
use crate::gb::RWSession;
use crate::general;
use crate::general::cache::PlayType;
use crate::general::model::*;
use crate::service::*;
use crate::utils::id_builder;
use common::bytes::Bytes;
use common::exception::{GlobalError, GlobalResult, TransError};
use common::log::error;
use common::serde_json;
use common::tokio::sync::mpsc;
use common::tokio::time::{sleep, Instant};
use std::time::Duration;

const KEY_STREAM_IN: &str = "KEY_STREAM_IN:";

pub fn on_publish(_: PublishRequest) -> OnPublishResponse {
    // true
    OnPublishResponse {
        code: 0,
        msg: "".to_string(),
        enable_hls: true,
        enable_hls_fmp4: true,
        enable_mp4: true,
        enable_rtsp: true,
        enable_rtmp: true,
        enable_ts: true,
        enable_fmp4: true,
        hls_demand: false,
        rtsp_demand: false,
        rtmp_demand: false,
        ts_demand: false,
        fmp4_demand: false,
        enable_audio: true,
        add_mute_audio: true,
        mp4_save_path: "".to_string(),
        mp4_save_second: 5,
        mp4_as_player: true,
        hls_save_path: "".to_string(),
        modify_stamp: true,
        continue_push_ms: 0,
        auto_close: true,
        stream_replace: "".to_string(),
    }
}

pub fn on_play(_: PlayRequest) -> OnPlayResponse {
    OnPlayResponse {
        code: 0,
        msg: "".to_string(),
    }
}

pub fn on_player_count_change(_: PlayerCountChangeRequest) -> OnPlayerCountChangeResponse {
    OnPlayerCountChangeResponse {
        code: 0,
        msg: "".to_string(),
    }
}

pub fn on_stream_changed(_: StreamChangedRequest) -> OnStreamChangedResponse {
    OnStreamChangedResponse {
        code: 0,
        msg: "".to_string(),
    }
}

pub fn on_stream_none_reader(_: StreamNoneReaderRequest) -> OnStreamNoneReaderResponse {
    OnStreamNoneReaderResponse {
        code: 0,
        close: true,
    }
}

pub fn on_stream_not_found(_: StreamNotFoundRequest) -> OnStreamNotFoundResponse {
    OnStreamNotFoundResponse {
        code: 0,
        msg: "".to_string(),
    }
}
pub fn on_rtp_server_timeout(_: RtpServerTimeoutRequest) -> OnRtpServerTimeoutResponse {
    OnRtpServerTimeoutResponse {
        code: 0,
        msg: "".to_string(),
    }
}

//无人观看则关闭流
pub async fn stream_idle(base_stream_info: BaseStreamInfo) -> bool {
    let stream_id = base_stream_info.get_stream_id();
    let cst_info = general::cache::Cache::stream_map_build_call_id_seq_from_to_tag(stream_id);

    let (device_id, channel_id, ssrc) = id_builder::de_stream_id(stream_id);
    if let Some((call_id, seq, from_tag, to_tag)) = cst_info {
        let _ =
            CmdStream::play_bye(seq, call_id, &device_id, &channel_id, &from_tag, &to_tag).await;
    }
    if let Some(play_type) =
        general::cache::Cache::stream_map_query_play_type_by_stream_id(stream_id)
    {
        general::cache::Cache::device_map_remove(
            &device_id,
            Some((&channel_id, Some((play_type, &ssrc)))),
        );
        general::cache::Cache::stream_map_remove(stream_id, None);
    }
    let ssrc = base_stream_info.rtp_info.ssrc;
    let ssrc_num = (ssrc % 10000) as u16;
    general::cache::Cache::ssrc_sn_set(ssrc_num);
    true
}

pub async fn off_play(stream_play_info: StreamPlayInfo) -> bool {
    let stream_id = stream_play_info.base_stream_info.get_stream_id();
    let gbs_token = stream_play_info.get_token();
    general::cache::Cache::stream_map_remove(stream_id, Some(gbs_token));
    true
}

pub async fn stream_in(base_stream_info: BaseStreamInfo) {
    let key_stream_in_id = format!("{KEY_STREAM_IN}{}", base_stream_info.get_stream_id());
    if let Some((_, Some(tx))) = general::cache::Cache::state_get(&key_stream_in_id) {
        let vec = serde_json::to_vec(&base_stream_info).unwrap();
        let bytes = Bytes::from(vec);
        let _ = tx.try_send(Some(bytes)).hand_log(|msg| error!("{msg}"));
    }
}

//gbs-stream接收流超时:还ssrc_sn,清理stream_map/device_map
pub fn stream_input_timeout(stream_state: StreamState) {
    let ssrc = stream_state.base_stream_info.rtp_info.ssrc;
    let ssrc_num = (ssrc % 10000) as u16;
    general::cache::Cache::ssrc_sn_set(ssrc_num);
    let stream_id = stream_state.base_stream_info.get_stream_id();
    if let Some(play_type) =
        general::cache::Cache::stream_map_query_play_type_by_stream_id(stream_id)
    {
        general::cache::Cache::stream_map_remove(stream_id, None);
        let (device_id, channel_id, ssrc) = id_builder::de_stream_id(stream_id);
        general::cache::Cache::device_map_remove(
            &device_id,
            Some((&channel_id, Some((play_type, &ssrc)))),
        );
    }
}

/*
1.检查设备状态：是否在线
2.判断通道是否为单IPC
3.开启直播流
4.建立流与用户关系
*/
pub async fn play_live(play_live_model: PlayLiveModel, token: String) -> GlobalResult<StreamInfo> {
    let device_id = play_live_model.get_device_id();
    if !RWSession::has_session_by_device_id(device_id) {
        return Err(GlobalError::new_biz_error(1000, "设备已离线", |msg| {
            error!("{msg}")
        }));
    }
    let channel_id = if let Some(channel_id) = play_live_model.get_channel_id() {
        channel_id
    } else {
        device_id
    };
    let play_type = PlayType::Live;
    //查看直播流是否已存在,有则直接返回
    if let Some((stream_id, node_name)) =
        enable_invite_stream(device_id, channel_id, &token, &play_type).await
    {
        general::cache::Cache::stream_map_insert_token(stream_id.clone(), token);
        return Ok(StreamInfo::build(stream_id, node_name));
    }
    let (stream_id, node_name) = start_invite_stream(
        device_id,
        channel_id,
        &token,
        play_type,
        TimeRange::build(0, 0),
    )
    .await?;
    general::cache::Cache::stream_map_insert_token(stream_id.clone(), token);
    Ok(StreamInfo::build(stream_id, node_name))
}

pub async fn play_back(play_back_model: PlayBackModel, token: String) -> GlobalResult<StreamInfo> {
    let device_id = play_back_model.get_device_id();
    if !RWSession::has_session_by_device_id(device_id) {
        return Err(GlobalError::new_biz_error(1000, "设备已离线", |msg| {
            error!("{msg}")
        }));
    }
    let channel_id = if let Some(channel_id) = play_back_model.get_channel_id() {
        channel_id
    } else {
        device_id
    };
    let play_type = PlayType::Back;
    //查看流是否已存在,有则直接返回
    if let Some((stream_id, node_name)) =
        enable_invite_stream(device_id, channel_id, &token, &play_type).await
    {
        general::cache::Cache::stream_map_insert_token(stream_id.clone(), token);
        return Ok(StreamInfo::build(stream_id, node_name));
    }
    let st = play_back_model.get_st();
    let et = play_back_model.get_et();
    let (stream_id, node_name) = start_invite_stream(
        device_id,
        channel_id,
        &token,
        play_type,
        TimeRange::build(*st, *et),
    )
    .await?;
    general::cache::Cache::stream_map_insert_token(stream_id.clone(), token);
    Ok(StreamInfo::build(stream_id, node_name))
}

pub async fn seek(seek_mode: PlaySeekModel, _token: String) -> GlobalResult<bool> {
    let (device_id, channel_id, _ssrc) = id_builder::de_stream_id(seek_mode.get_stream_id());
    let (call_id, seq, from_tag, to_tag) =
        general::cache::Cache::stream_map_build_call_id_seq_from_to_tag(seek_mode.get_stream_id())
            .ok_or_else(|| {
                GlobalError::new_biz_error(1100, "流不存在", |msg| error!("{msg}"))
            })?;
    CmdStream::play_seek(
        &device_id,
        &channel_id,
        *seek_mode.get_seek_second(),
        &from_tag,
        &to_tag,
        seq,
        call_id,
    )
    .await?;
    Ok(true)
}

pub async fn speed(speed_mode: PlaySpeedModel, _token: String) -> GlobalResult<bool> {
    let (device_id, channel_id, _ssrc) = id_builder::de_stream_id(speed_mode.get_stream_id());
    let (call_id, seq, from_tag, to_tag) =
        general::cache::Cache::stream_map_build_call_id_seq_from_to_tag(speed_mode.get_stream_id())
            .ok_or_else(|| {
                GlobalError::new_biz_error(1100, "流不存在", |msg| error!("{msg}"))
            })?;
    CmdStream::play_speed(
        &device_id,
        &channel_id,
        *speed_mode.get_speed_rate(),
        &from_tag,
        &to_tag,
        seq,
        call_id,
    )
    .await?;
    Ok(true)
}

pub async fn ptz(ptz_control_model: PtzControlModel, _token: String) -> GlobalResult<bool> {
    CmdControl::control_ptz(&ptz_control_model).await?;
    let mut model = PtzControlModel::default();
    model.set_device_id(ptz_control_model.get_device_id());
    sleep(Duration::from_millis(1000)).await;
    model.set_channel_id(ptz_control_model.get_channel_id());
    CmdControl::control_ptz(&model).await?;
    Ok(true)
}

//选择流媒体节点（可用+负载最小）-> 监听流注册
//发起实时点播 -> 监听设备响应
//缓存流信息
async fn start_invite_stream(
    device_id: &String,
    channel_id: &String,
    _token: &str,
    play_type: PlayType,
    range: TimeRange,
) -> GlobalResult<(String, String)> {
    let ssrc = general::cache::Cache::ssrc_sn_get().ok_or_else(|| {
        GlobalError::new_biz_error(1100, "ssrc已用完,并发达上限,等待释放", |msg| {
            error!("{msg}")
        })
    })?;
    let mut node_sets = general::cache::Cache::stream_map_order_node();
    let (ssrc, stream_id) =
        id_builder::build_ssrc_stream_id(device_id, channel_id, ssrc, true).await?;
    let conf = general::StreamConf::get_stream_conf();
    //TODO: 选择负载最小的节点开始尝试：节点是否可用;
    if let Some((_, node_name)) = node_sets.pop_first() {
        let stream_node = conf.get_node_map().get(&node_name).unwrap();
        //TODO: 将sdp支持从session固定的，转为stream支持的
        // if let Ok(true) = callback::_call_listen_ssrc(
        //     stream_id.clone(),
        //     &ssrc,
        //     token,
        //     stream_node.get_local_ip(),
        //     stream_node.get_local_port(),
        // )
        // .await
        {
            let (res, _media_map, from_tag, to_tag) = match play_type {
                PlayType::Live => {
                    CmdStream::play_live_invite(
                        device_id,
                        channel_id,
                        MediaAddress::build(
                            stream_node.get_pub_ip().to_string(),
                            *stream_node.get_pub_port(),
                        ),
                        StreamMode::Udp,
                        &ssrc,
                    )
                    .await?
                }
                PlayType::Back => {
                    CmdStream::play_back_invite(
                        device_id,
                        channel_id,
                        MediaAddress::build(
                            stream_node.get_pub_ip().to_string(),
                            *stream_node.get_pub_port(),
                        ),
                        StreamMode::Udp,
                        &ssrc,
                        range,
                    )
                    .await?
                } // PlayType::Down => {}
            };

            //回调给zlm 使其确认媒体类型
            // let _ = callback::_ident_rtp_media_info(
            //     &ssrc,
            //     media_map,
            //     token,
            //     stream_node.get_local_ip(),
            //     stream_node.get_local_port(),
            // )
            // .await;
            let (call_id, seq) = CmdStream::invite_ack(device_id, &res)?;
            return if let Some(_base_stream_info) =
                listen_stream_by_stream_id(&stream_id, RELOAD_EXPIRES).await
            {
                general::cache::Cache::stream_map_insert_info(
                    stream_id.clone(),
                    node_name.clone(),
                    call_id,
                    seq,
                    play_type,
                    from_tag,
                    to_tag,
                );
                general::cache::Cache::device_map_insert(
                    device_id.to_string(),
                    channel_id.to_string(),
                    ssrc,
                    stream_id.clone(),
                    play_type,
                );
                Ok((stream_id, node_name))
            } else {
                CmdStream::play_bye(seq + 1, call_id, device_id, channel_id, &from_tag, &to_tag)
                    .await?;
                Err(GlobalError::new_biz_error(
                    1100,
                    "未接收到监控推流",
                    |msg| error!("{msg}"),
                ))
            };
        }
    }
    Err(GlobalError::new_biz_error(
        1100,
        "无可用流媒体服务",
        |msg| error!("{msg}"),
    ))
}

//首先查看session缓存中是否有映射关系,然后看stream中是否有相应数据:都为true时返回数据
//当session有,stream无时：session调用stream->使其重新监听ssrc
//(避免stream重启后,数据不一致)
async fn enable_invite_stream(
    device_id: &String,
    channel_id: &String,
    token: &str,
    play_type: &PlayType,
) -> Option<(String, String)> {
    match general::cache::Cache::device_map_get_invite_info(device_id, channel_id, play_type) {
        None => None,
        //session -> true
        Some((stream_id, _ssrc)) => {
            let mut res = None;
            if let Some(node_name) = general::cache::Cache::stream_map_query_node_name(&stream_id) {
                //确认stream是否存在
                if let Some(stream_node) = general::StreamConf::get_stream_conf()
                    .get_node_map()
                    .get(&node_name)
                {
                    if let Ok(count) = callback::get_stream_count(
                        Some(&stream_id),
                        token,
                        stream_node.get_local_ip(),
                        stream_node.get_local_port(),
                    )
                    .await
                    {
                        if count == 0 {
                            //session有流信息,stream无流存在=>进一步判断可能是stream重启导致没有该监听,重启监听等待结果
                            // if let Ok(true) = callback::_call_listen_ssrc(
                            //     stream_id.clone(),
                            //     &ssrc,
                            //     token,
                            //     stream_node.get_local_ip(),
                            //     stream_node.get_local_port(),
                            // )
                            // .await
                            {
                                if (listen_stream_by_stream_id(&stream_id, EXPIRES).await).is_some()
                                {
                                    res = Some((stream_id.clone(), node_name));
                                }
                            }
                        } else {
                            //stream -> true
                            res = Some((stream_id.clone(), node_name));
                        }
                    }
                }
            }
            //stream中无stream_id映射,同步剔除session中映射
            if res.is_none() {
                general::cache::Cache::device_map_remove(device_id, None);
                general::cache::Cache::stream_map_remove(&stream_id, None);
            }
            res
        }
    }
}

async fn listen_stream_by_stream_id(stream_id: &String, secs: u64) -> Option<BaseStreamInfo> {
    let (tx, mut rx) = mpsc::channel(8);
    let when = Instant::now() + Duration::from_secs(secs);
    let key = format!("{KEY_STREAM_IN}{stream_id}");
    general::cache::Cache::state_insert(key.clone(), Bytes::new(), Some(when), Some(tx));
    let mut res = None;
    if let Some(Some(bytes)) = rx.recv().await {
        res = serde_json::from_slice::<BaseStreamInfo>(&bytes).ok();
    }
    general::cache::Cache::state_remove(&key);
    res
}

#[cfg(test)]
mod test {
    use common::chrono::Local;
    use common::tokio;
    use common::tokio::sync::mpsc;
    use common::tokio::time::{sleep_until, Instant};
    use std::time::Duration;

    #[tokio::test]
    async fn test() {
        let (tx, mut rx) = mpsc::channel::<Option<u32>>(8);
        let init = Local::now().timestamp_millis();
        println!("first init : {}", init);
        tokio::spawn(async move {
            sleep_until(Instant::now() + Duration::from_secs(2)).await;
            tx.send(None).await.unwrap();
            let current = Local::now().timestamp_millis();
            println!("sub : {}", current - init);
        });
        if let Some(Some(data)) = rx.recv().await {
            println!("res = {}", data);
        }
        let current = Local::now().timestamp_millis();
        println!("main : {}", current - init);
        sleep_until(Instant::now() + Duration::from_secs(6)).await;
    }
}
