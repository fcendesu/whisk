use std::process::Command;

#[test]
fn missing_credentials_fail_without_dumping_environment() {
    let output = Command::new(env!("CARGO_BIN_EXE_media-spike"))
        .env_remove("LIVEKIT_URL")
        .env("LIVEKIT_TOKEN", "must-not-leak")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("invalid_input"), "{stderr}");
    assert!(!stderr.contains("must-not-leak"));
}

#[test]
fn invalid_endpoint_diagnostics_never_include_credentials() {
    let output = Command::new(env!("CARGO_BIN_EXE_media-spike"))
        .env(
            "LIVEKIT_URL",
            "ws://user:password@localhost:7880?access_token=url-secret",
        )
        .env("LIVEKIT_TOKEN", "token-secret")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let report: serde_json::Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(report["error"], "invalid_input");
    for secret in ["password", "url-secret", "token-secret"] {
        assert!(!stderr.contains(secret));
    }
}
