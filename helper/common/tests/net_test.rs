#[allow(dead_code, unused_imports)]
#[cfg(feature = "net")]
mod test {
    use bytes::Bytes;
    use common::net;
    use common::net::state::Zip;
    use exception::TransError;
    use log::error;
    use std::net::SocketAddr;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_single_udp() {
        use std::collections::HashSet;
        use std::sync::Arc;
        use tokio::net::UdpSocket;
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let client_addr = "0.0.0.0:0";
        let server_addr = "0.0.0.0:18888";

        // 记录接收到的数据
        let received_messages = Arc::new(Mutex::new(HashSet::new()));

        // 初始化 UDP 网络
        let (tx, mut rx) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(server_addr).unwrap(),
        )
        .await
        .unwrap();

        // 启动一个任务，模拟外部客户端发送 UDP 数据
        let client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let socket = UdpSocket::bind(client_addr).await.unwrap(); // 绑定随机端口
                let target_addr = server_addr.parse::<SocketAddr>().unwrap();

                let mut expected_responses = HashSet::new();

                for i in 1..=5 {
                    let msg = format!("hello {}", i);
                    socket.send_to(msg.as_bytes(), &target_addr).await.unwrap();

                    // 记录客户端期望收到的 ACK 消息
                    expected_responses.insert(format!("ack - hello {}", i));

                    let mut buf = [0u8; 1024];
                    let (len, _) = socket.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();

                    // 记录收到的服务器响应
                    received_messages.lock().await.insert(response);
                }

                // 发送 `exit` 指令，通知服务器关闭
                socket.send_to(b"exit", &target_addr).await.unwrap();

                let mut buf = [0u8; 1024];
                let (len, _) = socket.recv_from(&mut buf).await.unwrap();
                let exit_response = String::from_utf8_lossy(&buf[..len]).to_string();
                received_messages.lock().await.insert(exit_response);

