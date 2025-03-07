/// 会话事件：与业务事件交互
/// 定位：请求 <——> 回复
pub mod event {
    use std::collections::hash_map::Entry;
    use std::collections::{BTreeSet, HashMap};
    use std::sync::Arc;
    use std::thread;

    use parking_lot::Mutex;
    use rsip::{Response, SipMessage};

    use common::anyhow::anyhow;
    use common::constructor::{Get, New};
    use common::exception::GlobalError::SysErr;
    use common::exception::{GlobalResult, TransError};
    use common::log::{error, warn};
    use common::once_cell::sync::Lazy;
    use common::tokio;
    use common::tokio::sync::mpsc::Sender;
    use common::tokio::sync::Notify;
    use common::tokio::time;
    use common::tokio::time::Instant;

    use crate::gb::shared::rw::RequestOutput;

    /// 会话超时 8s
    pub const EXPIRES: u64 = 8;
    static EVENT_SESSION: Lazy<EventSession> = Lazy::new(EventSession::init);

    pub struct EventSession {
        shared: Arc<Shared>,
    }

    impl EventSession {
        fn init() -> Self {
            let session = EventSession {
                shared: Arc::new(Shared {
                    state: Mutex::new(State {
                        expirations: BTreeSet::new(),
                        ident_map: HashMap::new(),
                        device_session: HashMap::new(),
                    }),
                    background_task: Notify::new(),
                }),
            };
            let shared = session.shared.clone();
            thread::Builder::new()
                .name("Shared:rw".to_string())
                .spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_time()
                        .thread_name("EVENT-SESSION")
                        .build()
                        .hand_log(|msg| error!("{msg}"))
                        .unwrap();
                    let _ = rt.block_on(Self::purge_expired_task(shared));
                })
                .expect("Shared:event background thread create failed");
            session
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

        //即时事件监听，延迟事件监听
        pub(crate) fn listen_event(
            ident: &Ident,
            when: Instant,
            container: Container,
        ) -> GlobalResult<()> {
            let mut guard = EVENT_SESSION.shared.state.lock();

            let state = &mut *guard;
            match state.device_session.entry(ident.call_id.clone()) {
                Entry::Occupied(_o) => {
                    Err(SysErr(anyhow!("new = {:?},事件重复-添加监听无效", ident)))
                }
                Entry::Vacant(en) => {
                    en.insert(ident.device_id.clone());
                    state.expirations.insert((when, ident.clone()));
                    state.ident_map.insert(ident.clone(), (when, container));

                    Ok(())
                }
            }
        }

        pub fn remove_event(ident: &Ident) {
            let mut guard = EVENT_SESSION.shared.state.lock();

            let state = &mut *guard;
            state.ident_map.remove(ident).map(|(when, _container)| {
                state.expirations.remove(&(when, ident.clone()));

                state.device_session.remove(ident.get_call_id())
            });
        }

