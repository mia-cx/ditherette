//! Host-wide benchmark exclusion and owned-child cleanup.
//!
//! Dropping the last inherited descriptor releases the OS lease. Never unlink
//! the lock files: doing so would let another process lock a different inode.

use std::{io, process::Command};

pub const LEASE_PATH: &str = "/tmp/ditherette-bench.lock";
const EXECUTION_PATH: &str = "/tmp/ditherette-bench.execution.lock";
pub const LEASE_FD_ENV: &str = "DITHERETTE_BENCH_LEASE_FD";
pub const QUIET_ENV: &str = "DITHERETTE_BENCH_QUIET";

/// Check the coordinator's explicit attestation after it drains agents and builds.
/// This does not inspect or certify unrelated host activity.
pub fn require_quiet() -> io::Result<()> {
    if std::env::var(QUIET_ENV).as_deref() != Ok("1") {
        return Err(io::Error::other(format!(
            "measurement requires a quiet phase; drain agents, builds and tests, then set {QUIET_ENV}=1 (see crates/ditherette-bench/EXECUTION.md)"
        )));
    }
    Ok(())
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::{
        fs::{File, OpenOptions},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{
                fs::{MetadataExt, OpenOptionsExt},
                process::CommandExt,
            },
        },
        process::{Child, ChildStdout, ExitStatus, Stdio},
        sync::atomic::{AtomicI32, Ordering},
    };

    static OWNED_CHILD: AtomicI32 = AtomicI32::new(0);

    /// An OS lease that can be lent to sequential child executables.
    pub struct Lease(File);

    impl Lease {
        /// Acquire the host lease for an external coordinator. Contention fails immediately.
        pub fn exclusive() -> io::Result<Self> {
            if std::env::var_os(LEASE_FD_ENV).is_some() {
                return Err(io::Error::other(
                    "a lease coordinator cannot run inside another lease",
                ));
            }
            install_signal_handlers()?;
            Ok(Self(lock(LEASE_PATH)?))
        }

        fn acquire_or_inherit() -> io::Result<Self> {
            install_signal_handlers()?;
            let Some(value) = std::env::var_os(LEASE_FD_ENV) else {
                return Ok(Self(lock(LEASE_PATH)?));
            };
            let fd: i32 = value
                .to_str()
                .and_then(|value| value.parse().ok())
                .filter(|fd| *fd >= 3)
                .ok_or_else(|| io::Error::other("invalid inherited benchmark lease descriptor"))?;
            // SAFETY: fcntl validates the external descriptor. The duplicate is owned here.
            let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 3) };
            if duplicate < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: fcntl returned a fresh owned descriptor.
            let file = unsafe { File::from_raw_fd(duplicate) };
            let actual = file.metadata()?;
            let expected = std::fs::metadata(LEASE_PATH)?;
            if !actual.is_file() || actual.dev() != expected.dev() || actual.ino() != expected.ino()
            {
                return Err(io::Error::other(
                    "inherited benchmark lease points to the wrong file",
                ));
            }
            // Reassert the lease on the shared open-file description. An unheld
            // descriptor cannot bypass a competing owner's lock.
            flock(&file, LEASE_PATH)?;
            Ok(Self(file))
        }

        /// Start one owned child with an inherited lease and a parent-liveness pipe.
        /// Drop terminates and reaps that child before releasing its lease.
        pub fn spawn(&self, mut command: Command) -> io::Result<OwnedChild> {
            if OWNED_CHILD.load(Ordering::SeqCst) != 0 {
                return Err(io::Error::other(
                    "only one owned benchmark child may run at a time",
                ));
            }
            // Keep a dedicated duplicate alive with the child. This also avoids
            // replacing an unrelated descriptor through a fixed dup2 target.
            let inherited = self.0.try_clone()?;
            let fd = inherited.as_raw_fd();
            command
                .env(LEASE_FD_ENV, fd.to_string())
                .stdin(Stdio::piped());
            let mask = SignalMask::block()?;
            // SAFETY: this closure only calls async-signal-safe functions after fork.
            unsafe {
                command.pre_exec(move || {
                    if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                    let mut signals = std::mem::zeroed();
                    signal_set(&mut signals);
                    let status =
                        libc::pthread_sigmask(libc::SIG_UNBLOCK, &signals, std::ptr::null_mut());
                    if status != 0 {
                        return Err(io::Error::from_raw_os_error(status));
                    }
                    Ok(())
                });
            }
            let mut child = command.spawn()?;
            let stdin = child.stdin.take();
            OWNED_CHILD.store(child.id() as i32, Ordering::SeqCst);
            drop(mask);
            Ok(OwnedChild {
                child,
                status: None,
                _lease: inherited,
                _stdin: stdin,
            })
        }
    }

    /// One benchmark execution, even when several children inherit a coordinator's lease.
    pub struct BenchmarkGuard {
        pub lease: Lease,
        _execution: File,
    }

    impl BenchmarkGuard {
        /// Acquire both lease ownership and independent execution ownership.
        pub fn acquire() -> io::Result<Self> {
            let lease = Lease::acquire_or_inherit()?;
            let execution = lock(EXECUTION_PATH)?;
            Ok(Self {
                lease,
                _execution: execution,
            })
        }
    }

    /// A direct child whose shutdown completes before its owner drops the lease.
    pub struct OwnedChild {
        child: Child,
        status: Option<ExitStatus>,
        _lease: File,
        _stdin: Option<std::process::ChildStdin>,
    }

    impl OwnedChild {
        /// OS process identity retained until this owned child is reaped.
        pub fn id(&self) -> u32 {
            self.child.id()
        }

        /// Take captured transport output while retaining child ownership.
        pub fn take_stdout(&mut self) -> Option<ChildStdout> {
            self.child.stdout.take()
        }

        /// Wait for the child and its transport cleanup to finish.
        pub fn wait(&mut self) -> io::Result<ExitStatus> {
            if let Some(status) = self.status {
                return Ok(status);
            }
            let status = self.child.wait()?;
            self.status = Some(status);
            let _ = OWNED_CHILD.compare_exchange(
                self.child.id() as i32,
                0,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            Ok(status)
        }
    }

    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if self.status.is_some() {
                return;
            }
            // SAFETY: this PID is our unreaped child, so it cannot be reused.
            unsafe {
                libc::kill(self.child.id() as i32, libc::SIGTERM);
            }
            if let Err(error) = self.wait() {
                eprintln!("failed to reap owned benchmark child: {error}");
            }
        }
    }

    fn lock(path: &str) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o666)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("benchmark lock must be a regular file"));
        }
        flock(&file, path)?;
        Ok(file)
    }

    fn flock(file: &File, path: &str) -> io::Result<()> {
        // SAFETY: file owns a valid descriptor; flock does not mutate Rust memory.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return Err(io::Error::new(error.kind(), format!(
                "benchmark lease busy or unavailable at {path}: {error}; wait for its owner, never delete the lock"
            )));
        }
        Ok(())
    }

    unsafe fn signal_set(set: *mut libc::sigset_t) {
        libc::sigemptyset(set);
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            libc::sigaddset(set, signal);
        }
    }

    struct SignalMask(libc::sigset_t);
    impl SignalMask {
        fn block() -> io::Result<Self> {
            // SAFETY: both signal sets are initialized before use.
            unsafe {
                let mut set = std::mem::zeroed();
                let mut previous = std::mem::zeroed();
                signal_set(&mut set);
                let status = libc::pthread_sigmask(libc::SIG_BLOCK, &set, &mut previous);
                if status != 0 {
                    return Err(io::Error::from_raw_os_error(status));
                }
                Ok(Self(previous))
            }
        }
    }
    impl Drop for SignalMask {
        fn drop(&mut self) {
            // SAFETY: restore the calling thread's previous initialized signal mask.
            unsafe {
                libc::pthread_sigmask(libc::SIG_SETMASK, &self.0, std::ptr::null_mut());
            }
        }
    }

    extern "C" fn interrupted(signal: i32) {
        let pid = OWNED_CHILD.load(Ordering::SeqCst);
        // SAFETY: kill, waitpid and _exit are async-signal-safe. Keep the lease
        // open until our child finishes its own browser cleanup.
        unsafe {
            if pid > 0 {
                libc::kill(pid, libc::SIGTERM);
                while libc::waitpid(pid, std::ptr::null_mut(), 0) < 0 {
                    if io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                        break;
                    }
                }
            }
            libc::_exit(128 + signal);
        }
    }

    fn install_signal_handlers() -> io::Result<()> {
        // SAFETY: sigaction copies initialized action data and installs a C handler.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = interrupted as *const () as usize;
            signal_set(&mut action.sa_mask);
            for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
                if libc::sigaction(signal, &action, std::ptr::null_mut()) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
        }
        Ok(())
    }
}

pub use platform::{BenchmarkGuard, Lease, OwnedChild};

#[cfg(not(unix))]
mod platform {
    use super::*;
    use std::process::{ChildStdout, ExitStatus};

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "exclusive benchmark execution currently requires Unix flock and process signals",
        )
    }

    pub struct Lease;
    impl Lease {
        pub fn exclusive() -> io::Result<Self> {
            Err(unsupported())
        }
        pub fn spawn(&self, _command: Command) -> io::Result<OwnedChild> {
            Err(unsupported())
        }
    }
    pub struct BenchmarkGuard {
        pub lease: Lease,
    }
    impl BenchmarkGuard {
        pub fn acquire() -> io::Result<Self> {
            Err(unsupported())
        }
    }
    pub struct OwnedChild;
    impl OwnedChild {
        pub fn id(&self) -> u32 {
            0
        }

        pub fn take_stdout(&mut self) -> Option<ChildStdout> {
            None
        }
        pub fn wait(&mut self) -> io::Result<ExitStatus> {
            Err(unsupported())
        }
    }
}
