//! External coordinator for sequential, prebuilt benchmark executables.

use ditherette_bench::lease::{Lease, QUIET_ENV};
use std::{
    env, io,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(5)
        }
    }
}

fn run() -> io::Result<u8> {
    let mut args = env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--quiet"))
        || args.next().as_deref() != Some(std::ffi::OsStr::new("--"))
    {
        return Err(io::Error::other("usage: ditherette-bench-lease --quiet -- COMMAND [ARGS...]; --quiet attests agents, builds and tests have drained"));
    }
    let executable = args
        .next()
        .ok_or_else(|| io::Error::other("expected a prebuilt command"))?;
    let lease = Lease::exclusive()?;
    let mut command = Command::new(executable);
    command.args(args).env(QUIET_ENV, "1");
    let status = lease.spawn(&mut command)?.wait()?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1))
}
