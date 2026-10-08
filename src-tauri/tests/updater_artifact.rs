//! Download and signature tests for real local release artifacts; never runs an installer.
//! Run after packaging: cargo test --test updater_artifact -- --ignored --test-threads=1
#![cfg(all(windows, debug_assertions))]

use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tauri_plugin_updater::{Error, UpdaterExt};

struct FixtureServer {
    endpoint: tauri::Url,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<usize>>,
}

impl FixtureServer {
    fn start(mut manifest: Value, binary: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        manifest["platforms"]["windows-x86_64"]["url"] = json!(format!("{base}/installer.exe"));
        let manifest = serde_json::to_vec(&manifest).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_worker = stop.clone();
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(15);
            let mut served = 0;
            while served < 2 && !stop_worker.load(Ordering::Relaxed) && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(stream) => stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                };
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut reader = BufReader::new(&stream);
                let mut request = String::new();
                reader.read_line(&mut request).unwrap();
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header == "\r\n" || header.is_empty() { break; }
                }
                drop(reader);
                let path = request.split_whitespace().nth(1).unwrap();
                let (body, content_type) = match path {
                    "/latest.json" => (manifest.as_slice(), "application/json"),
                    "/installer.exe" => (binary.as_slice(), "application/octet-stream"),
                    _ => panic!("unexpected fixture request: {path}"),
                };
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
                stream.write_all(body).unwrap();
                served += 1;
            }
            served
        });
        Self { endpoint: format!("{base}/latest.json").parse().unwrap(), stop, thread: Some(thread) }
    }

    fn finish(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        assert_eq!(self.thread.take().unwrap().join().unwrap(), 2, "both manifest and installer must be fetched");
    }
}

impl Drop for FixtureServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() { let _ = thread.join(); }
    }
}

fn artifacts() -> (Value, Value, Vec<u8>) {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let release = std::env::var_os("AI_TOKEN_RELEASE_DIR").map(PathBuf::from).unwrap_or_else(|| project.join("release"));
    let config: Value = serde_json::from_slice(&std::fs::read(project.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let updater = config["plugins"]["updater"].clone();
    assert_eq!(updater["requireSignedVersion"], true, "production signature version binding must be enabled");
    assert!(!updater["pubkey"].as_str().unwrap().is_empty(), "public verification key is required");
    let manifest: Value = serde_json::from_slice(&std::fs::read(release.join("latest.json")).expect("package the real release first")).unwrap();
    let url: tauri::Url = manifest["platforms"]["windows-x86_64"]["url"].as_str().unwrap().parse().unwrap();
    let filename = url.path_segments().unwrap().next_back().unwrap();
    assert!(!filename.contains('%'), "use a plain ASCII release filename");
    assert!(filename.ends_with("-setup.exe"), "the update asset must be the installer");
    let binary = std::fs::read(release.join(filename)).unwrap();
    let signature = std::fs::read_to_string(release.join(format!("{filename}.sig"))).unwrap();
    assert_eq!(signature.trim(), manifest["platforms"]["windows-x86_64"]["signature"].as_str().unwrap());
    assert_eq!(manifest["version"], config["version"], "packaged manifest and app must agree");
    (updater, manifest, binary)
}

fn download(updater_config: Value, manifest: Value, binary: Vec<u8>) -> Result<Vec<u8>, Error> {
    let server = FixtureServer::start(manifest, binary);
    let mut context = tauri::test::mock_context(tauri::test::noop_assets());
    context.config_mut().plugins.0.insert("updater".into(), updater_config);
    let app = tauri::test::mock_builder()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .build(context).unwrap();
    let result = tauri::async_runtime::block_on(async {
        // Mock runtime reports current version 0.1.0; production comparator is untouched.
        // HTTP is accepted only in this debug-only loopback fixture.
        let update = app.updater_builder().endpoints(vec![server.endpoint.clone()])?
            .target("windows-x86_64").no_proxy().timeout(Duration::from_secs(15))
            .build()?.check().await?.expect("the real packaged version must exceed mock 0.1.0");
        update.download(|_, _| {}, || {}).await
    });
    server.finish();
    result
}

#[test]
#[ignore = "requires the packaged installer, signature, and latest.json"]
fn real_installer_download_verifies_signature_and_version() {
    let (config, manifest, binary) = artifacts();
    let original = binary.clone();
    assert_eq!(download(config, manifest, binary).expect("real signature must verify"), original);
}

#[test]
#[ignore = "requires the packaged installer, signature, and latest.json"]
fn tampered_installer_is_rejected_cryptographically() {
    let (config, manifest, mut binary) = artifacts();
    let last = binary.last_mut().expect("installer cannot be empty");
    *last ^= 1;
    let error = download(config, manifest, binary).unwrap_err();
    assert!(matches!(&error, Error::Minisign(_)), "expected signature rejection, got {error}");
}

#[test]
#[ignore = "requires the packaged installer, signature, and latest.json"]
fn genuine_installer_cannot_be_relabelled_as_another_version() {
    let (config, mut manifest, binary) = artifacts();
    assert_ne!(manifest["version"], "99.0.0");
    // This altered metadata is served exclusively by the local test fixture.
    manifest["version"] = json!("99.0.0");
    let error = download(config, manifest, binary).unwrap_err();
    assert!(matches!(&error, Error::SignedVersionMismatch { .. }), "expected version-binding rejection, got {error}");
}
