use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub(crate) struct PubSubJob<QK: Clone + Eq + Hash + PartialEq, JK: Clone + Eq + Hash + PartialEq> {
    qk: QK,
    jk: JK,
}

impl<QK: Clone + Eq + Hash + PartialEq, JK: Clone + Eq + Hash + PartialEq> PubSubJob<QK, JK> {
    fn inst(qk: QK, jk: JK) -> Self {
        PubSubJob { qk, jk }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PubSubWorkerInfo<WK, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) key: WK,
    pub(crate) current: HashSet<PubSubJob<QK, JK>>,
    pub(crate) capacity: usize,
    pub(crate) queues: Vec<QK>,
}

impl<WK, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> PubSubWorkerInfo<WK, QK, JK> {
    pub fn ready(&self) -> bool {
        return self.current.len() < self.capacity;
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PubSub<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> {
    pub(crate) workers: HashMap<WK, PubSubWorkerInfo<WK, QK, JK>>,
    pub(crate) queues_workers: HashMap<QK, HashSet<WK>>,
    pub(crate) jobs_workers: HashMap<JK, WK>,
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

impl<WK: Clone + Eq + Hash, QK: Clone + Eq + Hash, JK: Clone + Eq + Hash> PubSub<WK, QK, JK> {
    pub fn add(&mut self, key: WK, capacity: usize, queues: Vec<QK>) {
        let worker = PubSubWorkerInfo { key, current: HashSet::<PubSubJob<QK, JK>>::default(), capacity, queues };
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

    pub fn resign(&mut self, key: &QK, job_key: &JK) {
        let worker_id = self.jobs_workers.get(job_key).unwrap().clone();

        self.jobs_workers.remove(job_key);

        let worker = self.workers.get_mut(&worker_id).unwrap();

        let is_full = !worker.ready();

        worker.current.remove(&PubSubJob::inst(key.clone(), job_key.clone()));

        if is_full {
            self.worker_enable(&worker_id);
        }
    }
}