use std::{collections::HashMap, path::{Path, PathBuf}, process::Child, time::Duration};

mod db;
mod http;

pub use self::http::{HttpClient, HttpReqBuilder, HttpResponse};
use self::db::{TmpDb, DB_PASSWORD};


/// Setup to run a Tobira process.
pub struct TobiraCmd {
    config: HashMap<String, toml::Value>,
    args: Vec<String>,
}

const EXE_PATH: &str = env!("CARGO_BIN_EXE_tobira");
const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");

impl TobiraCmd {
    pub fn new<const N: usize>(args: [&str; N]) -> Self {
        Self {
            config: Self::base_config(),
            args: args.into_iter().map(|s| s.to_owned()).collect(),
        }
    }

    /// Set a config value for this process, e.g. `"log.level"`.
    #[allow(dead_code)]
    pub fn set_config(mut self, key: &str, value: impl Into<toml::Value>) -> Self {
        let forbidden_configs = [
            "http.unix_socket", "http.port", "meili.index_prefix", "db.database"
        ];
        if forbidden_configs.contains(&key) {
            panic!("Cannot specify '{key}' as it will be overwritten later");
        }

        self.config.insert(key.into(), value.into());
        self
    }

    /// Run the configured Tobira process in an isolated way. The closure `f` is
    /// called once all setup is complete and Tobira runs. Once `f` exists,
    /// cleanup is performed.
    pub fn run<F>(mut self, f: F)
    where
        F: FnOnce(&RunningTobira) -> Result<(), Box<dyn std::error::Error>>,
    {
        // ===== Setup ====================
        let test_id = random_test_id();
        println!("Test ID: {test_id}"); // For debugging if the test fails

        // Create temporary directory
        let dir = Path::new(TMP_DIR).join(&format!("test_{test_id}"));
        let dir_str = dir.to_str().expect("test tempdir is not valid UTF8");
        std::fs::create_dir(&dir).expect("failed to create temp dir for test");

        // Configure isolated Meili indices for this test.
        let meili_prefix = format!("tobira_test_{test_id}");
        self.config.insert("meili.index_prefix".into(), meili_prefix.clone().into());

        // Configure Tobira to listen on UNIX socket
        let unix_socket = format!("{dir_str}/listen.sock");
        self.config.insert("http.unix_socket".into(), unix_socket.clone().into());

        // Create temporary empty database
        let db = TmpDb::create(&test_id);
        self.config.insert("db.database".into(), db.db_name.clone().into());

        // Place config file into temp dir
        let config_path = dir.join("config.toml");
        std::fs::write(&config_path, &self.config_file())
            .expect("failed to write config file for test");


        // ===== Running the test ===========
        let mut child = std::process::Command::new(EXE_PATH)
            .args(self.args)
            .env("TOBIRA_CONFIG_PATH", &config_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn Tobira process for testing");

        let running = RunningTobira {
            process: &child,
            unix_socket,
            config_file: config_path,
        };
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f(&running).expect("test runner returned an error");
        }));

        // Apparently the only reason for errors is if the child is not running
        // anymore. We ignore that case.
        let _ = child.kill();


        // ===== Cleanup ====================
        std::fs::remove_dir_all(&dir).expect("failed to remove temp directory for test");
        drop(db); // Make dropping explicit

        // TODO: remove meili indices

        if let Err(e) =  panic {
            std::panic::resume_unwind(e);
        }
    }

    fn config_file(&self) -> String {
        use std::fmt::Write;

        let mut out = String::new();
        for (key, value) in &self.config {
            writeln!(out, "{key} = {value}").unwrap();
        }
        out
    }

    fn base_config() -> HashMap<String, toml::Value> {
        let entries = [
            ("general.site_title.en", toml::Value::from("Tobira integration test")),
            ("meili.key", "tobira".into()),
            ("sync.user", "admin".into()),
            ("sync.password", "opencast".into()),
            ("opencast.host", "http://localhost:8080".into()),

            ("db.password", DB_PASSWORD.into()),
            ("db.tls_mode", "off".into()),

            // TODO: Http listen
            // TODO: meili prefix

            // The image files are just opened and served, not inspected. So we
            // can just use an empty file here.
            ("theme.favicon", "/dev/null".into()),
            ("theme.logo.large.path", "/dev/null".into()),
            ("theme.logo.large.resolution", vec![1, 1].into()),
        ];

        HashMap::from(entries.map(|(k, v)| (k.to_owned(), v)))
    }
}

/// A handle to the running (potentially already exited) Tobira process with
/// useful information.
pub struct RunningTobira<'a> {
    pub process: &'a Child,
    pub unix_socket: String,
    pub config_file: PathBuf,
}

impl RunningTobira<'_> {
    /// Wait until the Unix socket has been opened.
    pub fn wait_for_http_ready(&self) {
        let wait_interval_ms = 10;
        for _ in 0..3000 / wait_interval_ms {
            if Path::new(&self.unix_socket).exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(wait_interval_ms));
        }

        panic!("Timeout: Tobira never opened Unix socket");
    }

    /// Return a value with which you can easily perform HTTP requests against
    /// Tobira.
    pub fn http_client(&self) -> HttpClient {
        self.wait_for_http_ready();
        HttpClient::new(&self.unix_socket)
    }
}


fn random_test_id() -> String {
    let n = rand::random::<u64>();
    hex::encode(&n.to_be_bytes())
}
