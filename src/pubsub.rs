use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::collections::VecDeque;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct PubSubJob<QK: Clone + Eq + Hash + PartialEq, JK: Clone + Eq + Hash + PartialEq> {
    qk: QK,
    jk: JK,
}

#[derive(Clone, Debug)]
pub(crate) struct PubSubWorkerInfo<WK, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) key: WK,
    pub(crate) current: HashSet<PubSubJob<QK, JK>>,
    pub(crate) capacity: usize,
    pub(crate) queues: Vec<QK>,
}

#[derive(Clone, Debug)]
pub(crate) struct PubSub<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) workers: HashMap<WK, PubSubWorkerInfo<WK, QK, JK>>,
    pub(crate) queues_workers: HashMap<QK, HashSet<WK>>,
    pub(crate) jobs_workers: HashMap<JK, WK>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Assignment<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) action: Action,
    pub(crate) worker_key: WK,
    pub(crate) queue_key: QK,
    pub(crate) job_key: JK,
}

#[derive(Clone, Debug)]
pub(crate) struct MultiQueue<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) queues: HashMap<QK, VecDeque<JK>>,
    pub(crate) pubsub: PubSub<WK, QK, JK>,
    pub(crate) worker_queues: HashMap<WK, Vec<QK>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Action {
    Started,
    Done,
    Cancelled,
}

impl<QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> PubSubJob<QK, JK> {
    fn inst(qk: QK, jk: JK) -> Self {
        PubSubJob { qk, jk }
    }
}

impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> Assignment<WK, QK, JK> {
    pub fn new(action: Action, worker_key: WK, queue_key: QK, job_key: JK) -> Assignment<WK, QK, JK> {
        Assignment { action, worker_key, queue_key, job_key }
    }
}

impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> Default for PubSub<WK, QK, JK> {
    fn default() -> Self {
        PubSub {
            workers: HashMap::<WK, PubSubWorkerInfo<WK, QK, JK>>::default(),
            queues_workers: HashMap::<QK, HashSet<WK>>::default(),
            jobs_workers: HashMap::<JK, WK>::default(),
        }
    }
}

impl<WK, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> PubSubWorkerInfo<WK, QK, JK> {
    pub fn ready(&self) -> bool {
        return self.current.len() < self.capacity;
    }
}

impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> Default for MultiQueue<WK, QK, JK> {
    fn default() -> Self {
        MultiQueue {
            queues: HashMap::<QK, VecDeque<JK>>::default(),
            pubsub: PubSub::<WK, QK, JK>::default(),
            worker_queues: HashMap::<WK, Vec<QK>>::default(),
        }
    }
}


impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> PubSub<WK, QK, JK> {
    pub fn add(&mut self, key: WK, capacity: usize, queues: &Vec<QK>) {
        let worker = PubSubWorkerInfo { key, current: HashSet::<PubSubJob<QK, JK>>::default(), capacity, queues: queues.clone() };
        self.workers.insert(worker.key.clone(), worker.clone());

        if worker.ready() {
            self.worker_enable(&worker.key)
        }
    }

    fn worker_enable(&mut self, key: &WK) {
        let worker = self.workers.get(key).unwrap();

        for q in &worker.queues {
            if !self.queues_workers.contains_key(&q) {
                self.queues_workers.insert(q.clone(), HashSet::<WK>::default());
            }

            self.queues_workers.get_mut(&q).unwrap().insert(worker.key.clone());
        }
    }

    fn worker_disable(&mut self, key: &WK) {
        let worker = self.workers.get(key).unwrap();

        for queue_key in &worker.queues {
            if !self.queues_workers.contains_key(&queue_key) {
                self.queues_workers.insert(queue_key.clone(), HashSet::<WK>::default());
            }

            let queue_workers = self.queues_workers.get_mut(&queue_key).unwrap();

            queue_workers.remove(&worker.key);

            if queue_workers.len() == 0 {
                self.queues_workers.remove(&queue_key);
            }
        }
    }

    pub fn assign(&mut self, key: &QK, job_key: &JK) -> Option<WK> {
        let worker_id = match self.queues_workers.get(key) {
            Some(workers) => Some(workers.iter().nth(0).unwrap().clone()),
            None => None
        };

        match worker_id {
            Some(worker_id) => {
                let worker = self.workers.get_mut(&worker_id).unwrap();

                worker.current.insert(PubSubJob::inst(key.clone(), job_key.clone()));
                self.jobs_workers.insert(job_key.clone(), worker_id.clone());

                if !worker.ready() {
                    self.worker_disable(&worker_id.clone());
                }

                Some(worker_id.clone())
            }
            None => None
        }
    }

    pub fn resign(&mut self, key: &QK, job_key: &JK) -> WK {
        let worker_id = self.jobs_workers.get(job_key).unwrap().clone();

        self.jobs_workers.remove(job_key);

        let worker = self.workers.get_mut(&worker_id).unwrap();

        let was_full = !worker.ready();

        worker.current.remove(&PubSubJob::inst(key.clone(), job_key.clone()));

        if was_full {
            self.worker_enable(&worker_id);
        }

        worker_id
    }
}

