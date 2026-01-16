use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// A lock that can be used to ensure tests run in serial rather than in parallel.
/// For devnet tests, this is useful, because our tests individually query for UTxOs and might try to
/// spend the same UTxO at the same time.
pub struct TestLock<'a>(&'a AtomicBool);

static LOCK: AtomicBool = AtomicBool::new(false);

impl<'a> TestLock<'a> {
    /// Wait for the lock to be available and then lock it.
    pub fn wait_and_lock() -> TestLock<'a> {
        while LOCK
            .compare_exchange_weak(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            thread::yield_now();
        }

        // Sleep a full slot in devnet context
        std::thread::sleep(std::time::Duration::from_millis(100));

        Self(&LOCK)
    }
}

impl<'a> Drop for TestLock<'a> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst)
    }
}
