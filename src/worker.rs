// given an InterpolatedCommand, return a WorkerResult

use crate::obj::InterpolatedCommand;
use crate::daemon::WorkerResult;
use std::sync::mpsc::{Sender};
use crate::daemon::DaemonRequest;
use crate::obj::*;

#[derive(Clone)]
pub struct WorkerReplier<'a> {
    wid: WorkerId,
    qid: CommandId,
    tid: ThreadId,
    sid: StepId,
    sender: Sender<DaemonRequest<'a>>
}

impl <'a>WorkerReplier<'a> {
    pub fn new(
        wid: WorkerId,
        qid: CommandId,
        tid: ThreadId,
        sid: StepId,
        sender: Sender<DaemonRequest<'a>>
    ) -> Self {
        WorkerReplier {
            wid,
            qid,
            tid,
            sid, sender
        }
    }

    pub fn reply(&mut self, x: WorkerResult) {
        self.sender.send(DaemonRequest::Finished(self.wid.clone(), self.qid.clone(), self.tid.clone(), self.sid, x));
    }
}

pub trait Worker {
    /// Return the available worker capacity. This should not change.
    fn capacity(&self) -> Option<usize>;

    /// List queues associated with the worker
    fn queues(&self) -> Vec<String>;

    fn exec(&mut self, command: &InterpolatedCommand) -> WorkerResult;

    /// Execute a given command and return a result
    ///
    /// How does a worker return the result to the daemon?
    /// Callback would require a mutable reference to the daemon itself
    fn put(&mut self, command: &InterpolatedCommand, result_cb: WorkerReplier) {
        result_cb.clone().reply(self.exec(command))
    }
}