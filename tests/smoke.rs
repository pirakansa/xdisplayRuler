use std::process::Command;

#[test]
fn it_prints_a_display_ruler() {
    let output = Command::new(env!("CARGO_BIN_EXE_display-ruler"))
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "0         1         2         3         4         5         6         7\n",
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890\n"
        )
    );
}
