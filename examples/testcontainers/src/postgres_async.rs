
#[cfg(test)]
mod tests {
    
    use std::sync::OnceLock;

    use maybe_once::tokio::{Data, MaybeOnceAsync};
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;

    type MaybeOnceType = (String, ContainerAsync<Postgres>);

    /// Initializer function.
    /// Starts a Postgres container shared between all tests.
    /// It will be stopped when the tests terminate.
    async fn init() -> MaybeOnceType {
        // startup the container
        let node = Postgres::default().start().await.expect("Could not start container");

        // prepare connection string
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432).await.expect("Could not get db port")
        );

        (connection_string, node)
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
        let connection_string = &data.0;

        // container is up, you can use it
        let (conn, connection) = tokio_postgres::connect(connection_string, tokio_postgres::NoTls).await.unwrap();

            // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }


    #[tokio::test]
    async fn test_2() {
        let data = data(false).await;
        let connection_string = &data.0;

        // container is up, you can use it
        let (conn, connection) = tokio_postgres::connect(connection_string, tokio_postgres::NoTls).await.unwrap();

            // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }


    #[tokio::test]
    async fn test_3() {
        let data = data(false).await;
        let connection_string = &data.0;

        // container is up, you can use it
        let (conn, connection) = tokio_postgres::connect(connection_string, tokio_postgres::NoTls).await.unwrap();

            // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

        let rows = conn.query("SELECT 1 + 1", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
    }
}