mod epoll;
mod poll;
mod select;

use alloc::{sync::Arc, vec::Vec};
use core::task::Context;

use axpoll::{IoEvents, PollTable, Pollable};

pub use self::{epoll::*, poll::*, select::*};
use crate::file::FileLike;

struct FdPollSet(pub Vec<(Arc<dyn FileLike>, IoEvents)>);
impl Pollable for FdPollSet {
    fn poll(&self) -> IoEvents {
        unreachable!()
    }

    fn poll_wait(&self, _events: IoEvents, table: &mut PollTable) {
        for (file, events) in &self.0 {
            file.poll_wait(*events, table);
        }
    }

    fn register(&self, context: &mut Context<'_>, _events: IoEvents) {
        for (file, events) in &self.0 {
            file.register(context, *events);
        }
    }
}