                // 确保服务器返回的响应符合预期
                let received = received_messages.lock().await.clone();
                assert!(received.contains("ack - exit"));
                expected_responses.insert("ack - exit".to_string());
                assert_eq!(received, expected_responses);
                println!("received: {:?}", received);
            })
        };

        // 监听 UDP 数据
        while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
            match zip {
                Zip::Data(mut package) => {
                    let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                    // 处理 exit 指令，主动退出
                    if received_data.trim() == "exit" {
                        println!("[Server] Received exit signal, shutting down...");
                        package.set_data(Bytes::from("ack - exit"));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                        break;
                    }

                    // 回复客户端
                    package.set_data(Bytes::from(format!("ack - {}", received_data)));
                    let _ = tx
                        .clone()
                        .send(Zip::build_data(package))
                        .await
                        .hand_log(|msg| error!("{msg}"));
                }
                Zip::Event(_) => {}
            }
        }

        client_task.await.unwrap(); // 确保模拟客户端完成
        println!("[Server] Test completed.");
    }

    #[tokio::test]
    async fn test_single_tcp() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use exception::TransError;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let server_addr = "0.0.0.0:18889";

        // 记录接收到的数据
        let received_messages = Arc::new(Mutex::new(HashSet::new()));

        // 初始化 TCP 网络
        let (tx, mut rx) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(server_addr).unwrap(),
        )
        .await
        .unwrap();

        // 启动一个任务，模拟外部客户端发送 TCP 数据
        let client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let mut stream = TcpStream::connect(server_addr).await.unwrap();
                let mut buffer = [0u8; 1024];

                let mut expected_responses = HashSet::new();

                for i in 1..=5 {
                    let msg = format!("hello {}", i);
                    stream.write_all(msg.as_bytes()).await.unwrap();

                    expected_responses.insert(format!("ack - hello {}", i));

                    let n = stream.read(&mut buffer).await.unwrap();
                    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

                    received_messages.lock().await.insert(response);
                }

                // 发送 `exit` 指令，通知服务器关闭
                stream.write_all(b"exit").await.unwrap();

                let n = stream.read(&mut buffer).await.unwrap();
                let exit_response = String::from_utf8_lossy(&buffer[..n]).to_string();
                received_messages.lock().await.insert(exit_response);

                // 确保服务器返回的响应符合预期
                let received = received_messages.lock().await.clone();
                assert!(received.contains("ack - exit"));
                expected_responses.insert("ack - exit".to_string());
                assert_eq!(received, expected_responses);
                println!("received: {:?}", received);
            })
        };

        // 监听 TCP 数据
        while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
            match zip {
                Zip::Data(mut package) => {
                    let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                    // 处理 exit 指令，主动退出
                    if received_data.trim() == "exit" {
                        println!("[Server] Received exit signal, shutting down...");
                        package.set_data(Bytes::from("ack - exit"));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                        break;
                    }

                    // 回复客户端
                    package.set_data(Bytes::from(format!("ack - {}", received_data)));
                    let _ = tx
                        .clone()
                        .send(Zip::build_data(package))
                        .await
                        .hand_log(|msg| error!("{msg}"));
                }
                Zip::Event(_) => {}
            }
        }

        client_task.await.unwrap(); // 确保模拟客户端完成
        println!("[Server] Test completed.");
    }

    #[tokio::test]
    async fn test_single_all() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use exception::TransError;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpStream, UdpSocket};
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let client_addr = "0.0.0.0:0";
        let server_addr = "0.0.0.0:18890";

        // 记录接收到的数据
        let received_messages = Arc::new(Mutex::new(HashSet::new()));

        // 初始化 ALL 模式（支持 TCP & UDP）
        let (tx, mut rx) = net::init_net(
            net::state::Protocol::ALL,
            SocketAddr::from_str(server_addr).unwrap(),
        )
        .await
        .unwrap();

        // TCP 客户端任务
        let tcp_client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let mut stream = TcpStream::connect(server_addr).await.unwrap();
                let mut buffer = [0u8; 1024];

                let mut expected_responses = HashSet::new();

                for i in 1..=3 {
                    let msg = format!("tcp {}", i);
                    stream.write_all(msg.as_bytes()).await.unwrap();

                    expected_responses.insert(format!("ack - tcp {}", i));

                    let n = stream.read(&mut buffer).await.unwrap();
                    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

                    received_messages.lock().await.insert(response);
                }

                // 发送 `exit` 指令，通知服务器关闭
                stream.write_all(b"exit").await.unwrap();

                let n = stream.read(&mut buffer).await.unwrap();
                let exit_response = String::from_utf8_lossy(&buffer[..n]).to_string();
                received_messages.lock().await.insert(exit_response);
            })
        };

        // UDP 客户端任务
        let udp_client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let socket = UdpSocket::bind(client_addr).await.unwrap();
                let target_addr = server_addr.parse::<SocketAddr>().unwrap();

                let mut expected_responses = HashSet::new();

                for i in 1..=3 {
                    let msg = format!("udp {}", i);
                    socket.send_to(msg.as_bytes(), &target_addr).await.unwrap();

                    expected_responses.insert(format!("ack - udp {}", i));

                    let mut buf = [0u8; 1024];
                    let (len, _) = socket.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();

                    received_messages.lock().await.insert(response);
                }
            })
        };

        // 监听 TCP & UDP 数据
        while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
            match zip {
                Zip::Data(mut package) => {
                    let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                    // 处理 exit 指令，主动退出
                    if received_data.trim() == "exit" {
                        println!("[Server] Received exit signal, shutting down...");
                        package.set_data(Bytes::from("ack - exit"));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                        break;
                    }

                    // 回复客户端
                    package.set_data(Bytes::from(format!("ack - {}", received_data)));
                    let _ = tx
                        .clone()
                        .send(Zip::build_data(package))
                        .await
                        .hand_log(|msg| error!("{msg}"));
                }
                Zip::Event(_) => {}
            }
        }

        tcp_client_task.await.unwrap();
        udp_client_task.await.unwrap();

        // 确保所有预期的消息都被收到
        let received = received_messages.lock().await.clone();
        let mut expected_responses = HashSet::new();
        for i in 1..=3 {
            expected_responses.insert(format!("ack - tcp {}", i));
            expected_responses.insert(format!("ack - udp {}", i));
        }
        expected_responses.insert("ack - exit".to_string());

        assert!(received.contains("ack - exit"));
        assert_eq!(received, expected_responses);
        println!("received: {:?}", received);

        println!("[Server] Test completed.");
    }

    #[tokio::test]
    async fn test_single_std_all() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use exception::TransError;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpStream, UdpSocket};
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let client_addr = "0.0.0.0:0";
        let server_addr = "0.0.0.0:18891";

        // 记录接收到的数据
        let received_messages = Arc::new(Mutex::new(HashSet::new()));

        // 使用 net::sdx::run_by_tokio 初始化 ALL 模式（支持 TCP & UDP）
        let tu = net::sdx::listen(
            net::state::Protocol::ALL,
            SocketAddr::from_str(server_addr).unwrap(),
        )
        .unwrap();
        let (tx, mut rx) = net::sdx::run_by_tokio(tu).await.unwrap();

        // TCP 客户端任务
        let tcp_client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let mut stream = TcpStream::connect(server_addr).await.unwrap();
                let mut buffer = [0u8; 1024];

                let mut expected_responses = HashSet::new();

                for i in 1..=3 {
                    let msg = format!("tcp {}", i);
                    stream.write_all(msg.as_bytes()).await.unwrap();

                    expected_responses.insert(format!("ack - tcp {}", i));

                    let n = stream.read(&mut buffer).await.unwrap();
                    let response = String::from_utf8_lossy(&buffer[..n]).to_string();

                    received_messages.lock().await.insert(response);
                }

                // 发送 `exit` 指令，通知服务器关闭
                stream.write_all(b"exit").await.unwrap();

                let n = stream.read(&mut buffer).await.unwrap();
                let exit_response = String::from_utf8_lossy(&buffer[..n]).to_string();
                received_messages.lock().await.insert(exit_response);
            })
        };

        // UDP 客户端任务
        let udp_client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let socket = UdpSocket::bind(client_addr).await.unwrap();
                let target_addr = server_addr.parse::<SocketAddr>().unwrap();

                let mut expected_responses = HashSet::new();

                for i in 1..=3 {
                    let msg = format!("udp {}", i);
                    socket.send_to(msg.as_bytes(), &target_addr).await.unwrap();

                    expected_responses.insert(format!("ack - udp {}", i));

                    let mut buf = [0u8; 1024];
                    let (len, _) = socket.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();

                    received_messages.lock().await.insert(response);
                }
            })
        };

        // 监听 TCP & UDP 数据
        while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
            match zip {
                Zip::Data(mut package) => {
                    let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                    // 处理 exit 指令，主动退出
                    if received_data.trim() == "exit" {
                        println!("[Server] Received exit signal, shutting down...");
                        package.set_data(Bytes::from("ack - exit"));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                        break;
                    }

                    // 回复客户端
                    package.set_data(Bytes::from(format!("ack - {}", received_data)));
                    let _ = tx
                        .clone()
                        .send(Zip::build_data(package))
                        .await
                        .hand_log(|msg| error!("{msg}"));
                }
                Zip::Event(_) => {}
            }
        }

        tcp_client_task.await.unwrap();
        udp_client_task.await.unwrap();

        // 确保所有预期的消息都被收到
        let received = received_messages.lock().await.clone();
        let mut expected_responses = HashSet::new();
        for i in 1..=3 {
            expected_responses.insert(format!("ack - tcp {}", i));
            expected_responses.insert(format!("ack - udp {}", i));
        }
        expected_responses.insert("ack - exit".to_string());

        assert!(received.contains("ack - exit"));
        assert_eq!(received, expected_responses);
        println!("received: {:?}", received);

        println!("[Server] Test completed.");
    }

    #[tokio::test]
    async fn test_many_udp() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::net::UdpSocket;
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let client_addr = "0.0.0.0:0";
        let server_addr1 = "0.0.0.0:18900";
        let server_addr2 = "0.0.0.0:18901";
        let server_addr3 = "0.0.0.0:18902";

        // 记录接收到的数据
        let received_messages = Arc::new(Mutex::new(HashSet::new()));

        // 初始化 3 个 UDP 网络服务
        let (tx1, mut rx1) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(server_addr1).unwrap(),
        )
        .await
        .unwrap();
        let (tx2, mut rx2) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(server_addr2).unwrap(),
        )
        .await
        .unwrap();
        let (tx3, mut rx3) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(server_addr3).unwrap(),
        )
        .await
        .unwrap();

        // 启动三个任务，分别监听 3 个不同的端口
        tokio::spawn(async move {
            while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx1.recv()).await {
                match zip {
                    Zip::Data(mut package) => {
                        // println!(
                        //     "Port1: association = {:?} - data_size: {}",
                        //     package.get_association(),
                        //     package.get_data().len()
                        // );
                        let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                        // 处理 exit 指令，主动退出
                        if received_data.trim() == "exit" {
                            println!("[Server1] Received exit signal, shutting down...");
                            package.set_data(Bytes::from("ack - exit"));
                            let _ = tx1
                                .clone()
                                .send(Zip::build_data(package))
                                .await
                                .hand_log(|msg| error!("{msg}"));
                            break;
                        }

                        // 回复客户端
                        package.set_data(Bytes::from(format!("ack - {}", received_data)));
                        let _ = tx1
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                    Zip::Event(_) => {}
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx2.recv()).await {
                match zip {
                    Zip::Data(mut package) => {
                        // println!(
                        //     "Port2: association = {:?} - data_size: {}",
                        //     package.get_association(),
                        //     package.get_data().len()
                        // );
                        let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                        // 处理 exit 指令，主动退出
                        if received_data.trim() == "exit" {
                            println!("[Server2] Received exit signal, shutting down...");
                            package.set_data(Bytes::from("ack - exit"));
                            let _ = tx2
                                .clone()
                                .send(Zip::build_data(package))
                                .await
                                .hand_log(|msg| error!("{msg}"));
                            break;
                        }

                        // 回复客户端
                        package.set_data(Bytes::from(format!("ack - {}", received_data)));
                        let _ = tx2
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                    Zip::Event(_) => {}
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx3.recv()).await {
                match zip {
                    Zip::Data(mut package) => {
                        // println!(
                        //     "Port3: association = {:?} - data_size: {}",
                        //     package.get_association(),
                        //     package.get_data().len()
                        // );
                        let received_data = String::from_utf8_lossy(package.get_data()).to_string();

                        // 处理 exit 指令，主动退出
                        if received_data.trim() == "exit" {
                            println!("[Server3] Received exit signal, shutting down...");
                            package.set_data(Bytes::from("ack - exit"));
                            let _ = tx3
                                .clone()
                                .send(Zip::build_data(package))
                                .await
                                .hand_log(|msg| error!("{msg}"));
                            break;
                        }

                        // 回复客户端
                        package.set_data(Bytes::from(format!("ack - {}", received_data)));
                        let _ = tx3
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                    Zip::Event(_) => {}
                }
            }
        });

        // 创建客户端，分别发送数据到不同的端口
        let client_task = {
            let received_messages = received_messages.clone();
            tokio::spawn(async move {
                let socket1 = UdpSocket::bind(client_addr).await.unwrap(); // 绑定随机端口
                let socket2 = UdpSocket::bind(client_addr).await.unwrap();
                let socket3 = UdpSocket::bind(client_addr).await.unwrap();
                let addr1 = server_addr1.parse::<SocketAddr>().unwrap();
                let addr2 = server_addr2.parse::<SocketAddr>().unwrap();
                let addr3 = server_addr3.parse::<SocketAddr>().unwrap();

                let mut expected_responses = HashSet::new();

                // 发送消息到不同的端口
                for i in 1..=3 {
                    let msg1 = format!("udp1 {}", i);
                    socket1.send_to(msg1.as_bytes(), &addr1).await.unwrap();
                    expected_responses.insert(format!("ack - {}", msg1));

                    let msg2 = format!("udp2 {}", i);
                    socket2.send_to(msg2.as_bytes(), &addr2).await.unwrap();
                    expected_responses.insert(format!("ack - {}", msg2));

                    let msg3 = format!("udp3 {}", i);
                    socket3.send_to(msg3.as_bytes(), &addr3).await.unwrap();
                    expected_responses.insert(format!("ack - {}", msg3));
                }

                // 发送 exit 指令到三个端口
                socket1.send_to(b"exit", &addr1).await.unwrap();
                socket2.send_to(b"exit", &addr2).await.unwrap();
                socket3.send_to(b"exit", &addr3).await.unwrap();

                // 等待并收集响应
                let mut buf = [0u8; 1024];
                for _ in 0..3 {
                    // 3 messages + 3 exit responses
                    let (len, _) = socket1.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();
                    received_messages.lock().await.insert(response);

                    let (len, _) = socket2.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();
                    received_messages.lock().await.insert(response);

                    let (len, _) = socket3.recv_from(&mut buf).await.unwrap();
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();
                    received_messages.lock().await.insert(response);
                }

                // 确保服务器返回的响应符合预期
                let received = received_messages.lock().await.clone();
                assert_eq!(received, expected_responses);
                println!("received: {:?}", received);
            })
        };

        // 等待客户端任务完成
        client_task.await.unwrap();

        println!("[Server] Test completed.");
    }

    #[tokio::test]
    async fn test_many_tcp() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream, UdpSocket};
        use tokio::sync::Mutex;
        use tokio::time::{timeout, Duration};

        let server_addr1 = "0.0.0.0:18910";
        let server_addr2 = "0.0.0.0:18911";
        let server_addr3 = "0.0.0.0:18912";

        let (tx1, rx1) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(server_addr1).unwrap(),
        )
        .await
        .unwrap();
        let (tx2, rx2) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(server_addr2).unwrap(),
        )
        .await
        .unwrap();
        let (tx3, rx3) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(server_addr3).unwrap(),
        )
        .await
        .unwrap();

        let server_task = |mut rx: tokio::sync::mpsc::Receiver<Zip>,
                           tx: tokio::sync::mpsc::Sender<Zip>,
                           tag: String| {
            tokio::spawn(async move {
                while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
                    if let Zip::Data(mut package) = zip {
                        if package.get_data().as_ref() == b"exit" {
                            println!("Exit message received on {}. Exiting...", tag);
                            break;
                        }
                        // println!("{}: Received {} bytes", tag, package.get_data().len());
                        package.set_data(Bytes::from(format!("ack-{}", tag)));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                }
            });
        };

        server_task(rx1, tx1.clone(), "tcp1".to_string());
        server_task(rx2, tx2.clone(), "tcp2".to_string());
        server_task(rx3, tx3.clone(), "tcp3".to_string());

        let client_task = tokio::spawn(async move {
            let mut socket1 = TcpStream::connect(server_addr1).await.unwrap();
            let mut socket2 = TcpStream::connect(server_addr2).await.unwrap();
            let mut socket3 = TcpStream::connect(server_addr3).await.unwrap();

            let messages = vec![b"message1", b"message2", b"message3"];
            let mut received_responses = HashSet::new();

            for msg in messages {
                // 发送消息
                socket1.write_all(msg).await.unwrap();
                socket2.write_all(msg).await.unwrap();
                socket3.write_all(msg).await.unwrap();

                // 接收服务器返回的 ACK
                let mut buffer = [0u8; 64]; // 64字节缓存
                let n1 = socket1.read(&mut buffer).await.unwrap();
                let response1 = String::from_utf8_lossy(&buffer[..n1]);
                received_responses.insert(response1.to_string());

                let n2 = socket2.read(&mut buffer).await.unwrap();
                let response2 = String::from_utf8_lossy(&buffer[..n2]);
                received_responses.insert(response2.to_string());

                let n3 = socket3.read(&mut buffer).await.unwrap();
                let response3 = String::from_utf8_lossy(&buffer[..n3]);
                received_responses.insert(response3.to_string());
            }

            // 发送 exit 指令
            socket1.write_all(b"exit").await.unwrap();
            socket2.write_all(b"exit").await.unwrap();
            socket3.write_all(b"exit").await.unwrap();

            // 确保服务器返回了正确的 ACK
            assert!(received_responses.contains("ack-tcp1"));
            assert!(received_responses.contains("ack-tcp2"));
            assert!(received_responses.contains("ack-tcp3"));
            println!("received: {:?}", received_responses);
        });

        client_task.await.unwrap();
        println!("[Server] test_many_tcp completed.");
    }

    #[tokio::test]
    async fn test_many_all() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream, UdpSocket};
        use tokio::sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        };
        use tokio::time::{timeout, Duration};

        let client_addr = "0.0.0.0:0";
        let tcp_addr1 = "0.0.0.0:18920";
        let tcp_addr2 = "0.0.0.0:18921";
        let udp_addr1 = "0.0.0.0:18922";
        let udp_addr2 = "0.0.0.0:18923";

        // 初始化 TCP 连接
        let (tcp_tx1, tcp_rx1) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(tcp_addr1).unwrap(),
        )
        .await
        .unwrap();
        let (tcp_tx2, tcp_rx2) = net::init_net(
            net::state::Protocol::TCP,
            SocketAddr::from_str(tcp_addr2).unwrap(),
        )
        .await
        .unwrap();

        // 初始化 UDP 连接
        let (udp_tx1, udp_rx1) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(udp_addr1).unwrap(),
        )
        .await
        .unwrap();
        let (udp_tx2, udp_rx2) = net::init_net(
            net::state::Protocol::UDP,
            SocketAddr::from_str(udp_addr2).unwrap(),
        )
        .await
        .unwrap();

        // 服务器任务
        let server_task = |mut rx: Receiver<Zip>, tx: Sender<Zip>, tag: String| {
            tokio::spawn(async move {
                while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
                    if let Zip::Data(mut package) = zip {
                        if package.get_data().as_ref() == b"exit" {
                            println!("Exit message received on {}. Exiting...", tag);
                            break;
                        }
                        // println!("{}: Received {} bytes", tag, package.get_data().len());
                        package.set_data(Bytes::from(format!("ack-{}", tag)));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                }
            });
        };

        // 启动服务器任务
        server_task(tcp_rx1, tcp_tx1.clone(), "tcp1".to_string());
        server_task(tcp_rx2, tcp_tx2.clone(), "tcp2".to_string());
        server_task(udp_rx1, udp_tx1.clone(), "udp1".to_string());
        server_task(udp_rx2, udp_tx2.clone(), "udp2".to_string());

        // 客户端任务
        let client_task = tokio::spawn(async move {
            let mut tcp_socket1 = TcpStream::connect(tcp_addr1).await.unwrap();
            let mut tcp_socket2 = TcpStream::connect(tcp_addr2).await.unwrap();
            let udp_socket1 = UdpSocket::bind(client_addr).await.unwrap();
            let udp_socket2 = UdpSocket::bind(client_addr).await.unwrap();

            let messages = vec![b"message1", b"message2", b"message3"];
            let mut received_responses = HashSet::new();

            for msg in messages {
                // 发送 TCP 消息
                tcp_socket1.write_all(msg).await.unwrap();
                tcp_socket2.write_all(msg).await.unwrap();

                // 发送 UDP 消息
                udp_socket1.send_to(msg, udp_addr1).await.unwrap();
                udp_socket2.send_to(msg, udp_addr2).await.unwrap();

                // 读取 TCP 响应
                let mut buffer = [0u8; 64]; // 64字节缓存
                let n1 = tcp_socket1.read(&mut buffer).await.unwrap();
                let response1 = String::from_utf8_lossy(&buffer[..n1]);
                received_responses.insert(response1.to_string());

                let n2 = tcp_socket2.read(&mut buffer).await.unwrap();
                let response2 = String::from_utf8_lossy(&buffer[..n2]);
                received_responses.insert(response2.to_string());

                // 读取 UDP 响应
                let n3 = udp_socket1.recv(&mut buffer).await.unwrap();
                let response3 = String::from_utf8_lossy(&buffer[..n3]);
                received_responses.insert(response3.to_string());

                let n4 = udp_socket2.recv(&mut buffer).await.unwrap();
                let response4 = String::from_utf8_lossy(&buffer[..n4]);
                received_responses.insert(response4.to_string());
            }

            // 发送 exit 指令
            tcp_socket1.write_all(b"exit").await.unwrap();
            tcp_socket2.write_all(b"exit").await.unwrap();
            udp_socket1.send_to(b"exit", udp_addr1).await.unwrap();
            udp_socket2.send_to(b"exit", udp_addr2).await.unwrap();

            // 断言 ACK 是否正确
            assert!(received_responses.contains("ack-tcp1"));
            assert!(received_responses.contains("ack-tcp2"));
            assert!(received_responses.contains("ack-udp1"));
            assert!(received_responses.contains("ack-udp2"));
            println!("received: {:?}", received_responses);
        });

        client_task.await.unwrap();
        println!("[Server] test_many_all completed.");
    }

    #[tokio::test]
    async fn test_many_std_all() {
        use bytes::Bytes;
        use common::net;
        use common::net::state::Zip;
        use log::error;
        use std::collections::HashSet;
        use std::net::SocketAddr;
        use std::str::FromStr;
        use std::sync::Arc;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpStream, UdpSocket};
        use tokio::sync::{
            mpsc::{Receiver, Sender},
            Mutex,
        };
        use tokio::time::{timeout, Duration};

        let tcp_addr1 = "0.0.0.0:18930";
        let tcp_addr2 = "0.0.0.0:18931";
        let udp_addr1 = "0.0.0.0:18932";
        let udp_addr2 = "0.0.0.0:18933";
        let std_addr = "0.0.0.0:0";

        // 初始化 TCP 连接
        let tu1 = net::sdx::listen(
            net::state::Protocol::TCP,
            SocketAddr::from_str(tcp_addr1).unwrap(),
        )
        .unwrap();
        let (tcp_tx1, tcp_rx1) = net::sdx::run_by_tokio(tu1).await.unwrap();
        let tu2 = net::sdx::listen(
            net::state::Protocol::TCP,
            SocketAddr::from_str(tcp_addr2).unwrap(),
        )
        .unwrap();
        let (tcp_tx2, tcp_rx2) = net::sdx::run_by_tokio(tu2).await.unwrap();

        // 初始化 UDP 连接
        let tu3 = net::sdx::listen(
            net::state::Protocol::UDP,
            SocketAddr::from_str(udp_addr1).unwrap(),
        )
        .unwrap();
        let (udp_tx1, udp_rx1) = net::sdx::run_by_tokio(tu3).await.unwrap();
        let tu4 = net::sdx::listen(
            net::state::Protocol::UDP,
            SocketAddr::from_str(udp_addr2).unwrap(),
        )
        .unwrap();
        let (udp_tx2, udp_rx2) = net::sdx::run_by_tokio(tu4).await.unwrap();

        // 初始化标准输入输出
        let tu5 = net::sdx::listen(
            net::state::Protocol::ALL,
            SocketAddr::from_str(std_addr).unwrap(),
        )
        .unwrap();
        let (std_tx, std_rx) = net::sdx::run_by_tokio(tu5).await.unwrap();

        // 服务器任务
        let server_task = |mut rx: Receiver<Zip>, tx: Sender<Zip>, port: String| {
            tokio::spawn(async move {
                while let Ok(Some(zip)) = timeout(Duration::from_secs(10), rx.recv()).await {
                    if let Zip::Data(mut package) = zip {
                        if package.get_data().as_ref() == b"exit" {
                            println!("Exit message received on {}. Exiting...", port);
                            break;
                        }
                        // println!("{}: Received {} bytes", port, package.get_data().len());
                        package.set_data(Bytes::from(format!("ack-{}", port)));
                        let _ = tx
                            .clone()
                            .send(Zip::build_data(package))
                            .await
                            .hand_log(|msg| error!("{msg}"));
                    }
                }
            });
        };

        // 启动服务器任务
        server_task(tcp_rx1, tcp_tx1.clone(), "tcp1".to_string());
        server_task(tcp_rx2, tcp_tx2.clone(), "tcp2".to_string());
        server_task(udp_rx1, udp_tx1.clone(), "udp1".to_string());
        server_task(udp_rx2, udp_tx2.clone(), "udp2".to_string());
        server_task(std_rx, std_tx.clone(), "STD".to_string());

        // 客户端任务
        let client_task = tokio::spawn(async move {
            let mut tcp_socket1 = TcpStream::connect(tcp_addr1).await.unwrap();
            let mut tcp_socket2 = TcpStream::connect(tcp_addr2).await.unwrap();
            let udp_socket1 = UdpSocket::bind(std_addr).await.unwrap();
            let udp_socket2 = UdpSocket::bind(std_addr).await.unwrap();

            let messages = vec![b"message1", b"message2", b"message3"];
            let mut received_responses = HashSet::new();

            for msg in messages {
                // 发送 TCP 消息
                tcp_socket1.write_all(msg).await.unwrap();
                tcp_socket2.write_all(msg).await.unwrap();

                // 发送 UDP 消息
                udp_socket1.send_to(msg, udp_addr1).await.unwrap();
                udp_socket2.send_to(msg, udp_addr2).await.unwrap();

                // 读取 TCP 响应
                let mut buffer = [0u8; 64]; // 64字节缓存
                let n1 = tcp_socket1.read(&mut buffer).await.unwrap();
                let response1 = String::from_utf8_lossy(&buffer[..n1]);
                received_responses.insert(response1.to_string());

                let n2 = tcp_socket2.read(&mut buffer).await.unwrap();
                let response2 = String::from_utf8_lossy(&buffer[..n2]);
                received_responses.insert(response2.to_string());

                // 读取 UDP 响应
                let n3 = udp_socket1.recv(&mut buffer).await.unwrap();
                let response3 = String::from_utf8_lossy(&buffer[..n3]);
                received_responses.insert(response3.to_string());

                let n4 = udp_socket2.recv(&mut buffer).await.unwrap();
                let response4 = String::from_utf8_lossy(&buffer[..n4]);
                received_responses.insert(response4.to_string());
            }

            // 发送 exit 指令
            tcp_socket1.write_all(b"exit").await.unwrap();
            tcp_socket2.write_all(b"exit").await.unwrap();
            udp_socket1.send_to(b"exit", udp_addr1).await.unwrap();
            udp_socket2.send_to(b"exit", udp_addr2).await.unwrap();

            // 断言 ACK 是否正确
            assert!(received_responses.contains("ack-tcp1"));
            assert!(received_responses.contains("ack-tcp2"));
            assert!(received_responses.contains("ack-udp1"));
            assert!(received_responses.contains("ack-udp2"));
            println!("received: {:?}", received_responses);
        });

        client_task.await.unwrap();
        println!("[Server] test_many_std_all completed.");
    }
}
