use crate::gb::handler::cmd;
use crate::general::schedule;
use crate::general::schedule::ScheduleTask;
use crate::store::mapper;
use crate::store::snap::Snap;
use crate::utils::se_token;
use common::exception::{GlobalResult, TransError};
use common::log::error;
use common::tokio::time::sleep;
use cron::Schedule;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

//启动器完成后触发执行
pub trait Runner {
    fn next() -> impl Future<Output = ()> + Send;
}

pub struct SnapRunner;

impl SnapRunner {
    pub async fn snapshot(&self) -> GlobalResult<()> {
        let snap_conf = Snap::get_snap_by_conf();
        const COUNT: u32 = 50;
        let mut start = 0;
        let mut size = COUNT;
        while size == COUNT {
            let arr = mapper::get_snapshot_dc_by_limit(start, COUNT).await?;
            size = arr.len() as u32;
            start += COUNT;

            for item in arr {
                let (token, session_id) = se_token::build_token_session_id(&item.0, &item.1)?;
                let url = format!(
                    "{}?token={}",
                    snap_conf.get_push_url().clone().unwrap(),
                    token
                );
                cmd::CmdControl::snapshot_image(
                    &item.0,
                    &item.1,
                    *snap_conf.get_num(),
                    *snap_conf.get_interval(),
                    &url,
                    &session_id,
                )
                .await?;
            }
            //图片上传延迟3秒，缓解带宽瓶颈
            sleep(Duration::from_secs(3)).await;
        }

        Ok(())
    }
}

impl ScheduleTask for SnapRunner {
    fn do_something(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let _ = self.snapshot().await;
        })
    }
}

impl Runner for SnapRunner {
    fn next() -> impl Future<Output = ()> + Send {
        async {
            let snap_conf = Snap::get_snap_by_conf();
            if !snap_conf.get_enable() {
                return;
            }
            let cron = snap_conf.get_cron_cycle();
            let schedule = Schedule::from_str(cron).unwrap(); // 服务启动：连接代码段时已检查cron表达式 - 正确
            let tx = schedule::get_schedule_tx();
            let _ = tx
                .send((schedule, Arc::new(SnapRunner)))
                .await
                .hand_log(|msg| error!("{msg}"));
        }
    }
}
