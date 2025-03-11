use common::chrono::NaiveDateTime;
use common::dbx::mysqlx::get_conn_by_pool;
use common::exception::{GlobalResult, TransError};
use common::log::error;
use common::sqlx;

pub async fn get_device_channel_status(
    device_id: &String,
    channel_id: &String,
) -> GlobalResult<Option<String>> {
    let pool = get_conn_by_pool()?;
    let res: Option<(String,)> = sqlx::query_as(
        "SELECT IFNULL(c.`status`,'ONLY') FROM gb_device_list d LEFT JOIN gb_device_channel_list c on d.device_id=c.device_id and c.channel_id=? WHERE d.device_id=?"
    )
        .bind(channel_id)
        .bind(device_id)
        .fetch_optional(pool).await.hand_log(|msg| error!("{msg}"))?;
    Ok(res.map(|(v,)| v))
}

pub async fn get_device_status_info(
    device_id: &String,
) -> GlobalResult<Option<(u8, u8, u32, NaiveDateTime, u8)>> {
    let pool = get_conn_by_pool()?;
    let res = sqlx::query_as::<_, (u8, u8, u32, NaiveDateTime, u8)>(
        "SELECT o.heartbeat_sec,o.`status`,d.register_expires,d.register_time,d.`status` FROM gb_oauth o INNER JOIN gb_device_list d ON o.device_id = d.device_id where d.device_id=?",
    ).bind(device_id).fetch_optional(pool).await.hand_log(|msg| error!("{msg}"))?;
    Ok(res)
}

#[cfg(test)]
#[allow(dead_code, unused_imports)]
mod test {
    use super::*;
    use common::confgen::conf::init_confgen;
    use common::dbx::mysqlx;
    use common::tokio;

    // #[tokio::test]
    async fn test_get_device_channel_status() {
        init();
        let result = get_device_channel_status(
            &"34020000001110000001".to_string(),
            &"34020000001320000180".to_string(),
        )
        .await;
        println!("{:?}", result);
    }

    // #[tokio::test]
    async fn test_get_device_status_info() {
        init();
        let status_info = get_device_status_info(&"34020000001110000001".to_string()).await;
        println!("{:?}", status_info);
    }

    fn init() {
        init_confgen("config.yml".to_string());
        let _ = mysqlx::init_conn_pool();
    }
}
