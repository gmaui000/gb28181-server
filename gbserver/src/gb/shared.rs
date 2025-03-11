/// 数据读写会话：与网络协议交互
/// UDP：三次心跳超时则移除会话
/// TCP：连接断开或三次心跳超时则移除会话
pub mod rw {
    use crate::gb::handler::events::event::{Container, EventSession, Ident, EXPIRES};
    use crate::store::entity::GbsDevice;
    use common::anyhow::anyhow;
    use common::bytes::Bytes;
    use common::constructor::New;
    use common::exception::GlobalError::SysErr;
    use common::exception::{GlobalResult, TransError};
    use common::log::{error, warn};
    use common::net::state::{Association, Event, Package, Protocol, Zip};
    use common::once_cell::sync::Lazy;
    use common::tokio;
    use common::tokio::sync::mpsc::{Receiver, Sender};
    use common::tokio::sync::{mpsc, Notify};
    use common::tokio::time;
    use common::tokio::time::Instant;
    use parking_lot::Mutex;
    use rsip::{Response, SipMessage};
    use std::collections::{BTreeSet, HashMap};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    static RW_SESSION: Lazy<RWSession> = Lazy::new(RWSession::init);

    pub struct RWSession {
        shared: Arc<Shared>,
        //更新设备状态
        db_task: Sender<String>,
    }

    impl RWSession {
        fn init() -> Self {
            let (tx, rx) = mpsc::channel(16);
            let session = RWSession {
                shared: Arc::new(Shared {
                    state: Mutex::new(State {
                        sessions: HashMap::new(),
                        expirations: BTreeSet::new(),
                        bill_map: HashMap::new(),
                    }),
                    background_task: Notify::new(),
                }),
                db_task: tx.clone(),
            };
            let shared = session.shared.clone();
            thread::Builder::new()
                .name("Shared:rw".to_string())
                .spawn(|| {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_time()
                        .thread_name("RW-SESSION")
                        .build()
                        .hand_log(|msg| error!("{msg}"))
                        .unwrap();
                    rt.block_on(async {
                        let db_task = tokio::spawn(async move {
                            Self::do_update_device_status(rx).await;
                        });
                        let clean_task = tokio::spawn(async move {
                            let _ = Self::purge_expired_task(shared).await;
                        });
                        let _ = db_task.await.hand_log(|msg| error!("Session:{msg}"));
                        let _ = clean_task.await.hand_log(|msg| error!("WEB:{msg}"));
                    });
                })
                .expect("Shared:rw background thread create failed");
            session
        }
        async fn do_update_device_status(mut rx: Receiver<String>) {
            while let Some(device_id) = rx.recv().await {
                let _ = GbsDevice::update_gbs_device_status_by_device_id(&device_id, 0).await;
            }
        }

        async fn purge_expired_task(shared: Arc<Shared>) -> GlobalResult<()> {
            loop {
                if let Some(when) = shared.purge_expired_state().await? {
                    tokio::select! {
                        _ = time::sleep_until(when) =>{},
                        _ = shared.background_task.notified() =>{},
                    }
                } else {
                    shared.background_task.notified().await;
                }
            }
        }

        pub fn insert(device_id: &str, tx: Sender<Zip>, heartbeat: u8, bill: &Association) {
            let expires = Duration::from_secs(heartbeat as u64 * 3);
            let when = Instant::now() + expires;

            let mut state = RW_SESSION.shared.state.lock();

            let notify = state.next_expiration().map(|ts| ts > when).unwrap_or(true);
            state.expirations.insert((when, device_id.to_string()));
            //当插入时，已有该设备映射时，需删除老数据，插入新数据
            if let Some((_tx, when, _expires, old_bill)) = state
                .sessions
                .insert(device_id.to_string(), (tx, when, expires, bill.clone()))
            {
                state.expirations.remove(&(when, device_id.to_string()));
                state.bill_map.remove(&old_bill);
                state.bill_map.insert(bill.clone(), device_id.to_string());
            }
            drop(state);

            if notify {
                RW_SESSION.shared.background_task.notify_one();
            }
        }

