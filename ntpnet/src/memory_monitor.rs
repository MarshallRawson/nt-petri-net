use defer::defer;
#[cfg(not(target_os = "macos"))]
use procinfo::pid::statm_self;
#[cfg(target_os = "macos")]
use memory_stats::memory_stats;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use plotmux::plotsink::PlotSink;

#[cfg(not(target_os = "macos"))]
fn plot_mem(t0: Instant, plot_sink: &mut PlotSink) {
    if let Ok(status) = statm_self() {
        let now = (Instant::now() - t0).as_secs_f64();
        plot_sink.plot_series_2d("memory usage", "total (B)", now, status.size as f64);
        plot_sink.plot_series_2d(
            "memory usage",
            "non-swapped (B)",
            now,
            status.resident as f64,
        );
        plot_sink.plot_series_2d(
            "memory usage",
            "shared (B)",
            now,
            status.share as f64,
        );
        plot_sink.plot_series_2d(
            "memory usage",
            "executable (B)",
            now,
            status.text as f64,
        );
        plot_sink.plot_series_2d(
            "memory usage",
            "stack + heap (B)",
            now,
            status.data as f64,
        );
    }
}

#[cfg(target_os = "macos")]
fn plot_mem(t0: Instant, plot_sink: &mut PlotSink) {
    if let Some(usage) = memory_stats() {
        let now = (Instant::now() - t0).as_secs_f64();
        plot_sink.plot_series_2d(
            "memory usage",
            "physical memory (B)",
            now,
            usage.physical_mem as f64
        );
        plot_sink.plot_series_2d(
            "memory usag",
            "virtual memory (B)",
            now,
            usage.virtual_mem as f64
        );
    }
}

pub fn memory_monitor(period: f64, mut plot_sink: PlotSink) -> impl Drop {
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();
    let t = thread::Builder::new()
        .name("memory_monitor".into())
        .spawn(move || {
            let t0 = Instant::now();
            let &(ref lock, ref cvar) = &*pair;
            let mut exit = lock.lock().unwrap();
            loop {
                let result = cvar
                    .wait_timeout(exit, Duration::from_secs_f64(period))
                    .unwrap();
                exit = result.0;
                if *exit {
                    break;
                }
                plot_mem(t0, &mut plot_sink);
            }
        })
        .expect("unable to spawn memory monitor thread");
    defer(move || {
        let &(ref lock, ref cvar) = &*pair2;
        let mut exit = lock.lock().unwrap();
        *exit = true;
        cvar.notify_one();
        t.join().expect("unable to join memory monitor thread")
    })
}
