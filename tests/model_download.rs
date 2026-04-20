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
}
