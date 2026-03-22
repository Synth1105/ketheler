use std::process::Command;

#[test]
fn debug_prints_handle_debug_output() {
    let exe = env!("CARGO_BIN_EXE_debug_helper");
    let output = Command::new(exe).output().expect("run debug helper");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert_eq!(stdout, "Echo(7)\n");
}
