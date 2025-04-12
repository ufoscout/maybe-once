#[cfg(test)]
mod tests {

    use std::sync::OnceLock;

    use deadpool::{managed::Pool, Runtime};
    use deadpool_postgres::Manager;
    use maybe_once::tokio::{Data, MaybeOnceAsync};
    use postgres::NoTls;
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;

    type MaybeOnceType = (Pool<Manager>, ContainerAsync<Postgres>);

    /// Initializer function.
    /// Starts a Postgres container shared between all tests.
    /// It will be stopped when the tests terminate.
    async fn init() -> MaybeOnceType {
        // startup the container
        let node = Postgres::default()
            .start()
            .await
            .expect("Could not start container");

        let config = deadpool_postgres::Config {
            user: Some("postgres".to_owned()),
            password: Some("postgres".to_owned()),
            dbname: Some("postgres".to_owned()),
            host: Some("127.0.0.1".to_string()),
            port: Some(
                node.get_host_port_ipv4(5432)
                    .await
                    .expect("Could not get db port"),
            ),
            ..Default::default()
        };

        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .expect("Could not create connection pool");

        (pool, node)
    }

    /// A function that holds a static reference to the container
    pub async fn data(serial: bool) -> Data<'static, MaybeOnceType> {
        static DATA: OnceLock<MaybeOnceAsync<MaybeOnceType>> = OnceLock::new();
        DATA.get_or_init(|| MaybeOnceAsync::new(|| Box::pin(init())))
            .data(serial)
            .await
    }

    // --------------------------------------------
    // All tests share the same container instance
    // --------------------------------------------

    #[tokio::test]
    async fn test_1() {
        let data = data(false).await;
        let pool = &data.0;
        let conn = pool.get().await.unwrap();

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn test_2() {
        let data = data(false).await;
        let pool = &data.0;
        let conn = pool.get().await.unwrap();

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn test_3() {
        let data = data(false).await;
        let pool = &data.0;
        let conn = pool.get().await.unwrap();

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }
}
