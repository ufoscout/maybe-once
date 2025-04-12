#[cfg(test)]
mod tests {

    use std::sync::OnceLock;

    use maybe_once::blocking::{Data, MaybeOnce};
    use testcontainers::{runners::SyncRunner, Container};
    use testcontainers_modules::postgres::Postgres;

    type MaybeOnceType = (String, Container<Postgres>);

    /// Initializer function.
    /// Starts a Postgres container shared between all tests.
    /// It will be stopped when the tests terminate.
    fn init() -> MaybeOnceType {
        // startup the container
        let node = Postgres::default()
            .start()
            .expect("Could not start container");

        // prepare connection string
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
                .expect("Could not get db port")
        );

        (connection_string, node)
    }

    /// A function that holds a static reference to the container
    pub fn data(serial: bool) -> Data<'static, MaybeOnceType> {
        static DATA: OnceLock<MaybeOnce<MaybeOnceType>> = OnceLock::new();
        DATA.get_or_init(|| MaybeOnce::new(init)).data(serial)
    }

    // --------------------------------------------
    // All tests share the same container instance
    // --------------------------------------------

    #[test]
    fn test_1() {
        let data = data(false);
        let connection_string = &data.0;

        // container is up, you can use it
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_2() {
        let data = data(false);
        let connection_string = &data.0;

        // container is up, you can use it
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_3() {
        let data = data(false);
        let connection_string = &data.0;

        // container is up, you can use it
        let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
        let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
        assert_eq!(rows.len(), 1);
    }
}
