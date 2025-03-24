use common::constructor::{Get, New};
use common::serde::{Deserialize, Serialize};
use poem_openapi::{
    types::{ParseFromJSON, ToJSON, Type},
    Enum, Object,
};
pub mod callback;
pub mod control;
pub mod handler;
pub const EXPIRES: u64 = 8;
pub const RELOAD_EXPIRES: u64 = 2;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Enum)]
#[serde(crate = "common::serde")]
pub enum PlayType {
    Flv,
    Hls,
}

#[derive(Serialize, Deserialize, Debug, Object)]
#[serde(crate = "common::serde")]
pub struct ResMsg<T: Serialize + Sync + Send + Type + ToJSON + ParseFromJSON> {
    code: u16,
    msg: String,
    data: Option<T>,
}

#[derive(New, Serialize, Object, Deserialize, Get, Debug)]
#[serde(crate = "common::serde")]
pub struct StreamState {
    base_stream_info: BaseStreamInfo,
    user_count: u32,
    // record_name: Option<String>,
}

#[derive(Object, Serialize, Deserialize, Default, Get, Debug, Clone)]
#[serde(crate = "common::serde")]
pub struct PublishRequest {
    app: String,
    id: String,
    ip: String,
    params: String,
    port: i32,
    schema: String,
    protocol: String,
    stream: String,
    vhost: String,
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    media_server_id: String,
}

//推流鉴权事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnPublishResponse {
    code: i32,
    msg: String,
    enable_hls: bool,
    enable_hls_fmp4: bool,
    enable_mp4: bool,
    enable_rtsp: bool,
    enable_rtmp: bool,
    enable_ts: bool,
    enable_fmp4: bool,
    hls_demand: bool,
    rtsp_demand: bool,
    rtmp_demand: bool,
    ts_demand: bool,
    fmp4_demand: bool,
    enable_audio: bool,
    add_mute_audio: bool,
    mp4_save_path: String,
    mp4_save_second: i32,
    mp4_as_player: bool,
    hls_save_path: String,
    modify_stamp: bool,
    continue_push_ms: i32,
    auto_close: bool,
    stream_replace: String,
}

//播放鉴权事件请求消息
#[derive(Object, Deserialize, Serialize, Default, Get, Debug, Clone)]
#[serde(crate = "common::serde")]
pub struct PlayRequest {
    app: String,
    id: String,
    ip: String,
    params: String,
    port: i32,
    schema: String,
    protocol: String,
    stream: String,
    vhost: String,
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    media_server_id: String,
}

//播放鉴权事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnPlayResponse {
    code: i32,
    msg: String,
}

//流观看者人数变化事件请求消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct PlayerCountChangeRequest {
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    pub media_server_id: String,
    pub app: String,
    pub stream: String,
    pub vhost: String,
}

//流观看者人数变化事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnPlayerCountChangeResponse {
    pub code: i32,
    pub msg: String,
}

//流注册/注销事件请求消息
#[derive(Object, Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "common::serde")]
pub struct StreamChangedRequest {
    pub regist: bool,
    #[serde(rename = "aliveSecond")]
    #[oai(rename = "aliveSecond")]
    pub alive_second: Option<u64>,
    pub app: String,
    #[serde(rename = "bytesSpeed")]
    #[oai(rename = "bytesSpeed")]
    pub bytes_speed: Option<i32>,
    #[serde(rename = "createStamp")]
    #[oai(rename = "createStamp")]
    pub create_stamp: Option<u64>,
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    pub media_server_id: String,
    #[serde(rename = "originType")]
    #[oai(rename = "originType")]
    pub origin_type: Option<i32>,
    #[serde(rename = "originUrl")]
    #[oai(rename = "originUrl")]
    pub origin_url: Option<String>,
    #[serde(rename = "readerCount")]
    #[oai(rename = "readerCount")]
    pub reader_count: Option<i32>,
    pub schema: String,
    pub stream: String,
    #[serde(rename = "totalReaderCount")]
    #[oai(rename = "totalReaderCount")]
    pub total_reader_count: Option<i32>,
    pub tracks: Option<Vec<Track>>,
    pub vhost: String,
    pub params: String,
}

//音视频轨道
#[derive(Object, Deserialize, Serialize, Debug, Clone)]
#[serde(crate = "common::serde")]
pub struct Track {
    pub ready: bool,
    pub codec_type: i32,
    pub codec_id_name: String,
    pub codec_id: i32, //Video = 0, Audio = 1
    //视频参数
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fps: Option<i32>,
    //音频参数
    pub channels: Option<i32>,
    pub sample_rate: Option<i32>,
    pub sample_bit: Option<i32>,
}

//流注册/注销事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnStreamChangedResponse {
    code: i32,
    msg: String,
}

//流无人观看事件请求消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct StreamNoneReaderRequest {
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    media_server_id: String,
    app: String,
    schema: String,
    stream: String,
    vhost: String,
}

//流无人观看事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnStreamNoneReaderResponse {
    code: i32,
    close: bool,
}

//流未找到事件请求消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct StreamNotFoundRequest {
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    media_server_id: String,
    app: String,
    id: String,
    ip: String,
    params: String,
    port: u16,
    schema: String,
    protocol: String,
    stream: String,
    vhost: String,
}

//流未找到事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnStreamNotFoundResponse {
    code: i32,
    msg: String,
}

//RTP Server数据接收超时事件请求消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct RtpServerTimeoutRequest {
    local_port: u16,
    re_use_port: bool,
    ssrc: u32,
    stream_id: String,
    tcp_mode: i32,
    #[serde(rename = "mediaServerId")]
    #[oai(rename = "mediaServerId")]
    media_server_id: String,
}

//RTP Server数据接收超时事件响应消息
#[derive(Object, Deserialize, Serialize, Debug, Default, Clone)]
#[serde(crate = "common::serde")]
pub struct OnRtpServerTimeoutResponse {
    code: i32,
    msg: String,
}

#[derive(New, Serialize, Object, Deserialize, Get, Debug)]
#[serde(crate = "common::serde")]
pub struct BaseStreamInfo {
    rtp_info: RtpInfo,
    stream_id: String,
    in_time: u32,
}

#[derive(New, Serialize, Get, Deserialize, Object, Debug)]
#[serde(crate = "common::serde")]
pub struct NetSource {
    remote_addr: String,
    protocol: String,
}

#[derive(New, Object, Serialize, Deserialize, Get, Debug)]
#[serde(crate = "common::serde")]
pub struct RtpInfo {
    ssrc: u32,
    //媒体流源地址,tcp/udp
    origin_trans: Option<NetSource>,
    // //tcp/udp
    // protocol: Option<String>,
    // //媒体流源地址
    // origin_addr: Option<String>,
    server_name: String,
}

#[derive(New, Object, Serialize, Deserialize, Get, Debug)]
#[serde(crate = "common::serde")]
pub struct StreamPlayInfo {
    base_stream_info: BaseStreamInfo,
    remote_addr: String,
    token: String,
    play_type: PlayType,
    //当前观看人数
    user_count: u32,
}

#[derive(New, Object, Serialize, Deserialize, Get, Debug)]
#[serde(crate = "common::serde")]
pub struct StreamRecordInfo {
    base_stream_info: BaseStreamInfo,
    file_path: String,
    file_name: String,
    //单位kb
    file_size: u32,
}
