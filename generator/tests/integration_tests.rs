use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_generate_rust_scanner() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.json");
    let config_content = r#"{
        "scanner_name": "TestScanner",
        "patterns": [
            { "token": "NUM", "regex": "[0-9]+" },
            { "token": "ID", "regex": "[a-zA-Z_]+" }
        ]
    }"#;
    fs::write(&config_path, config_content).unwrap();
    let output_path = dir.path().join("scanner.rs");

    let project_root = std::env::current_dir().unwrap();
    let status = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg(&config_path)
        .arg("-o")
        .arg(&output_path)
        .current_dir(&project_root)
        .status()
        .unwrap();
    assert!(status.success());

    let status = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&output_path)
        .arg("-o")
        .arg(dir.path().join("scanner.rlib"))
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}
