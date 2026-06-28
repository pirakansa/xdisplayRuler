use std::process::Command;

#[test]
fn it_prints_an_empty_display_state_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_display-ruler"))
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "display-ruler\n",
            "backend: in-memory\n",
            "outputs: 0\n",
            "windows: 0\n",
            "focused: none\n",
            "top: none\n"
        )
    );
}

#[test]
fn it_prints_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_display-ruler"))
        .arg("--help")
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}
