use sha2::{Digest, Sha256};
use tempfile::tempdir;
use tscribe::model::{Model, download, ensure, sha256_file};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn download_writes_and_verifies_file() {
    let server = MockServer::start().await;
    let body = b"fake model data".to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let real_sha = hex::encode(hasher.finalize());

    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;

    let url: &'static str = Box::leak(format!("{}/model.bin", server.uri()).into_boxed_str());
    let sha: &'static str = Box::leak(real_sha.into_boxed_str());
    let model = Model {
        name: "test",
        size_mb: 0,
        url,
        sha256: sha,
        multilingual: false,
    };

    let dir = tempdir().unwrap();
    let path = download(&model, dir.path(), |_, _| {}).await.unwrap();
    assert!(path.exists());
    assert_eq!(sha256_file(&path).unwrap(), model.sha256);
}

#[tokio::test]
async fn ensure_uses_cached_file_when_valid() {
    let server = MockServer::start().await;
    let body = b"cached model data".to_vec();
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let real_sha = hex::encode(hasher.finalize());

    // Server fails on second hit; if `ensure` redownloads, we'd error.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let url: &'static str = Box::leak(format!("{}/model.bin", server.uri()).into_boxed_str());
    let sha: &'static str = Box::leak(real_sha.into_boxed_str());
    let model = Model {
        name: "test",
        size_mb: 0,
        url,
        sha256: sha,
        multilingual: false,
    };

    let dir = tempdir().unwrap();
    ensure(&model, dir.path(), |_, _| {}).await.unwrap();
    ensure(&model, dir.path(), |_, _| {}).await.unwrap();
}

#[tokio::test]
async fn checksum_mismatch_returns_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"wrong data".to_vec()))
        .mount(&server)
        .await;

    let url: &'static str = Box::leak(format!("{}/model.bin", server.uri()).into_boxed_str());
    let model = Model {
        name: "test",
        size_mb: 0,
        url,
        sha256: "0000000000000000000000000000000000000000000000000000000000000000",
        multilingual: false,
    };

    let dir = tempdir().unwrap();
    let err = download(&model, dir.path(), |_, _| {}).await.unwrap_err();
    assert_eq!(err.exit_code(), 6);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn cancelling_a_partial_download_removes_the_temporary_file() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url =
        Box::leak(format!("http://{}/model", listener.local_addr().unwrap()).into_boxed_str());
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).await.unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\npartial")
            .await
            .unwrap();
        std::future::pending::<()>().await;
    });
    let model = Model {
        name: "test",
        size_mb: 0,
        url,
        sha256: "unused",
        multilingual: false,
    };
    let dir = tempdir().unwrap();
    let (sent, received) = tokio::sync::oneshot::channel();
    let sent = std::sync::Mutex::new(Some(sent));
    {
        let download = download(&model, dir.path(), |_, _| {
            if let Some(sent) = sent.lock().unwrap().take() {
                let _ = sent.send(());
            }
        });
        tokio::pin!(download);
        tokio::select! {
            result = &mut download => panic!("download should still be streaming: {result:?}"),
            result = tokio::time::timeout(std::time::Duration::from_secs(5), received) => { result.unwrap().unwrap(); }
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    server.abort();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
