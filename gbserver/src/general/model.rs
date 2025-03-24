use crate::gb::handler::parser::xml::KV2Model;
use crate::general;
use common::anyhow::anyhow;
use common::constructor::{Get, New, Set};
use common::exception::GlobalError::SysErr;
use common::exception::{GlobalResult, TransError};
use common::log::error;
use common::serde::{Deserialize, Serialize};
use poem_openapi::types::{ParseFromJSON, ToJSON, Type};
use poem_openapi::{self, Object};

#[derive(Debug, Get)]
pub struct MediaAddress {
    ip: String,
    port: u16,
}

impl MediaAddress {
    pub fn build(ip: String, port: u16) -> Self {
        Self { ip, port }
    }
}

#[derive(Debug, Get)]
pub struct TimeRange {
    start_time: u32,
    end_time: u32,
}

impl TimeRange {
    pub fn build(start_time: u32, end_time: u32) -> Self {
        Self {
            start_time,
            end_time,
        }
    }
}

pub enum StreamMode {
    Udp,
    TcpActive,
    TcpPassive,
}

impl StreamMode {
    pub fn build(m: u8) -> GlobalResult<Self> {
        match m {
            0 => Ok(StreamMode::Udp),
            1 => Ok(StreamMode::TcpActive),
            2 => Ok(StreamMode::TcpPassive),
            _ => Err(SysErr(anyhow!("无效流模式"))),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Object)]
pub struct ResultMessageData<T: Type + ParseFromJSON + ToJSON> {
    code: u16,
    msg: Option<String>,
    data: Option<T>,
}

impl<T: Type + ParseFromJSON + ToJSON> ResultMessageData<T> {
    #[allow(dead_code)]
    pub fn build(code: u16, msg: String, data: T) -> Self {
        Self {
            code,
            msg: Some(msg),
            data: Some(data),
        }
    }