use std;

impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> MultiQueue<WK, QK, JK>
    where QK: std::fmt::Debug,
          JK: std::fmt::Debug
{
    fn assignment(capacity: usize) -> Vec<Assignment<WK, QK, JK>> {
        Vec::<Assignment<WK, QK, JK>>::with_capacity(capacity)
    }

    pub fn job_create(&mut self, queue_key: &QK, job_key: &JK) -> Vec<Assignment<WK, QK, JK>> {
        match self.pubsub.assign(queue_key, job_key) {
            Some(worker_key) => vec![Assignment::new(
                Action::Started, worker_key,
                queue_key.clone(),
                job_key.clone(),
            )],
            None => {
                if !self.queues.contains_key(queue_key) {
                    self.queues.insert(queue_key.clone(), VecDeque::<JK>::default());
                }

                match self.queues.get_mut(queue_key) {
                    Some(x) => {
                        x.push_back(job_key.clone());
                    },
                    None => panic!("should never happen")
                };

                vec![]
            }
        }
    }

    pub fn job_finish(&mut self, queue_key: &QK, job_key: &JK) -> Vec<Assignment<WK, QK, JK>> {
        // todo what if the job is not assigned yet?

        let mut assignment = Self::assignment(2);

        let worker_key = self.pubsub.resign(queue_key, job_key);

        assignment.push(
            Assignment::new(Action::Done, worker_key.clone(), queue_key.clone(), job_key.clone())
        );

        let vec = self.worker_queues.get(&worker_key).unwrap().clone();

        assignment.append(&mut self.assign_queues(&vec, Some(1)));

        assignment
    }

    pub fn worker_add(&mut self, key: WK, capacity: usize, queues: Vec<QK>) -> Vec<Assignment<WK, QK, JK>> {
        match self.worker_queues.get(&key) {
            Some(x) => panic!("worker already exists"),
            None => {}
        }

        self.pubsub.add(key.clone(), capacity, &queues);

        self.worker_queues.insert(key, queues.clone());

        self.assign_queues(&queues, Some(capacity))
    }

    fn assign_queues(&mut self, queues: &Vec<QK>, capacity: Option<usize>) -> Vec<Assignment<WK, QK, JK>> {
        let mut capacity = capacity.clone();

        let mut assignment = Self::assignment(capacity.unwrap_or(5));

        let check_capacity = |capacity: Option<usize>| match capacity {
            Some(x) => x > 0,
            None => true
        };

        let mut queues_iter = queues.iter();

        while check_capacity(capacity) {
            let queue_key = match queues_iter.next() {
                Some(x) => x,
                None => break
            };

            let queue = match self.queues.get_mut(queue_key) {
                Some(x) => x,
                None => continue
            };

            while check_capacity(capacity) {
                let job_key = match queue.pop_front() {
                    Some(x) => x,
                    None => break
                };

                match self.pubsub.assign(&queue_key, &job_key) {
                    Some(worker_key) => {
                        assignment.push(
                            Assignment::new(
                                Action::Started,
                                worker_key.clone(),
                                queue_key.clone(),
                                job_key.clone(),
                            )
                        );

                        match capacity {
                            Some(x) => {
                                capacity = Some(x - 1)
                            }
                            None => {
                                continue;
                            }
                        }
                    }
                    None => {
                        match capacity {
                            Some(mut x) => {
                                capacity = Some(0)
                            }
                            None => {
                                continue;
                            }
                        }
                    }
                }
            }
        };

        assignment
    }
}