        //用于收到网络出口对端连接断开时，清理rw_session数据
        pub fn clean_rw_session_by_bill(bill: &Association) {
            let mut guard = RW_SESSION.shared.state.lock();

            let state = &mut *guard;
            if let Some(device_id) = state.bill_map.remove(bill) {
                if let Some((_tx, when, _expires, _bill)) = state.sessions.remove(&device_id) {
                    state.expirations.remove(&(when, device_id));
                }
            }
        }

        pub fn get_device_id_by_association(bill: &Association) -> Option<String> {
            let guard = RW_SESSION.shared.state.lock();
            guard.bill_map.get(bill).cloned()
        }

        //用于清理rw_session数据及端口TCP网络连接
        //todo 禁用设备时需调用
        pub async fn clean_rw_session_and_net(device_id: &String) {
            let res = {
                let mut guard = RW_SESSION.shared.state.lock();

                let state = &mut *guard;
                if let Some((tx, when, _expires, bill)) = state.sessions.remove(device_id) {
                    state.expirations.remove(&(when, device_id.clone()));
                    state.bill_map.remove(&bill);
                    //通知网络出口关闭TCP连接
                    if &Protocol::TCP == bill.get_protocol() {
                        Some((tx, bill))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((tx, bill)) = res {
                let _ = tx
                    .try_send(Zip::build_event(Event::new(bill, 0)))
                    .hand_log(|msg| warn!("{msg}"));
            }
        }

        pub fn heart(device_id: &String, new_bill: Association) {
            let mut guard = RW_SESSION.shared.state.lock();
            let state = &mut *guard;

            if let Some(mut_value) = state.sessions.get_mut(device_id) {
                let (_tx, when, expires, bill) = mut_value;
                if bill.get_protocol() == &Protocol::UDP {
                    let old_bill = (*bill).clone();
                    state.bill_map.remove(&old_bill);
                    state.bill_map.insert(old_bill, device_id.clone());
                    *bill = new_bill;
                }
                let old_when = *when;
                state.expirations.remove(&(old_when, device_id.clone()));
                let ct = Instant::now() + *expires;
                *when = ct;
                state.expirations.insert((ct, device_id.clone()));
            }
        }

        pub fn get_bill_by_device_id(device_id: &String) -> Option<Association> {
            let guard = RW_SESSION.shared.state.lock();

            let option_bill = guard
                .sessions
                .get(device_id)
                .map(|(_tx, _when, _expires, bill)| bill.clone());

            option_bill
        }

        pub fn get_expires_by_device_id(device_id: &String) -> Option<Duration> {
            let guard = RW_SESSION.shared.state.lock();
            let option_expires = guard
                .sessions
                .get(device_id)
                .map(|(_tx, _when, expires, _bill)| *expires);

            option_expires
        }

        fn get_output_sender_by_device_id(
            device_id: &String,
        ) -> Option<(Sender<Zip>, Association)> {
            let guard = RW_SESSION.shared.state.lock();
            let opt = guard
                .sessions
                .get(device_id)
                .map(|(sender, _, _, bill)| (sender.clone(), bill.clone()));

            opt
        }

        pub fn has_session_by_device_id(device_id: &String) -> bool {
            let guard = RW_SESSION.shared.state.lock();

            guard.sessions.contains_key(device_id)
        }
    }

    #[derive(New)]
    pub struct RequestOutput {
        ident: Ident,
        msg: SipMessage,
        event_sender: Option<Sender<(Option<Response>, Instant)>>,
    }

    impl RequestOutput {
        pub fn do_send_off(device_id: &String, msg: SipMessage) -> GlobalResult<()> {
            let (request_sender, bill) = RWSession::get_output_sender_by_device_id(device_id)
                .ok_or(SysErr(anyhow!("设备 {device_id},已下线")))?;
            let _ = request_sender
                .try_send(Zip::build_data(Package::new(bill, Bytes::from(msg))))
                .hand_log(|msg| warn!("{msg}"));
            Ok(())
        }

        pub fn do_send(self) -> GlobalResult<()> {
            let device_id = self.ident.get_device_id();
            let (request_sender, bill) = RWSession::get_output_sender_by_device_id(device_id)
                .ok_or(SysErr(anyhow!("设备 {device_id},已下线")))?;
            let when = Instant::now() + Duration::from_secs(EXPIRES);
            EventSession::listen_event(&self.ident, when, Container::build_res(self.event_sender))?;
            let _ = request_sender
                .try_send(Zip::build_data(Package::new(bill, Bytes::from(self.msg))))
                .hand_log(|msg| error!("{msg}"));
            Ok(())
        }

        pub fn do_send_outer(self) -> GlobalResult<()> {
            let device_id = self.ident.get_device_id();
            let (request_sender, bill) = RWSession::get_output_sender_by_device_id(device_id)
                .ok_or(SysErr(anyhow!("设备 {device_id},已下线")))?;
            let _ = request_sender
                .try_send(Zip::build_data(Package::new(bill, Bytes::from(self.msg))))
                .hand_log(|msg| error!("{msg}"));
            Ok(())
        }
    }

    struct Shared {
        state: Mutex<State>,
        background_task: Notify,
    }

    impl Shared {
        //清理过期state,并返回下一个过期瞬间刻度
        async fn purge_expired_state(&self) -> GlobalResult<Option<Instant>> {
            let mut guard = RW_SESSION.shared.state.lock();

            let state = &mut *guard;
            let now = Instant::now();
            while let Some((when, device_id)) = state.expirations.iter().next() {
                if when > &now {
                    return Ok(Some(*when));
                }
                //放入队列中处理，避免阻塞导致锁长期占用:更新DB中设备状态为离线
                let _ = RW_SESSION
                    .db_task
                    .clone()
                    .try_send(device_id.clone())
                    .hand_log(|msg| warn!("{msg}"));
                // GbsDevice::update_gbs_device_status_by_device_id(device_id, 0);
                //移除会话map
                if let Some((tx, when, _dur, bill)) = state.sessions.remove(device_id) {
                    state.bill_map.remove(&bill);
                    state.expirations.remove(&(when, device_id.to_string()));
                    //通知网络出口关闭TCP连接
                    if &Protocol::TCP == bill.get_protocol() {
                        let _ = tx
                            .try_send(Zip::build_event(Event::new(bill, 0)))
                            .hand_log(|msg| warn!("{msg}"));
                    }
                }
            }

            Ok(None)
        }
    }

    struct State {
        //映射设备ID，会话发送端，过期瞬时，心跳周期，网络三元组，device_id,msg,dst_addr,time,duration,bill
        sessions: HashMap<String, (Sender<Zip>, Instant, Duration, Association)>,
        //标识设备状态过期时刻，instant,device_id
        expirations: BTreeSet<(Instant, String)>,
        //映射网络三元组与设备ID，bill,device_id
        bill_map: HashMap<Association, String>,
    }

    impl State {
        //获取下一个过期瞬间刻度
        fn next_expiration(&self) -> Option<Instant> {
            self.expirations.first().map(|expiration| expiration.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::Entry;
    use std::collections::{BTreeSet, HashMap};

    #[test]
    fn test_bt_set() {
        let mut set = BTreeSet::new();
        set.insert(2);
        set.insert(1);
        set.insert(6);
        set.insert(3);
        let mut iter = set.iter();
        assert_eq!(Some(&1), iter.next());
        assert_eq!(Some(&2), iter.next());
        assert_eq!(Some(&3), iter.next());
        assert_eq!(Some(&6), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_map_entry() {
        let mut map = HashMap::new();
        map.insert(1, 2);
        map.insert(3, 4);
        map.insert(5, 6);

        match map.entry(3) {
            Entry::Occupied(_) => {
                println!("repeat");
            }
            Entry::Vacant(en) => {
                en.insert(10);
            }
        }
        match map.entry(7) {
            Entry::Occupied(_) => {
                println!("repeat");
            }
            Entry::Vacant(en) => {
                en.insert(8);
            }
        }
        println!("{map:?}");
    }
}
