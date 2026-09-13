#![cfg(unix)]
//! Process fixtures only. These tests never execute a benchmark workload.

use ditherette_bench::lease::{require_quiet, BenchmarkGuard, Lease, LEASE_FD_ENV, QUIET_ENV};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

fn fixture(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--ignored", "--exact", "child_fixture", "--nocapture"])
        .env("DITHERETTE_LEASE_FIXTURE", mode)
        .env_remove(LEASE_FD_ENV)
        .env_remove(QUIET_ENV);
    command
}

fn ready(stdout: impl std::io::Read) {
    let mut lines = BufReader::new(stdout).lines();
    assert!(lines.any(|line| line.unwrap() == "READY"));
}

#[test]
fn exclusive_lifecycle_across_processes() {
    assert!(!fixture("quiet").output().unwrap().status.success());
    let lease = Lease::exclusive().unwrap();

    let blocked = fixture("try").current_dir("/").output().unwrap();
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("benchmark lease busy"));

    let mut command = fixture("hold");
    command.stdout(Stdio::piped());
    let mut borrower = lease.spawn(command).unwrap();
    ready(borrower.take_stdout().unwrap());
    // The borrowed lease cannot authorize a concurrent or nested execution.
    drop(borrower);

    // The outer lease remains owned between sequential executions.
    assert!(!fixture("try").output().unwrap().status.success());
    let next = fixture("try");
    let mut finished = lease.spawn(next).unwrap();
    assert!(finished.wait().unwrap().success());
    let mut later = fixture("hold");
    later.stdout(Stdio::piped());
    let mut active = lease.spawn(later).unwrap();
    ready(active.take_stdout().unwrap());
    drop(finished);
    assert!(lease.spawn(fixture("try")).is_err());
    drop(active);
    concurrent_spawn_keeps_one_child(&lease);
    assert!(lease
        .spawn(Command::new("/no-such-ditherette-fixture"))
        .is_err());
    assert!(lease
        .spawn(fixture("try"))
        .unwrap()
        .wait()
        .unwrap()
        .success());
    let browser_tests = browser_fixture();
    assert!(lease
        .spawn(browser_tests)
        .unwrap()
        .wait()
        .unwrap()
        .success());
    drop(lease);
    assert!(fixture("try").output().unwrap().status.success());

    // A signal must wait for owned-child cleanup before releasing the lease.
    let mut supervisor = fixture("supervisor")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ready(supervisor.stdout.take().unwrap());
    // SAFETY: supervisor is our live, unreaped fixture child.
    assert_eq!(
        unsafe { libc::kill(supervisor.id() as i32, libc::SIGTERM) },
        0
    );
    assert!(!supervisor.wait().unwrap().success());
    assert!(fixture("try").output().unwrap().status.success());

    let marker = std::env::temp_dir().join(format!("ditherette-cleanup-{}", std::process::id()));
    let mut supervisor = fixture("browser-supervisor")
        .env("DITHERETTE_CLEANUP_MARKER", &marker)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ready(supervisor.stdout.take().unwrap());
    // SAFETY: supervisor is our unreaped fixture child.
    assert_eq!(
        unsafe { libc::kill(supervisor.id() as i32, libc::SIGTERM) },
        0
    );
    assert!(!supervisor.wait().unwrap().success());
    assert_eq!(
        std::fs::read_to_string(&marker).unwrap(),
        "owned browser exited"
    );
    std::fs::remove_file(marker).unwrap();
    assert!(fixture("try").output().unwrap().status.success());
}

fn concurrent_spawn_keeps_one_child(lease: &Lease) {
    use std::{os::unix::process::CommandExt, sync::Barrier};
    let barrier = Barrier::new(2);
    let children = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let barrier = &barrier;
                scope.spawn(move || {
                    let mut command = Command::new("sh");
                    command.args(["-c", "read value"]);
                    unsafe {
                        command.pre_exec(|| {
                            libc::usleep(100_000);
                            Ok(())
                        });
                    }
                    barrier.wait();
                    lease.spawn(command)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let started = children.iter().filter(|child| child.is_ok()).count();
    for error in children.iter().filter_map(|child| child.as_ref().err()) {
        assert!(error.to_string().contains("only one owned benchmark child"));
    }
    drop(children);
    assert_eq!(started, 1, "concurrent spawn must retain one owned child");
}

fn browser_fixture() -> Command {
    let mut command = Command::new("node");
    command
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../scripts/benchmark-transport.test.mjs"
        ))
        .env("DITHERETTE_BENCH_TRANSPORT", "1")
        .env(QUIET_ENV, "1");
    command
}

#[test]
#[ignore = "invoked only as a controlled subprocess"]
fn child_fixture() {
    match std::env::var("DITHERETTE_LEASE_FIXTURE").unwrap().as_str() {
        "quiet" => require_quiet().unwrap(),
        "try" => {
            let _guard = BenchmarkGuard::acquire().unwrap();
        }
        "hold" => {
            let guard = BenchmarkGuard::acquire().unwrap();
            // Opening the independent execution lock again must fail, even
            // while this process owns the inherited host lease.
            assert!(BenchmarkGuard::acquire().is_err());
            assert!(Lease::exclusive().is_err());
            println!("READY");
            std::io::stdout().flush().unwrap();
            std::thread::park();
            drop(guard);
        }
        "supervisor" => {
            let guard = BenchmarkGuard::acquire().unwrap();
            let mut command = Command::new("node");
            command.args(["-e", "process.on('SIGTERM', () => setTimeout(() => process.exit(0), 100)); console.log('READY'); setInterval(() => {}, 1000);"])
                .stdout(Stdio::piped());
            let mut child = guard.lease.spawn(command).unwrap();
            ready(child.take_stdout().unwrap());
            println!("READY");
            std::io::stdout().flush().unwrap();
            child.wait().unwrap();
        }
        "browser-supervisor" => {
            let guard = BenchmarkGuard::acquire().unwrap();
            let mut command = browser_fixture();
            command.arg("--interrupt-fixture").stdout(Stdio::piped());
            let mut child = guard.lease.spawn(command).unwrap();
            ready(child.take_stdout().unwrap());
            println!("READY");
            std::io::stdout().flush().unwrap();
            child.wait().unwrap();
        }
        mode => panic!("unknown fixture mode {mode}"),
    }
}
