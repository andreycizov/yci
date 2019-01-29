// given an InterpolatedCommand, return a WorkerResult

use crate::obj::InterpolatedCommand;
use crate::daemon::WorkerResult;
use crate::daemon::DaemonRequest;
use crate::obj::*;
use mio_extras::channel::Sender;

#[derive(Clone)]
pub struct WorkerReplier {
    wid: WorkerId,
    qid: CommandId,
    tid: ThreadId,
    sid: StepId,
    sender: Sender<DaemonRequest>,
}

impl WorkerReplier {
    pub fn new(
        wid: WorkerId,
        qid: CommandId,
        tid: ThreadId,
        sid: StepId,
        sender: Sender<DaemonRequest>,
    ) -> Self {
        WorkerReplier {
            wid,
            qid,
            tid,
            sid,
            sender,
        }
    }

    pub fn reply(&mut self, x: WorkerResult) {
        self.sender.send(DaemonRequest::Finished(self.wid.clone(), self.tid.clone(), self.sid, self.qid.clone(), x)).unwrap();
    }
}

unsafe impl Send for WorkerReplier {

}

pub trait Worker {
    /// Return the available worker capacity. This should not change.
    fn capacity(&self) -> Option<usize>;

    /// List queues associated with the worker
    fn queues(&self) -> Vec<CommandId>;

    /// Execute a given command and return a result
    ///
    /// How does a worker return the result to the daemon?
    /// Callback would require a mutable reference to the daemon itself
    fn put(&mut self, command: &InterpolatedCommand, result_cb: WorkerReplier);
}