    pub fn build_success(data: T) -> Self {
        Self {
            code: 200,
            msg: Some("success".to_string()),
            data: Some(data),
        }
    }
    pub fn build_success_none() -> Self {
        Self {
            code: 200,
            msg: Some("success".to_string()),
            data: None,
        }
    }
    pub fn build_failure() -> Self {
        Self {
            code: 500,
            msg: Some("failure".to_string()),
            data: None,
        }
    }
    pub fn build_failure_msg(msg: String) -> Self {
        Self {
            code: 500,
            msg: Some(msg),
            data: None,
        }
    }
}

#[derive(Debug, Deserialize, Object, Serialize, Default, Get, New)]
#[serde(crate = "common::serde")]
pub struct PlayLiveModel {
    #[oai(validator(min_length = "20", max_length = "20"))]
    device_id: String,
    #[oai(validator(min_length = "20", max_length = "20"))]
    channel_id: Option<String>,
    #[oai(validator(maximum(value = "2"), minimum(value = "0")))]
    trans_mode: Option<u8>,
}

#[derive(Debug, Deserialize, Object, Serialize, Get)]
#[serde(crate = "common::serde")]
pub struct PlayBackModel {
    #[oai(validator(min_length = "20", max_length = "20"))]
    device_id: String,
    #[oai(validator(min_length = "20", max_length = "20"))]
    channel_id: Option<String>,
    #[oai(validator(maximum(value = "2"), minimum(value = "0")))]
    trans_mode: Option<u8>,
    st: u32,
    et: u32,
}

#[derive(Debug, Deserialize, Object, Serialize, Get)]
#[serde(crate = "common::serde")]
#[allow(non_snake_case)]
pub struct PlaySeekModel {
    #[oai(validator(min_length = "24", max_length = "32"))]
    stream_id: String,
    #[oai(validator(maximum(value = "86400"), minimum(value = "1")))]
    seek_second: u32,
}

#[derive(Debug, Deserialize, Object, Serialize, Get)]
#[serde(crate = "common::serde")]
#[allow(non_snake_case)]
pub struct PlaySpeedModel {
    #[oai(validator(min_length = "24", max_length = "32"))]
    stream_id: String,
    #[oai(validator(maximum(value = "8"), minimum(value = "0.25")))]
    speed_rate: f32,
}

#[derive(Object, Debug, Deserialize, Serialize, Default, Set, Get)]
#[serde(crate = "common::serde")]
#[allow(non_snake_case)]
pub struct PtzControlModel {
    #[oai(validator(min_length = "20", max_length = "20"))]
    device_id: String,
    #[oai(validator(min_length = "20", max_length = "20"))]
    channel_id: String,
    #[oai(validator(maximum(value = "2"), minimum(value = "0")))]
    ///镜头左移右移 0:停止 1:左移 2:右移
    left_right: u8,
    #[oai(validator(maximum(value = "2"), minimum(value = "0")))]
    ///镜头上移下移 0:停止 1:上移 2:下移
    up_down: u8,
    #[oai(validator(maximum(value = "2"), minimum(value = "0")))]
    ///镜头放大缩小 0:停止 1:缩小 2:放大
    in_out: u8,
    #[oai(validator(maximum(value = "255"), minimum(value = "0")))]
    ///水平移动速度：1-255
    horizon_speed: u8,
    #[oai(validator(maximum(value = "255"), minimum(value = "0")))]
    ///垂直移动速度：0-255
    vertical_speed: u8,
    #[oai(validator(maximum(value = "15"), minimum(value = "0")))]
    ///焦距缩放速度：0-15
    zoom_speed: u8,
}

#[derive(Debug, Deserialize, Object, Serialize)]
#[serde(crate = "common::serde")]
#[allow(non_snake_case)]
pub struct StreamInfo {
    streamId: String,
    flv: String,
    m3u8: String,
}

impl StreamInfo {
    pub fn build(stream_id: String, node_name: String) -> Self {
        let stream_conf = general::StreamConf::get_stream_conf();
        match stream_conf.get_proxy_addr() {
            None => {
                let node_stream = stream_conf.get_node_map().get(&node_name).unwrap();
                Self {
                    flv: format!(
                        "http://{}:{}/{node_name}/play/{stream_id}.flv",
                        node_stream.get_pub_ip(),
                        node_stream.get_local_port()
                    ),
                    m3u8: format!(
                        "http://{}:{}/{node_name}/play/{stream_id}.m3u8",
                        node_stream.get_pub_ip(),
                        node_stream.get_local_port()
                    ),
                    streamId: stream_id,
                }
            }
            Some(addr) => Self {
                flv: format!("{addr}/{node_name}/play/{stream_id}.flv"),
                m3u8: format!("{addr}/{node_name}/play/{stream_id}.m3u8"),
                streamId: stream_id,
            },
        }
    }
}

#[derive(Debug, Deserialize, Object, Serialize, Default)]
#[serde(crate = "common::serde")]
#[allow(non_snake_case)]
pub struct AlarmInfo {
    pub priority: u8,
    pub method: u8,
    pub alarmType: u8,
    pub timeStr: String,
    pub deviceId: String,
    pub channelId: String,
}

impl KV2Model for AlarmInfo {
    fn kv_to_model(arr: Vec<(String, String)>) -> GlobalResult<Self> {
        use crate::gb::handler::parser::xml::*;
        let mut model = AlarmInfo::default();
        for (k, v) in arr {
            match &k[..] {
                NOTIFY_DEVICE_ID => {
                    model.channelId = v;
                }
                NOTIFY_ALARM_PRIORITY => {
                    model.priority = v.parse::<u8>().hand_log(|msg| error!("{msg}"))?;
                }
                NOTIFY_ALARM_TIME => {
                    model.timeStr = v;
                }
                NOTIFY_ALARM_METHOD => {
                    model.method = v.parse::<u8>().hand_log(|msg| error!("{msg}"))?;
                }
                NOTIFY_INFO_ALARM_TYPE => {
                    model.alarmType = v.parse::<u8>().hand_log(|msg| error!("{msg}"))?;
                }
                &_ => {}
            }
        }
        Ok(model)
    }
}

#[cfg(test)]
mod test {
    use poem_openapi::payload::Json;
    use poem_openapi::types::ToJSON;

    use crate::general::model::{ResultMessageData, StreamInfo};

    #[test]
    fn t1() {
        let m = StreamInfo {
            streamId: "streamId".to_string(),
            flv: "streamId".to_string(),
            m3u8: "streamId".to_string(),
        };
        let data = ResultMessageData::build_success(m);
        println!("{:#?}", Json(data).to_json_string());
    }
}
