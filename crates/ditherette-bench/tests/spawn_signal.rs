#![cfg(target_os = "linux")]
use ditherette_bench::lease::{Lease, LEASE_FD_ENV, QUIET_ENV};
use std::{
    fs::File,
    io::{Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Command, Stdio},
};

#[test]
fn interruption_during_spawn_reaps_the_child() {
    let mut previous = 0;
    assert_eq!(
        unsafe { libc::prctl(libc::PR_GET_CHILD_SUBREAPER, &mut previous) },
        0
    );
    assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
    let mut supervisor = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "spawn_supervisor", "--nocapture"])
        .env_remove(LEASE_FD_ENV)
        .env_remove(QUIET_ENV)
        .env("DITHERETTE_SPAWN_SIGNAL_FIXTURE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = std::io::BufReader::new(supervisor.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        line.clear();
        assert!(std::io::BufRead::read_line(&mut stdout, &mut line).unwrap() > 0);
        if line.trim_end().ends_with("FORKING") {
            break;
        }
    }
    let mut pid = [0u8; 4];
    stdout.read_exact(&mut pid).unwrap();
    let pid = i32::from_ne_bytes(pid);
    assert!(pid > 0);
    assert_eq!(
        unsafe {
            libc::syscall(
                libc::SYS_tgkill,
                supervisor.id(),
                supervisor.id(),
                libc::SIGTERM,
            )
        },
        0
    );
    std::thread::sleep(std::time::Duration::from_millis(100));
    supervisor.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(!supervisor.wait().unwrap().success());
    let mut status = 0;
    let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    let reaped_by_owner =
        waited == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD);
    if waited == 0 {
        assert_eq!(unsafe { libc::kill(pid, libc::SIGTERM) }, 0);
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    }
    assert_eq!(
        unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, previous) },
        0
    );
    assert!(
        reaped_by_owner,
        "supervisor exited without reaping its forked child"
    );
    drop(Lease::exclusive().unwrap());
}

#[test]
#[ignore = "invoked only as a controlled subprocess"]
fn spawn_supervisor() {
    assert_eq!(
        std::env::var("DITHERETTE_SPAWN_SIGNAL_FIXTURE").as_deref(),
        Ok("1")
    );
    let lease = Lease::exclusive().unwrap();
    let release = File::open("/dev/stdin").unwrap();
    let release_fd = release.as_raw_fd();
    println!("FORKING");
    std::io::stdout().flush().unwrap();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut command = Command::new("sh");
            command.args(["-c", "read value"]).stdout(Stdio::inherit());
            unsafe {
                command.pre_exec(move || {
                    let pid = libc::getpid().to_ne_bytes();
                    if libc::write(libc::STDOUT_FILENO, pid.as_ptr().cast(), pid.len())
                        != pid.len() as isize
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                    let mut byte = 0u8;
                    if libc::read(release_fd, (&mut byte as *mut u8).cast(), 1) != 1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            lease.spawn(command).unwrap().wait().unwrap();
        });
        loop {
            std::thread::park();
        }
    });
}
