// given an InterpolatedCommand, return a WorkerResult

use crate::obj::InterpolatedCommand;
use crate::daemon::WorkerResult;
use std::sync::mpsc::Receiver;

pub trait Worker {
    /// Return the available worker capacity. This should not change.
    fn capacity(&self) -> Option<usize>;

    /// List queues associated with the worker
    fn queues(&self) -> Vec<String>;

    /// Execute a given command and return a result
    ///
    /// How does a worker return the result to the daemon?
    /// Callback would require a mutable reference to the daemon itself
    fn exec(&mut self, command: &InterpolatedCommand) -> WorkerResult;
}