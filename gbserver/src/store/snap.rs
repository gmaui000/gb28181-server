use common::confgen::conf;
use common::constructor::{Get, New};
use common::exception::{GlobalError, GlobalResult, TransError};
use common::log::error;
use common::once_cell::sync::Lazy;
use common::serde::Deserialize;
use common::serde_default;
use crossbeam_channel::Sender;
use image::ImageFormat;
use std::path::Path;
use std::thread;

#[derive(Debug, Get, Deserialize)]
#[serde(crate = "common::serde")]
#[conf(prefix = "server.snap")]
pub struct Snap {
    #[serde(default = "default_enable")]
    enable: bool,
    #[serde(default = "default_cycle")]
    cycle: u16,
    #[serde(default = "default_num")]
    num: u8,
    #[serde(default = "default_interval")]
    interval: u8,
    #[serde(default = "default_raw_path")]
    raw_path: String,
    #[serde(default = "default_snapshot_path")]
    snapshot_path: String,
}
serde_default!(default_enable, bool, true);
serde_default!(default_cycle, u16, 300);
serde_default!(default_num, u8, 1);
serde_default!(default_interval, u8, 1);
serde_default!(default_raw_path, String, "./snap/raw".to_string());
serde_default!(
    default_snapshot_path,
    String,
    "./snap/snapshot_path".to_string()
);

impl Snap {
    pub fn get_snap_by_conf() -> &'static Self {
        static INSTANCE: Lazy<Snap> = Lazy::new(|| {
            let snap = Snap::conf();
            let _ = std::fs::create_dir_all(&snap.raw_path)
                .hand_log(|msg| error!("create raw_path dir failed: {msg}"));
            let _ = std::fs::create_dir_all(&snap.snapshot_path)
                .hand_log(|msg| error!("create snapshot_path dir failed: {msg}"));
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
                let small_path = Snap::get_snap_by_conf().get_snapshot_path();
                let large_path = Snap::get_snap_by_conf().get_raw_path();
                let s_path = Path::new(small_path).join(format!("s{}.{}", self.file_name, ty));
                let l_path = Path::new(large_path).join(format!("l{}.{}", self.file_name, ty));
                let s_img = l_img.thumbnail(240, 240);
                l_img.save(l_path).hand_log(|msg| error!("{msg}"))?;
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

#[test]
fn test() {
    let s = "image/jpeg".to_string();
    println!("{:?}", s.get(6..));
}
