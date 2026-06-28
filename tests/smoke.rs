use std::process::Command;

#[test]
fn it_prints_hello_world() {
    let output = Command::new(env!("CARGO_BIN_EXE_cli-template"))
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "Hello, world!\n");
}
