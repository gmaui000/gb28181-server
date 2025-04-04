use crate::store::entity::GbsOauth;
use crate::store::mapper;
use common::exception::{GlobalError, GlobalResult};
use common::log::error;
use std::time::{SystemTime, UNIX_EPOCH};

const D_DIC: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
//按键盘从上至下，从左到右形成小写、大写字母字典表
const A_DIC: [char; 52] = [
    'q', 'a', 'z', 'w', 's', 'x', 'e', 'd', 'c', 'r', 'f', 'v', 't', 'g', 'b', 'y', 'h', 'n', 'u',
    'j', 'm', 'i', 'k', 'o', 'l', 'p', 'Q', 'A', 'Z', 'W', 'S', 'X', 'E', 'D', 'C', 'R', 'F', 'V',
    'T', 'G', 'B', 'Y', 'H', 'N', 'U', 'J', 'M', 'I', 'K', 'O', 'L', 'P',
];
// const E_DIC: [char; 2] = ['_', '.'];

// pub fn en_short_id_from_digit_str(digit_str: &str) -> GlobalResult<String> {
//     let dic = [&E_DIC[..], &D_DIC[..], &A_DIC[..]].concat();
//     let mut tmp_key0 = String::new();
//     for ch in digit_str.chars() {
//         let digit = ch.to_digit(10).ok_or_else(|| GlobalError::new_sys_error(&format!("{ch} not digit"), |msg| error!("{msg}")))?;
//         tmp_key0.push_str(&format!("{:04b}", digit));
//     }
//     let short_id: String = tmp_key0.chars().collect::<Vec<_>>().chunks(6)
//         .map(|cs| dic.get(usize::from_str_radix(&cs.iter().collect::<String>(), 2).unwrap())).flatten().collect();
//     Ok(short_id)
// }
//
// #[test]
// fn t1() {
//     let str = en_short_id_from_digit_str("1238978766700465046540897056104511120658").unwrap();
//     println!("{str}");
// }

//生成stream_id,参数由调用方校验,简单对称加密算法
// device_id 20位十进制纯数字
// channel_id 20位十进制纯数字
// ssrc 10位十进制纯数字
pub fn en_stream_id(device_id: &str, channel_id: &str, ssrc: &str) -> String {
    let ori_key = format!("{device_id}{channel_id}{ssrc}");
    //转换为二进制字符串: 50*4=200位
    let mut tmp_key0 = String::new();
    for ch in ori_key.chars() {
        let digit = ch.to_digit(10).expect("Invalid digit");
        tmp_key0.push_str(&format!("{:04b}", digit));
    }
    //使用纳秒的后两位生成填充字符串,并取7个字符
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let nanos = since_the_epoch.as_nanos();
    let fill_str = format!("{:07b}", nanos % 100);
    let mut fill = fill_str.chars();
    //插入7个数进行填充,200+7=207,便于按9位整除分组
    let mut tmp_key1 = String::new();
    for (i, ch) in tmp_key0.chars().enumerate() {
        tmp_key1.push(ch);
        //跳过第一个23后,每隔23位且小于200,满足有且插入7个数
        if i > 23 && i % 23 == 0 {
            tmp_key1.push(fill.next().unwrap());
        }
    }
    //按每9位为一组进行分组,且每组数字再分为3个子分组,子分组左侧值与右侧值交换位置
    let chunks: Vec<String> = tmp_key1
        .chars()
        .collect::<Vec<_>>()
        .chunks_mut(9)
        .map(|chunk0| {
            chunk0
                .chunks_mut(3)
                .map(|item| {
                    item.swap(0, 2);
                })
                .count();
            chunk0.iter().collect()
        })
        .collect();
    //生成最终的key:当商大于0时,取数字字典进行填充
    let mut dst_key = String::new();
    for chunk in chunks {
        let val = usize::from_str_radix(&chunk, 2).expect("Invalid binary group");
        let circle = val / 52;
        let index = val % 52;
        if circle > 0 {
            dst_key.push(D_DIC[circle - 1]);
        }
        dst_key.push(A_DIC[index]);
    }
    dst_key
}