        pub async fn handle_response(
            to_device_id: String,
            call_id: String,
            cs_eq: String,
            response: Response,
        ) -> GlobalResult<()> {
            let res = {
                let mut guard = EVENT_SESSION.shared.state.lock();

                let state = &mut *guard;
                match state.device_session.get(&call_id) {
                    None => {
                        warn!("丢弃：超时或未知响应。device_id={to_device_id},call_id={call_id},cs_eq={cs_eq}");
                        None
                    }
                    Some(device_id) => {
                        let ident: Ident = Ident::new(device_id.clone(), call_id, cs_eq);
                        //用于一次请求有多次响应：如点播时，有100-trying，再200-OK两次响应
                        //接收端确认无后继响应时，需调用remove_event()，清理会话
                        match state.ident_map.get(&ident) {
                            None => {
                                warn!("{:?},超时或未知响应", &ident);
                                None
                            }
                            Some((when, container)) => {
                                match container {
                                    Container::Res(res) => {
                                        //当tx为some时发送响应结果，不清理会话，由相应rx接收端根据自身业务清理
                                        if let Some(tx) = res {
                                            let when = *when;
                                            let sender = tx.clone();
                                            Some((sender, when))
                                            // drop(guard);
                                            // let _ = sender.send((Some(response), when)).await.hand_log(|msg| error!("{msg}"));
                                        } else {
                                            //清理会话
                                            state.ident_map.remove(&ident).map(
                                                |(when, _container)| {
                                                    state
                                                        .expirations
                                                        .remove(&(when, ident.clone()));
                                                    state.device_session.remove(ident.get_call_id())
                                                },
                                            );
                                            None
                                        }
                                    }
                                    Container::Actor(..) => {
                                        Err(SysErr(anyhow!("{:?},无效事件", &ident)))?;
                                        None
                                    }
                                }
                            }
                        }
                    }
                }
            };

            if let Some((tx, when)) = res {
                let _ = tx
                    .try_send((Some(response), when))
                    .hand_log(|msg| error!("{msg}"));
            }
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
            let mut guard = EVENT_SESSION.shared.state.lock();

            let state = &mut *guard;
            let now = Instant::now();
            while let Some((when, expire_ident)) = state.expirations.iter().next() {
                if when > &now {
                    return Ok(Some(*when));
                }
                if let Some((ident, (when, container))) = state.ident_map.remove_entry(expire_ident)
                {
                    state.expirations.remove(&(when, expire_ident.clone()));
                    state.device_session.remove(ident.get_call_id());
                    match container {
                        Container::Res(res) => {
                            warn!("{:?},响应超时。", &ident);
                            //响应超时->发送None->接收端收到None,不需要再清理State
                            if let Some(tx) = res {
                                let _ = tx.try_send((None, when)).hand_log(|msg| error!("{msg}"));
                            }
                        }
                        //延迟事件触发后，添加延迟事件执行后的监听
                        Container::Actor(ActorData {
                            ident: inner_ident,
                            msg,
                            sender,
                        }) => {
                            match state.device_session.entry(inner_ident.call_id.clone()) {
                                Entry::Occupied(_o) => {
                                    Err(SysErr(anyhow!("{:?},事件重复监听", inner_ident)))?
                                }
                                //插入事件监听
                                Entry::Vacant(en) => {
                                    en.insert(inner_ident.device_id.clone());
                                    let expires = time::Duration::from_secs(EXPIRES);
                                    let new_when = Instant::now() + expires;
                                    state.expirations.insert((new_when, inner_ident.clone()));
                                    state.ident_map.insert(
                                        inner_ident,
                                        (new_when, Container::build_res(sender)),
                                    );
                                }
                            }
                            RequestOutput::new(ident, msg, None).do_send_outer()?
                        }
                    }
                }
            }

            Ok(None)
        }
    }

    #[derive(New, Get, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct Ident {
        device_id: String,
        call_id: String,
        cs_eq: String,
    }

    // 封装 Actor 变体的数据
    #[derive(Debug)]
    pub struct ActorData {
        ident: Ident,
        msg: SipMessage,
        sender: Option<Sender<(Option<Response>, Instant)>>,
    }

    //Res : 请求响应，当需要做后继处理时，Sender不为None,接收端收到数据时如果后继不再接收数据，需调用清理state
    //Actor : 延时之后所做操作,对设备Bill发送请求数据
    #[derive(Debug)]
    pub enum Container {
        //Option<Response> 可能无响应
        //实时事件
        Res(Option<Sender<(Option<Response>, Instant)>>),
        //延时事件,执行时，会加入实时事件
        Actor(ActorData),
    }

    impl Container {
        pub fn build_res(res: Option<Sender<(Option<Response>, Instant)>>) -> Self {
            Container::Res(res)
        }

        pub fn build_actor(
            ident: Ident,
            msg: SipMessage,
            sender: Option<Sender<(Option<Response>, Instant)>>,
        ) -> Self {
            Container::Actor(ActorData { ident, msg, sender })
        }
    }

    struct State {
        expirations: BTreeSet<(Instant, Ident)>,
        ident_map: HashMap<Ident, (Instant, Container)>,
        //call_id:device_id
        device_session: HashMap<String, String>,
    }
}
