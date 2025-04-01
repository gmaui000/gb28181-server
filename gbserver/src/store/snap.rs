use common::confgen::conf;
use common::confgen::conf::{CheckFromConf, FieldCheckError};
use common::constructor::{Get, New};
use common::exception::{GlobalError, GlobalResult, TransError};
use common::log::error;
use common::once_cell::sync::Lazy;
use common::serde::Deserialize;
use common::serde_default;
use cron::Schedule;
use crossbeam_channel::Sender;
use image::ImageFormat;
use std::{fs, path::Path, str::FromStr, thread};
use url::Url;

#[derive(Debug, Get, Deserialize)]
#[serde(crate = "common::serde")]
#[conf(prefix = "server.snap", check)]
pub struct Snap {
    #[serde(default = "default_enable")]
    enable: bool,
    push_url: Option<String>,
    #[serde(default = "default_cron_cycle")]
    cron_cycle: String,
    #[serde(default = "default_num")]
    num: u8,
    #[serde(default = "default_interval")]
    interval: u8,
    #[serde(default = "default_storage_path")]
    storage_path: String,
    #[serde(default = "default_storage_format")]
    storage_format: String,
}
serde_default!(default_enable, bool, false);
serde_default!(default_cron_cycle, String, String::from("0 */5 * * * *"));
serde_default!(default_num, u8, 1);
serde_default!(default_interval, u8, 1);
serde_default!(default_storage_path, String, "./snap/raw".to_string());
serde_default!(default_storage_format, String, "jpeg".to_string());

impl CheckFromConf for Snap {
    fn _field_check(&self) -> Result<(), FieldCheckError> {
        let snap: Snap = Snap::conf();
        if snap.enable {
            let uri = self.push_url.as_ref().ok_or(FieldCheckError::BizError(
                "push_url is required".to_string(),
            ))?;
            Url::parse(uri).map_err(|e| {
                FieldCheckError::BizError(format!("Invalid push_url: {}", e.to_string()))
            })?;
        }
        match &*snap.storage_format.to_ascii_lowercase() {
            "avif" | "bmp" | "farbfeld" | "gif" | "hdr" | "ico" | "jpeg" | "exr" | "png"
            | "pnm" | "qoi" | "tga" | "tiff" | "webp" => {}
            _ => {
                return Err(FieldCheckError::BizError("storage_format must be in [avif,bmp,farbfeld,gif,hdr,ico,jpeg,exr,png,pnm,qoi,tga,tiff,webp]".to_string()));
            }
        }
        Schedule::from_str(snap.get_cron_cycle()).map_err(|e| {
            FieldCheckError::BizError(format!("Invalid cron expression: {}", e.to_string()))
        })?;
        fs::create_dir_all(snap.get_storage_path()).map_err(|e| {
            FieldCheckError::BizError(format!("create raw_path dir failed: {}", e.to_string()))
        })?;
        Ok(())
    }
}

impl Snap {
    pub fn get_snap_by_conf() -> &'static Self {
        static INSTANCE: Lazy<Snap> = Lazy::new(|| {
            let snap = Snap::conf();
            let _ = std::fs::create_dir_all(snap.get_storage_path())
                .hand_log(|msg| error!("create raw_path dir failed: {msg}"));
            snap
        });
        &INSTANCE
    }
}

//file_name:data
#[derive(New)]
pub struct ImageInfo {
    image_type: String,
    file_name: String,
    data: Vec<u8>,
}

impl ImageInfo {
    pub fn sender() -> Sender<Self> {
        static SENDER: Lazy<Sender<ImageInfo>> = Lazy::new(|| {
            let (tx, rx) = crossbeam_channel::bounded(1000);
            thread::Builder::new()
                .name("Shared:rw".to_string())
                .spawn(move || {
                    let r = rayon::ThreadPoolBuilder::new()
                        .build()
                        .expect("snap: rayon init failed");
                    r.scope(|s| {
                        s.spawn(move |_| {
                            rx.iter().for_each(|image_info: ImageInfo| {
                                let _ = image_info.hand_snap();
                            })
                        })
                    })
                })
                .expect("Store:snap background thread create failed");
            tx
        });
        SENDER.clone()
    }

    fn hand_snap(self) -> GlobalResult<()> {
        if let Some(ty) = self.image_type.get(6..) {
            if let Some(format) = ImageFormat::from_extension(ty) {
                let l_img = image::load_from_memory_with_format(&self.data, format)
                    .hand_log(|msg| error!("{msg}"))?;
                let storage_path = Snap::get_snap_by_conf().get_storage_path();
                let s_path = Path::new(storage_path).join(format!("s{}.{}", self.file_name, ty));
                let s_img = l_img.thumbnail(240, 240);
                s_img.save(s_path).hand_log(|msg| error!("{msg}"))?;
                return Ok(());
            }
        }
        Err(GlobalError::new_sys_error(
            "File is not a valid image",
            |msg| error!("{msg}"),
        ))
    }
}

// fn print_diff(index: u8, last: i64) -> i64 {
//     let current = Local::now().timestamp_millis();
//     println!("{} : {}", index, current - last);
//     current
// }

#[cfg(test)]
mod test {
    use common::chrono::Local;
    use image::ImageFormat;
    use image::ImageFormat::Jpeg;

    #[test]
    fn test() {
        let content_type = "image/jpeg";
        let format = content_type
            .split_once('/')
            .map(|(_, fmt)| fmt)
            .unwrap_or("");
        println!("格式: {}", format);
        assert_eq!("jpeg", format);
        let option = ImageFormat::from_extension(format);
        println!("{:?}", option);
        assert_eq!(Some(Jpeg), option);
        let date_str = Local::now().format("%Y%m%d").to_string();
        println!("{}", date_str);
    }
}