//返回(device_id,channel_id,ssrc)
pub fn de_stream_id(stream_id: &str) -> (String, String, String) {
    let mut tmp_key0 = String::new();
    let mut pre = 0;
    for ch in stream_id.chars() {
        if let Some(circle) = ch.to_digit(10) {
            pre = (circle + 1) * 52;
        } else {
            let a_index = A_DIC.iter().position(|a| a == &ch).expect("非法字符");
            let digit = pre as usize + a_index;
            tmp_key0.push_str(&format!("{:09b}", digit));
            pre = 0;
        }
    }
    let tmp_key1 = tmp_key0
        .chars()
        .collect::<Vec<_>>()
        .chunks_mut(3)
        .flat_map(|item| {
            item.swap(0, 2);
            item.iter()
        })
        .collect::<String>();
    let mut ti = 23 + 23 + 1;
    let bin_str = tmp_key1
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| {
            if index == ti {
                ti += 23 + 1;
                None
            } else {
                Some(ch)
            }
        })
        .collect::<String>();
    let ori_str: String =
        bin_str
            .chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .fold(String::new(), |mut result, chunk| {
                let num_str = u32::from_str_radix(chunk.iter().collect::<String>().as_str(), 2)
                    .expect("Invalid binary group")
                    .to_string();
                result.push_str(&num_str);
                result
            });
    (
        ori_str[0..20].to_string(),
        ori_str[20..40].to_string(),
        ori_str[40..].to_string(),
    )
}

//为十进制整数字符串,表示SSRC值。格式如下:dddddddddd。其中,第1位为历史或实时
// 媒体流的标识位,0为实时,1为历史;第2位至第6位取20位SIP监控域ID之中的4到8位作为域标
// 识,例如“13010000002000000001”中取数字“10000”;第7位至第10位作为域内媒体流标识,是一个与
// 当前域内产生的媒体流SSRC值后4位不重复的四位十进制整数
// 返回(ssrc,stream_id)
pub async fn build_ssrc_stream_id(
    device_id: &String,
    channel_id: &String,
    ssrc: u16,
    live: bool,
) -> GlobalResult<(String, String)> {
    let gbs_oauth = GbsOauth::read_gbs_oauth_by_device_id(device_id)
        .await?
        .ok_or_else(|| {
            GlobalError::new_biz_error(1100, "设备不存在", |msg| error!("{msg}"))
        })?;
    //直播：需校验摄像头是否在线；回放：录像机在线即可
    let mut front_live_or_back = 1;
    if live {
        let channel_status = mapper::get_device_channel_status(device_id, channel_id)
            .await?
            .ok_or_else(|| {
                GlobalError::new_biz_error(1100, "未知设备", |msg| error!("{msg}"))
            })?;
        match &channel_status.to_ascii_uppercase()[..] {
            "ON" | "ONLINE" | "ONLY" | "" => {}
            _ => {
                return Err(GlobalError::new_biz_error(1000, "设备已离线", |msg| {
                    error!("{msg}")
                }));
            }
        }
        front_live_or_back = 0;
    }
    let domain_id = gbs_oauth.get_domain_id();
    let middle_domain_mark = &domain_id[4..=8];
    let ssrc = format!("{front_live_or_back}{middle_domain_mark}{ssrc:04}");
    let stream_id = en_stream_id(device_id, channel_id, &ssrc);
    Ok((ssrc, stream_id))
}

#[test]
fn test1() {
    let device_id = "34020000001110000001";
    let channel_id = "34020000001320000101";
    let ssrc = "1100000001";
    let stream_id = en_stream_id(device_id, channel_id, ssrc);
    let (d_d_id, d_c_id, d_ssrc) = de_stream_id(&stream_id);
    println!("stream_id = {}", &stream_id);
    assert_eq!(device_id, &d_d_id[..]);
    assert_eq!(channel_id, &d_c_id[..]);
    assert_eq!(ssrc, &d_ssrc[..]);
}

#[test]
fn test_ssrc_to_ssrc_num() {
    let ssrc1: u32 = 1100009001;
    let ssrc_num1 = (ssrc1 % 10000) as u16;
    assert_eq!(ssrc_num1, 9001);
    let ssrc2: u32 = 1100000001;
    let ssrc_num2 = (ssrc2 % 10000) as u16;
    assert_eq!(ssrc_num2, 1);
    let ssrc3: u32 = 1100000801;
    let ssrc_num3 = (ssrc3 % 10000) as u16;
    assert_eq!(ssrc_num3, 801);
    let ssrc4: u32 = 1100019999;
    let ssrc_num4 = (ssrc4 % 10000) as u16;
    assert_eq!(ssrc_num4, 9999)
}

#[test]
fn test_de_stream_id() {
    let stream_id = "4FEqqzfqqa0Vzqq3K2lsqc4ZqqH5Z";
    let (device_id, channel_id, ssrc) = de_stream_id(stream_id);
    println!(
        "device_id = {},channel_id = {},ssrc = {}",
        &device_id, &channel_id, &ssrc,
    );
}
