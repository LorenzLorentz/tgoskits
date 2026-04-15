//! A library for polling I/O events and waking up blocked tasks.

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

use alloc::{sync::Arc, task::Wake};
use core::task::{Context, Waker};

use bitflags::bitflags;
use linux_raw_sys::general::*;
use spin::Mutex;

bitflags! {
    /// I/O events.
    #[derive(Debug, Clone, Copy)]
    pub struct IoEvents: u32 {
        /// Available for read
        const IN     = POLLIN;
        /// Urgent data for read
        const PRI    = POLLPRI;
        /// Available for write
        const OUT    = POLLOUT;

        /// Error condition
        const ERR    = POLLERR;
        /// Hang up
        const HUP    = POLLHUP;
        /// Invalid request
        const NVAL   = POLLNVAL;

        /// Equivalent to [`IN`](Self::IN)
        const RDNORM = POLLRDNORM;
        /// Priority band data can be read
        const RDBAND = POLLRDBAND;
        /// Equivalent to [`OUT`](Self::OUT)
        const WRNORM = POLLWRNORM;
        /// Priority data can be written
        const WRBAND = POLLWRBAND;

        /// Message
        const MSG    = POLLMSG;
        /// Remove
        const REMOVE = POLLREMOVE;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        const RDHUP  = POLLRDHUP;

        /// Events that are always polled even without specifying them.
        const ALWAYS_POLL = Self::ERR.bits() | Self::HUP.bits();
    }
}

/// Waiter object used by [`PollWaitQueue`].
pub trait PollWaiter: Send + Sync {
    /// Wake the blocked task represented by this waiter.
    fn wake(&self);
}

/// A queue of task-native waiters.
pub struct PollWaitQueue(Mutex<alloc::vec::Vec<Arc<dyn PollWaiter>>>);

impl Default for PollWaitQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PollWaitQueue {
    /// Creates a new empty queue.
    pub const fn new() -> Self {
        Self(Mutex::new(alloc::vec::Vec::new()))
    }

    /// Registers the current waiter from the poll table.
    pub fn wait(&self, table: &mut PollTable) {
        self.0.lock().push(table.waiter.clone());
    }

    /// Registers a legacy [`Waker`] directly.
    pub fn register_waker(&self, waker: &Waker) {
        self.0.lock().push(Arc::new(WakerWaiter(waker.clone())));
    }

    /// Legacy compatibility wrapper.
    pub fn register(&self, waker: &Waker) {
        self.register_waker(waker);
    }

    /// Wakes all registered waiters.
    pub fn wake(&self) -> usize {
        let entries = {
            let mut guard = self.0.lock();
            if guard.is_empty() {
                return 0;
            }
            core::mem::take(&mut *guard)
        };
        let len = entries.len();
        for waiter in entries {
            waiter.wake();
        }
        len
    }
}

/// Backward-compatible alias name kept for existing users while the codebase
/// migrates away from the old Waker-based API.
pub type PollSet = PollWaitQueue;

/// Registration context used during one blocking poll attempt.
pub struct PollTable {
    waiter: Arc<dyn PollWaiter>,
}

impl PollTable {
    /// Creates a new poll table for the given waiter.
    pub fn new(waiter: Arc<dyn PollWaiter>) -> Self {
        Self { waiter }
    }

    /// Creates a poll table backed by a legacy [`Waker`].
    pub fn from_waker(waker: &Waker) -> Self {
        Self::new(Arc::new(WakerWaiter(waker.clone())))
    }
}

/// Trait for types that can be polled for I/O events.
pub trait Pollable {
    /// Polls for current I/O events.
    fn poll(&self) -> IoEvents;

    /// Registers the current waiter for the given events.
    fn poll_wait(&self, events: IoEvents, table: &mut PollTable) {
        let waker = Waker::from(Arc::new(PollWaiterWake(table.waiter.clone())));
        let mut context = Context::from_waker(&waker);
        self.register(&mut context, events);
    }

    /// Legacy Waker-based registration entry point kept as a compatibility
    /// layer while the tree migrates to [`Self::poll_wait`].
    fn register(&self, _context: &mut Context<'_>, _events: IoEvents) {}
}

struct WakerWaiter(Waker);

impl PollWaiter for WakerWaiter {
    fn wake(&self) {
        self.0.wake_by_ref();
    }
}

struct PollWaiterWake(Arc<dyn PollWaiter>);

impl Wake for PollWaiterWake {
    fn wake(self: Arc<Self>) {
        self.0.wake();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.wake();
    }
}
