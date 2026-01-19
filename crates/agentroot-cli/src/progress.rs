//! Progress reporting with ETA

use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Simple progress reporter for CLI commands
pub struct ProgressReporter {
    total: usize,
    processed: usize,
    start_time: Instant,
    last_update: Instant,
    show_percentage: bool,
}

impl ProgressReporter {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            processed: 0,
            start_time: Instant::now(),
            last_update: Instant::now(),
            show_percentage: true,
        }
    }

    pub fn with_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn set_message(&self, msg: &str) {
        if self.show_percentage && self.total > 0 {
            let percentage = (self.processed as f64 / self.total as f64 * 100.0) as u32;
            eprint!(
                "\r{} [{}%] ({}/{})    ",
                msg, percentage, self.processed, self.total
            );
        } else {
            eprint!("\r{:<50}", msg);
        }
        io::stderr().flush().ok();
    }

    #[allow(dead_code)]
    pub fn set_message_with_count(&self, msg: &str, count: usize) {
        if self.show_percentage && self.total > 0 {
            let percentage = (self.processed as f64 / self.total as f64 * 100.0) as u32;
            let eta = self.estimate_time_remaining();
            let eta_str = if let Some(eta) = eta {
                format!(" ETA: {}s", eta.as_secs())
            } else {
                String::new()
            };
            eprint!(
                "\r{} [{}%] ({}/{}) - {} items{}    ",
                msg, percentage, self.processed, self.total, count, eta_str
            );
        } else {
            eprint!("\r{} - {} items    ", msg, count);
        }
        io::stderr().flush().ok();
    }

    pub fn increment(&mut self) {
        self.processed += 1;
        self.last_update = Instant::now();
    }

    #[allow(dead_code)]
    pub fn increment_by(&mut self, amount: usize) {
        self.processed += amount;
        self.last_update = Instant::now();
    }

    #[allow(dead_code)]
    pub fn set_total(&mut self, total: usize) {
        self.total = total;
    }

    fn estimate_time_remaining(&self) -> Option<Duration> {
        if self.processed == 0 || self.total == 0 {
            return None;
        }

        let elapsed = self.start_time.elapsed();
        let rate = self.processed as f64 / elapsed.as_secs_f64();
        let remaining = self.total - self.processed;
        let eta_secs = remaining as f64 / rate;

        Some(Duration::from_secs_f64(eta_secs))
    }

    pub fn finish(&self) {
        let elapsed = self.start_time.elapsed();
        let rate = if elapsed.as_secs() > 0 {
            format!(" ({}/s)", self.processed / elapsed.as_secs() as usize)
        } else {
            String::new()
        };

        eprintln!(
            "\rDone ({}/{}) in {:.1}s{}                    ",
            self.processed,
            self.total,
            elapsed.as_secs_f64(),
            rate
        );
    }

    pub fn finish_with_message(&self, msg: &str) {
        let elapsed = self.start_time.elapsed();
        eprintln!(
            "\r{} ({}/{}) in {:.1}s                    ",
            msg,
            self.processed,
            self.total,
            elapsed.as_secs_f64()
        );
    }
}
