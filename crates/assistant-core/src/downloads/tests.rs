use super::*;
#[tokio::test]
async fn resumes_partial_download_with_range_and_if_range() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"complete installer fixture";
    let (service, release, asset, mut task) = fixture(dir.path(), bytes);
    let folder = dir.path().join("downloads").join(&release.tag);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(format!("{}.part", task.id)), &bytes[..5]).unwrap();
    task.etag = Some("\"stable\"".into());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let body = bytes.to_vec();
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let n = socket.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..n]).to_lowercase();
        assert!(request.contains("range: bytes=5-"));
        assert!(request.contains("if-range: \"stable\""));
        let header=format!("HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 5-{}/{}\r\nETag: \"stable\"\r\nConnection: close\r\n\r\n",body.len()-5,body.len()-1,body.len());
        socket.write_all(header.as_bytes()).unwrap();
        socket.write_all(&body[5..]).unwrap();
    });
    let result = service.transfer(task, release, asset, url, |_| {}).await;
    thread.join().unwrap();
    assert_eq!(result.stage, "ready");
    assert_eq!(fs::read(result.path.unwrap()).unwrap(), bytes);
}
#[tokio::test]
async fn ignored_range_restarts_instead_of_appending() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"complete fixture";
    let (service, release, asset, task) = fixture(dir.path(), bytes);
    let folder = dir.path().join("downloads").join(&release.tag);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join(format!("{}.part", task.id)), b"wrong prefix").unwrap();
    let result = service
        .transfer(
            task,
            release,
            asset,
            server(bytes, Some(bytes.len())),
            |_| {},
        )
        .await;
    assert_eq!(result.stage, "ready");
    assert_eq!(fs::read(result.path.unwrap()).unwrap(), bytes);
}
#[test]
fn rejects_wrong_resume_ranges() {
    assert!(validate_range("bytes 5-9/10", 5, 10).is_ok());
    assert!(validate_range("bytes 0-9/10", 5, 10).is_err());
    assert!(validate_range("bytes 5-19/20", 5, 10).is_err());
}
#[test]
fn mirror_identity_and_channel_changes_are_validated() {
    let config = ChannelConfig::default();
    assert!(config
        .user_agent()
        .unwrap()
        .contains("https://github.com/shibaweidu/claude-cn-assistant"));
    assert!(ChannelConfig {
        accelerated: true,
        repository: REPO.into()
    }
    .user_agent()
    .is_err());
    assert!(ChannelConfig {
        accelerated: true,
        repository: "https://github.com/owner/repo?token=secret".into()
    }
    .user_agent()
    .is_err());
    let dir = tempfile::tempdir().unwrap();
    let service = Downloads::new(dir.path().into()).unwrap();
    let state = service
        .configure_channel(ChannelConfig {
            accelerated: false,
            ..config
        })
        .unwrap();
    assert!(!state.channel.accelerated);
    assert!(state.release.is_none());
}
use std::net::TcpListener;
const MANIFEST: &[u8] = include_bytes!("../../../../tests/fixtures/mirror-manifest-v2.json");
const CHECKSUMS: &str = include_str!("../../../../tests/fixtures/SHA256SUMS.txt");
#[test]
fn live_release_fixture_and_cross_version_rejection() {
    let release = parse_release("claude-app-v6.28899.6", MANIFEST, CHECKSUMS).unwrap();
    assert_eq!(release.assets.len(), 5);
    assert_eq!(release.assets[0].size, 312231751);
    assert!(parse_release("claude-app-v6.0.0", MANIFEST, CHECKSUMS).is_err());
    let mut bytes = MANIFEST.to_vec();
    bytes.push(b' ');
    assert!(parse_release("claude-app-v6.28899.6", &bytes, CHECKSUMS).is_err());
    let invalid = CHECKSUMS.replace("d95045a5", "00000000");
    assert!(parse_release("claude-app-v6.28899.6", MANIFEST, &invalid).is_err());
}
#[test]
fn checksum_duplicates_and_unsafe_names_rejected() {
    let hash = "a".repeat(64);
    assert!(parse_checksums(&format!("{hash}  file\n{hash} *file")).is_err());
    assert!(parse_checksums(&format!("{hash}  ../file")).is_err());
    assert!(parse_checksums("wrong  file").is_err());
}
#[test]
fn semantic_version_comparison() {
    assert!(compare_versions("6.28899.6", "6.6395.0").unwrap().is_gt());
    assert!(compare_versions("6.9", "6.10").unwrap().is_lt());
    assert!(compare_versions("6.9.0", "6.9").unwrap().is_eq());
    assert!(compare_versions("6.9-beta", "6.9").is_err());
}
fn fixture(root: &Path, bytes: &[u8]) -> (Downloads, Release, Asset, Task) {
    let service = Downloads::new(root.to_path_buf()).unwrap();
    service.channel.lock().unwrap().accelerated = false;
    let asset = Asset {
        platform: "windows".into(),
        architecture: "x64".into(),
        name: "Claude-win-x64.msix".into(),
        version: "1.2.3".into(),
        size: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    };
    let release = Release {
        accelerated: false,
        version: "1.2.3".into(),
        tag: "claude-app-v1.2.3".into(),
        checked_at: now(),
        assets: vec![asset.clone()],
    };
    service.cache.lock().unwrap().release = Some(release.clone());
    let (task, _, _) = service.reserve("windows", "x64").unwrap();
    (service, release, asset, task)
}
fn server(body: &[u8], length: Option<usize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_vec();
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 4096];
        let _ = socket.read(&mut buffer);
        let header = match length {
            Some(n) => {
                format!("HTTP/1.1 200 OK\r\nContent-Length: {n}\r\nConnection: close\r\n\r\n")
            }
            None => "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".into(),
        };
        let _ = socket.write_all(header.as_bytes());
        let _ = socket.write_all(&body);
    });
    format!("http://{address}/package")
}
#[tokio::test]
async fn unknown_http_length_download_is_verified_and_persists() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"installer-fixture-not-executable";
    let (service, release, asset, task) = fixture(dir.path(), bytes);
    assert!(service.reserve("windows", "x64").is_err());
    let completed = service
        .transfer(task, release, asset, server(bytes, None), |_| {})
        .await;
    assert_eq!(completed.stage, "ready");
    assert_eq!(fs::read(completed.path.unwrap()).unwrap(), bytes);
    let reloaded = Downloads::new(dir.path().into()).unwrap();
    assert_eq!(reloaded.state().task.unwrap().stage, "unverified");
    assert_eq!(reloaded.verify_saved().unwrap().stage, "ready");
    let path = reloaded.ready_path().unwrap();
    fs::write(path, b"changed").unwrap();
    assert_eq!(reloaded.verify_saved().unwrap().stage, "failed");
}
#[tokio::test]
async fn hash_mismatch_and_interrupted_body_never_become_ready() {
    for interrupted in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let bytes = b"expected installer";
        let (service, release, asset, task) = fixture(dir.path(), bytes);
        let body = if interrupted {
            &bytes[..3]
        } else {
            b"corrupted contents"
        };
        let completed = service
            .transfer(
                task,
                release,
                asset,
                server(body, Some(bytes.len())),
                |_| {},
            )
            .await;
        assert_eq!(completed.stage, "failed");
        assert!(completed.path.is_none());
        assert!(completed.error.is_some());
        let files: Vec<_> = fs::read_dir(dir.path().join("downloads/claude-app-v1.2.3"))
            .unwrap()
            .collect();
        if interrupted {
            assert_eq!(files.len(), 1);
        } else {
            assert!(files.is_empty());
        }
    }
}
#[tokio::test]
async fn cancellation_cleans_partial_file() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = b"expected installer";
    let (service, release, asset, task) = fixture(dir.path(), bytes);
    let id = task.id.clone();
    let completed = service
        .transfer(
            task,
            release,
            asset,
            server(bytes, Some(bytes.len())),
            |t| {
                if t.stage == "downloading" {
                    service.cancel(&id).unwrap();
                }
            },
        )
        .await;
    assert_eq!(completed.stage, "cancelled");
    assert!(completed.path.is_none());
    assert!(service.cancel(&id).is_err());
}
#[test]
fn restart_marks_pending_transfer_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    let (_service, _, _, _) = fixture(dir.path(), b"test");
    let reloaded = Downloads::new(dir.path().into()).unwrap();
    assert_eq!(reloaded.state().task.unwrap().stage, "failed");
}
#[tokio::test]
async fn rate_limit_sets_cooldown_and_retains_cached_release() {
    let dir = tempfile::tempdir().unwrap();
    let (service, _, _, _) = fixture(dir.path(), b"test");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/metadata", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        let _ = socket.read(&mut request);
        socket.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    });
    assert!(service.metadata(url).await.is_err());
    assert!(service.state().retry_at >= now() + 119);
    assert!(service.state().release.is_some());
    assert!(service.check(true).await.is_err());
}
#[test]
fn release_redirects_are_restricted() {
    assert!(!allowed(&Url::parse("http://github.com/file").unwrap()));
    assert!(!allowed(&Url::parse("https://evil.example/file").unwrap()));
    assert!(allowed(
        &Url::parse("https://release-assets.githubusercontent.com/file").unwrap()
    ));
}

#[test]
fn tampered_cache_cannot_change_download_path() {
    let dir = tempfile::tempdir().unwrap();
    let (service, _, _, _) = fixture(dir.path(), b"test");
    *service.task.lock().unwrap() = None;
    service
        .cache
        .lock()
        .unwrap()
        .release
        .as_mut()
        .unwrap()
        .assets[0]
        .name = "../evil.msix".into();
    assert!(service.reserve("windows", "x64").is_err());
}
