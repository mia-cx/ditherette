#![cfg(unix)]
use ditherette_bench::lease::{LEASE_FD_ENV, QUIET_ENV};
use std::process::Command;

#[test]
fn wasm_help_uses_owned_text_transport_without_measurement() {
    for quiet in [true, false] {
        for args in [
            vec!["wasm-resize", "--help"],
            vec!["wasm-resize", "run", "color", "--help"],
        ] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_ditherette-bench"));
            command
                .args(args)
                .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
                .env_remove(LEASE_FD_ENV)
                .env_remove(QUIET_ENV);
            if quiet {
                command.env(QUIET_ENV, "1");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let help = String::from_utf8(output.stdout).unwrap();
            assert!(help.contains("Preparation:"));
            assert!(help.contains("Usage:"));
            assert!(!help.contains("\"kind\":\"start\""));
        }
    }
}
