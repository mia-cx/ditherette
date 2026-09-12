//! Best-effort runtime controls for repeatable benchmark execution.

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use crate::error::BenchError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessPriority {
    Normal,
    High,
}

impl ProcessPriority {
    pub(crate) fn parse(value: &str) -> Result<Self, BenchError> {
        match value {
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            _ => Err(BenchError::Config(format!(
                "invalid --process-priority {value:?}; expected normal or high"
            ))),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeTuningReport {
    pub(crate) priority: ProcessPriority,
    pub(crate) priority_notes: Vec<String>,
    pub(crate) affinity_notes: Vec<String>,
}

pub(crate) fn tune_runtime(priority: ProcessPriority) -> RuntimeTuningReport {
    if priority == ProcessPriority::Normal {
        return RuntimeTuningReport {
            priority,
            priority_notes: vec!["normal process priority requested".to_owned()],
            affinity_notes: vec!["no CPU affinity changes requested".to_owned()],
        };
    }

    let mut priority_notes = Vec::new();
    let mut affinity_notes = Vec::new();
    apply_high_priority(&mut priority_notes);
    apply_performance_affinity(&mut affinity_notes);
    RuntimeTuningReport {
        priority,
        priority_notes,
        affinity_notes,
    }
}

pub(crate) fn preheat_cpu(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    let start = Instant::now();
    let mut state = 0x9e37_79b9_7f4a_7c15u64;
    while start.elapsed() < duration {
        for _ in 0..4096 {
            state = state.rotate_left(7) ^ state.wrapping_mul(0xbf58_476d_1ce4_e5b9);
            state = state.wrapping_add(0x94d0_49bb_1331_11ebu64);
        }
        black_box(state);
    }
}

#[cfg(target_os = "macos")]
fn apply_high_priority(notes: &mut Vec<String>) {
    // SAFETY: pthread_set_qos_class_self_np affects only the current thread and
    // accepts constant QoS values. setpriority targets the current process.
    unsafe {
        let qos_result =
            libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
        if qos_result == 0 {
            notes.push("requested macOS user-interactive QoS for the benchmark thread".to_owned());
        } else {
            notes.push(format!(
                "failed to request macOS user-interactive QoS: errno {}",
                qos_result
            ));
        }
        let nice_result = libc::setpriority(libc::PRIO_PROCESS, 0, -10);
        if nice_result == 0 {
            notes.push("raised process priority with nice -10".to_owned());
        } else {
            notes.push("could not raise nice priority; continuing with QoS hint".to_owned());
        }
    }
    notes.push(
        "macOS does not expose public P-core pinning; QoS is the P-core/boost hint".to_owned(),
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
fn apply_high_priority(notes: &mut Vec<String>) {
    // SAFETY: setpriority targets the current process.
    let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, -10) };
    if result == 0 {
        notes.push("raised process priority with nice -10".to_owned());
    } else {
        notes.push(
            "could not raise nice priority; run with elevated privileges for stronger priority"
                .to_owned(),
        );
    }
}

#[cfg(windows)]
fn apply_high_priority(notes: &mut Vec<String>) {
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority,
        HIGH_PRIORITY_CLASS, THREAD_PRIORITY_HIGHEST,
    };
    // SAFETY: Win32 calls target the current process/thread handles returned by the OS.
    unsafe {
        if SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS) != 0 {
            notes.push("set Windows process priority class to HIGH_PRIORITY_CLASS".to_owned());
        } else {
            notes.push("failed to set Windows HIGH_PRIORITY_CLASS".to_owned());
        }
        if SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) != 0 {
            notes.push(
                "set Windows benchmark thread priority to THREAD_PRIORITY_HIGHEST".to_owned(),
            );
        } else {
            notes.push("failed to set Windows benchmark thread priority".to_owned());
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn apply_high_priority(notes: &mut Vec<String>) {
    notes.push("high process priority is unsupported on this platform".to_owned());
}

#[cfg(target_os = "linux")]
fn apply_performance_affinity(notes: &mut Vec<String>) {
    let Some(cpus) = linux_performance_cpus() else {
        notes.push("no Linux cpu_capacity data found; leaving CPU affinity unchanged".to_owned());
        return;
    };
    if cpus.is_empty() {
        notes.push("Linux cpu_capacity data was empty; leaving CPU affinity unchanged".to_owned());
        return;
    }

    // SAFETY: cpu_set_t is initialized before use and sched_setaffinity targets
    // pid 0, the current process.
    let result = unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        for cpu in &cpus {
            libc::CPU_SET(*cpu, &mut set);
        }
        libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set)
    };
    if result == 0 {
        notes.push(format!(
            "pinned process to Linux max-capacity CPUs: {}",
            cpus.iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    } else {
        notes.push("failed to set Linux CPU affinity to max-capacity CPUs".to_owned());
    }
}

#[cfg(target_os = "linux")]
fn linux_performance_cpus() -> Option<Vec<usize>> {
    use std::fs;
    let mut capacities = Vec::new();
    for entry in fs::read_dir("/sys/devices/system/cpu").ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(cpu_name) = name.strip_prefix("cpu") else {
            continue;
        };
        let Ok(cpu) = cpu_name.parse::<usize>() else {
            continue;
        };
        let capacity_path = entry.path().join("cpu_capacity");
        let Ok(capacity) = fs::read_to_string(capacity_path) else {
            continue;
        };
        let Ok(capacity) = capacity.trim().parse::<usize>() else {
            continue;
        };
        capacities.push((cpu, capacity));
    }
    let max_capacity = capacities.iter().map(|(_, capacity)| *capacity).max()?;
    Some(
        capacities
            .into_iter()
            .filter_map(|(cpu, capacity)| (capacity == max_capacity).then_some(cpu))
            .collect(),
    )
}

#[cfg(all(unix, not(target_os = "linux")))]
fn apply_performance_affinity(notes: &mut Vec<String>) {
    notes.push("performance-core CPU affinity is unavailable on this platform".to_owned());
}

#[cfg(windows)]
fn apply_performance_affinity(notes: &mut Vec<String>) {
    notes.push("performance-core CPU affinity is not detected on Windows yet".to_owned());
}

#[cfg(not(any(unix, windows)))]
fn apply_performance_affinity(notes: &mut Vec<String>) {
    notes.push("CPU affinity is unsupported on this platform".to_owned());
}
