pub mod cpu;
pub mod io;

pub const MAGIC_NUMBER: i32 = u16::MAX as i32;

use std::sync::*;
use std::collections::*;

#[derive(Debug)]
/// Thread-safe queue that blocks de_q on empty
pub struct BlockingQueue<T> {
    q: Mutex<VecDeque<T>>,
    cv: Condvar,
}
impl<T> BlockingQueue<T> {
    /// Create empty blocking queue
    pub fn new() -> Self {
        Self {
            q: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
        }
    }
    /// push input on back of queue
    /// - unrecoverable if lock fails so just unwrap
    pub fn en_q(&self, t:T) {
        let mut lq = self.q.lock().unwrap();
        lq.push_back(t);
        self.cv.notify_one();
    }
    /// pop element from front of queue
    /// - unrecoverable if lock fails so just unwrap
    /// - same for condition variable
    pub fn de_q(&self) -> T {
        let mut lq = self.q.lock().unwrap();
        while lq.len() == 0 {
            lq = self.cv.wait(lq).unwrap();
        }
        lq.pop_front().unwrap()
    }
    /// return number of elements in queue
    pub fn len(&self) -> usize {
        self.q.lock().unwrap().len()
    }
}