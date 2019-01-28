
// Note this useful idiom: importing names from outer (for mod tests) scope.
use crate::pubsub::*;
use Action::*;

#[test]
fn test_worker_first() {
    let mut a = PubSub::<u32, u32, u32>::default();

    a.add(1000, Some(5), &vec![100, 200, 300]);

    assert_eq!(
        a.assign(&400, &50),
        None
    );

    assert_eq!(
        a.assign(&300, &60),
        Some(1000)
    );
}

#[test]
fn test_job_first() {
    let mut a = PubSub::<u32, u32, u32>::default();
    assert_eq!(
        a.assign(&400, &50),
        None
    );

    a.add(1000, Some(0), &vec![100, 200, 300]);

    assert_eq!(
        a.assign(&400, &50),
        None
    );
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, Ord, PartialOrd)]
enum Jobs {
    A,
    B,
    C,
    D,
    E,
}

#[test]
fn test_multi_queue_a() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![],
    );

    let ops2 = a.worker_add("a".to_string(), Some(2), &vec![1, 2, 3]);

    assert_eq!(
        ops2,
        vec![Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A }],
    );
}

#[test]
fn test_multi_queue_b() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::B);

    assert_eq!(
        ops,
        vec![],
    );

    let ops2 = a.worker_add("a".to_string(), Some(2), &vec![1, 2, 3]);

    assert_eq!(
        ops2,
        vec![
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::B },
        ],
    );
}

#[test]
fn test_multi_queue_c() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::B);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::C);

    assert_eq!(
        ops,
        vec![],
    );

    let ops2 = a.worker_add("a".to_string(), Some(2), &vec![1, 2, 3]);

    assert_eq!(
        ops2,
        vec![
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::B },
        ],
    );
}

#[test]
fn test_multi_queue_d() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::B);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::C);

    assert_eq!(
        ops,
        vec![],
    );

    let ops2 = a.worker_add("a".to_string(), Some(2), &vec![1, 2, 3]);

    assert_eq!(
        ops2,
        vec![
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::B },
        ],
    );

    let ops = a.job_finish(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![
            Assignment { action: Action::Done, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::C },
        ],
    );
}

#[test]
fn test_multi_queue_e() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::B);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::C);

    assert_eq!(
        ops,
        vec![],
    );

    let ops = a.job_create(&2, &Jobs::D);

    assert_eq!(
        ops,
        vec![],
    );

    let ops2 = a.worker_add("a".to_string(), Some(2), &vec![1, 2, 3]);

    assert_eq!(
        ops2,
        vec![
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::B },
        ],
    );

    let ops3 = a.worker_add("b".to_string(), Some(1), &vec![1, 2, 3]);

    assert_eq!(
        ops3,
        vec![
            Assignment { action: Action::Started, worker_key: "b".to_string(), queue_key: 2, job_key: Jobs::C },
        ],
    );

    let ops = a.job_finish(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![
            Assignment { action: Action::Done, worker_key: "a".to_string(), queue_key: 1, job_key: Jobs::A },
            Assignment { action: Action::Started, worker_key: "a".to_string(), queue_key: 2, job_key: Jobs::D },
        ],
    );
}

#[test]
fn test_multi_queue_cancel_job() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![]
    );

    let ops = a.job_finish(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![]
    );
}

#[test]
fn test_multi_queue_cancel() {
    let mut a = MultiQueue::<String, u32, Jobs>::default();
    let ops2 = a.worker_add("a".to_string(), None, &vec![1, 2, 3]);

    let ops = a.job_create(&1, &Jobs::A);

    assert_eq!(
        ops,
        vec![
            Assignment::new(Started, "a".into(), 1, Jobs::A)
        ],
    );

    let ops = a.job_create(&1, &Jobs::B);

    assert_eq!(
        ops,
        vec![
            Assignment::new(Started, "a".into(), 1, Jobs::B)
        ],
    );

    let mut ops = a.worker_remove(&"a".into());

    ops.sort_by_key(|x| (x.queue_key.clone(), x.job_key.clone()));

    assert_eq!(
        ops,
        vec![
            Assignment::new(Cancelled, "a".into(), 1, Jobs::A),
            Assignment::new(Cancelled, "a".into(), 1, Jobs::B),
        ],
    );

    let mut ops = a.worker_add("b".to_string(), None, &vec![1, 2, 3]);

    ops.sort_by_key(|x| (x.queue_key.clone(), x.job_key.clone()));

    assert_eq!(
        ops,
        vec![
            Assignment::new(Started, "b".into(), 1, Jobs::A),
            Assignment::new(Started, "b".into(), 1, Jobs::B),
        ],
    );
}
