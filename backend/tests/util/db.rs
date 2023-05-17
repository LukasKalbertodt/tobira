pub(super) const DB_USER: &str = "tobira";
pub(super) const DB_PASSWORD: &str = "tobira";

pub(super) struct TmpDb {
    pub(super) db_name: String,
}

impl TmpDb {
    pub(super) fn create(test_id: &str) -> Self {
        let mut client = Self::client();
        let db_name = format!("tobira_test_{test_id}");
        client.execute(&format!("create database {db_name}"), &[])
            .expect("failed to create temporary test DB");

        Self { db_name }
    }

    fn client() -> postgres::Client {
        postgres::config::Config::new()
            .user(DB_USER)
            .password(DB_PASSWORD)
            .dbname("tobira")
            .host("127.0.0.1")
            .application_name("Tobira integration tests")
            .connect(postgres::NoTls)
            .expect("could not connect to DB in tests")
    }
}

impl Drop for TmpDb {
    fn drop(&mut self) {
        let mut client = Self::client();
        client.execute(&format!("drop database {}", self.db_name), &[])
            .expect("failed to drop temporary test database");
    }
}
