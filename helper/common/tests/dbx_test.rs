#[allow(dead_code, unused_imports)]
mod test {
    use common::dbx::mysqlx;
    use confgen::conf;
    use sqlx::query_builder::QueryBuilder;
    use sqlx::ConnectOptions;
    use sqlx::Row;

    #[tokio::test]
    async fn test_mysqlx() {
        conf::init_confgen("tests/mysql.yaml".to_string());
        mysqlx::init_conn_pool().unwrap();
        let pool = mysqlx::get_conn_by_pool().unwrap();
        let mut create_table_builder = QueryBuilder::new(
            "CREATE TABLE IF NOT EXISTS test_table (id INT AUTO_INCREMENT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                age INT NOT NULL);",
        );
        let res = create_table_builder.build().execute(pool).await;
        println!("create: {:?}", res);

        let mut insert_builder = QueryBuilder::new(
            r#"INSERT INTO test_table (name, age) VALUES ("Alice", 25), ("Bob", 30);"#,
        );
        let res = insert_builder.build().execute(pool).await;
        println!("insert: {:?}", res);

        // 查询数据
        let mut select_builder = QueryBuilder::new("SELECT * FROM test_table");
        let select_query = select_builder.build().fetch_all(pool).await.unwrap();
        // println!("查询结果: {:?}", select_query);
        for row in select_query {
            let id: i32 = row.get("id");
            let name: String = row.get("name");
            let age: i32 = row.get("age");
            println!("ID: {}, 姓名: {}, 年龄: {}", id, name, age);
        }

        // 修改数据
        let mut update_builder = QueryBuilder::new("UPDATE test_table SET ");
        update_builder
            .push("age = ")
            .push_bind(26)
            .push(" WHERE name = ")
            .push_bind("Alice");
        let res = update_builder.build().execute(pool).await;
        println!("{:?}", res);

        // 再次查询数据，验证修改结果
        let mut updated_select_builder = QueryBuilder::new("SELECT * FROM test_table");
        let updated_select_query = updated_select_builder
            .build()
            .fetch_all(pool)
            .await
            .unwrap();
        // println!("修改后的查询结果: {:?}", updated_select_query);
        for row in updated_select_query {
            let id: i32 = row.get("id");
            let name: String = row.get("name");
            let age: i32 = row.get("age");
            println!("ID: {}, 姓名: {}, 年龄: {}", id, name, age);
        }

        // 删除测试表格
        let mut drop_table_builder = QueryBuilder::new("DROP TABLE IF EXISTS test_table");
        let res = drop_table_builder.build().execute(pool).await;
        println!("{:?}", res);
    }

    use tokio::sync::oneshot;
    use tokio::task;

    #[tokio::test]
    async fn test_oneshot_channel() {
        // 创建一个 oneshot 通道
        let (tx, rx) = oneshot::channel();

        // 启动一个异步任务来发送消息
        let send_task = task::spawn(async move {
            // 模拟一些工作
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            // 发送消息
            let message = "Hello from sender!";
            let send_result = tx.send(message);
            assert!(send_result.is_ok(), "消息发送失败");
        });

        // 启动一个异步任务来接收消息
        let recv_task = task::spawn(async move {
            // 等待接收消息
            let recv_result = rx.await;
            assert!(recv_result.is_ok(), "消息接收失败");
            let received_message = recv_result.unwrap();
            assert_eq!(
                received_message, "Hello from sender!",
                "接收到的消息与发送的消息不匹配"
            );
        });

        // 等待发送任务完成
        send_task.await.expect("发送任务执行失败");
        // 等待接收任务完成
        recv_task.await.expect("接收任务执行失败");
    }

    #[tokio::test]
    async fn test_oneshot_channel2() {
        // 创建一个 oneshot 通道
        let (tx, rx) = oneshot::channel();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        // 发送消息
        let message = "Hello from sender!";
        let send_result = tx.send(message);
        assert!(send_result.is_ok(), "消息发送失败");

        // 启动一个异步任务来接收消息
        let recv_task = task::spawn(async move {
            // 等待接收消息
            let recv_result = rx.await;
            assert!(recv_result.is_ok(), "消息接收失败");
            let received_message = recv_result.unwrap();
            assert_eq!(
                received_message, "Hello from sender!",
                "接收到的消息与发送的消息不匹配"
            );
        });

        // 等待接收任务完成
        recv_task.await.expect("接收任务执行失败");
    }

    #[tokio::test]
    async fn test_oneshot_channel3() {
        // 创建一个 oneshot 通道
        let (tx, rx) = oneshot::channel();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        // 发送消息
        let message = "Hello from sender!";
        let send_result = tx.send(message);
        assert!(send_result.is_ok(), "消息发送失败");

        // 等待接收消息
        let recv_result = rx.await;
        assert!(recv_result.is_ok(), "消息接收失败");
        let received_message = recv_result.unwrap();
        assert_eq!(
            received_message, "Hello from sender!",
            "接收到的消息与发送的消息不匹配"
        );
    }
}
