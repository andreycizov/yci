#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use crate::pubsub::*;

    #[test]
    fn test_worker_first() {
        let mut a = PubSub::<u32, u32, u32>::default();

        a.add(1000, 5, &vec![100, 200, 300]);

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

        a.add(1000, 0, &vec![100, 200, 300]);

        assert_eq!(
            a.assign(&400, &50),
            None
        );
    }
}