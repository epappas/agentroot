//! Progress reporting with ETA

use std::io::{self, Write};

/// Simple progress reporter for CLI commands
pub struct ProgressReporter {
    total: usize,
    processed: usize,
}

impl ProgressReporter {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            processed: 0,
        }
    }

    pub fn set_message(&self, msg: &str) {
        eprint!("\r{:<50}", msg);
        io::stderr().flush().ok();
    }

    pub fn increment(&mut self) {
        self.processed += 1;
    }

    pub fn finish(&self) {
        eprintln!("\rDone ({}/{})                    ", self.processed, self.total);
    }
}
