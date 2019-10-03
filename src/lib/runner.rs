#![deny(clippy::all)]

use std::collections::HashMap;

use crate::jobs::{self, Execute, Status};

pub fn run(jobs: &mut Vec<impl Execute>) {
    let mut results = HashMap::<String, jobs::Result>::new();

    jobs.iter().for_each(|job| {
        if job.needs().is_empty() {
            results.insert(job.name(), Ok(Status::Pending));
        } else {
            results.insert(job.name(), Ok(Status::Blocked));
        }
    });
    while results.is_empty() || !is_all_done(&results) {
        jobs.iter().for_each(|job| {
            let name = job.name();
            // this .unwrap() is fine, as all jobs have a registered Status
            match results.get(&name).unwrap() {
                Ok(Status::Pending) => {
                    results.insert(name.clone(), Ok(Status::InProgress));
                    println!("job: {}: {:?}", &name, results.get(&name).unwrap());
                    results.insert(name.clone(), job.execute());
                    println!("job: {}: {:?}", &name, results.get(&name).unwrap());
                }
                _ => {
                    println!("job: {}: {:?}", &name, results.get(&name).unwrap());
                }
            }
        });
        for job in jobs.iter() {
            let name = job.name();
            if !is_equal_status(results.get(&name).unwrap(), &Status::Blocked) {
                continue;
            }
            if job
                .needs()
                .iter()
                .all(|n| is_equal_status(results.get(n).unwrap(), &Status::Done))
            {
                results.insert(name, Ok(Status::Pending));
            }
        }
    }
}

fn is_all_done(results: &HashMap<String, jobs::Result>) -> bool {
    results.iter().all(|(_, result)| match result {
        Ok(s) => match s {
            Status::Blocked | Status::Done => true,
            _ => false,
        },
        Err(_) => true,
    })
}

fn is_equal_status(result: &jobs::Result, status: &Status) -> bool {
    match result {
        Ok(s) => s == status,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        time::Instant,
    };

    use super::*;

    struct FakeJob {
        r#fn: Box<dyn Fn() -> jobs::Result>,
        name: String,
        needs: Vec<String>,
    }
    impl FakeJob {
        fn new<S>(name: S, result: jobs::Result) -> (Self, Arc<Mutex<FakeJobSpy>>)
        where
            S: AsRef<str>,
        {
            let spy = Arc::new(Mutex::new(FakeJobSpy {
                calls: AtomicUsize::new(0),
                time: None,
            }));
            let spy_arc = spy.clone();
            let j = FakeJob {
                r#fn: Box::new(move || {
                    let mut my_spy = spy_arc.lock().unwrap();
                    my_spy.calls.fetch_add(1, Ordering::Relaxed);
                    my_spy.time = Some(Instant::now());
                    result.clone()
                }),
                name: String::from(name.as_ref()),
                needs: vec![],
            };
            (j, spy)
        }
    }
    impl Execute for FakeJob {
        fn execute(&self) -> jobs::Result {
            (self.r#fn)()
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn needs(&self) -> Vec<String> {
            self.needs.clone()
        }
    }

    struct FakeJobSpy {
        calls: AtomicUsize,
        time: Option<Instant>,
    }

    #[test]
    fn run_executes_unordered_jobs() {
        let (a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (b, b_spy) = FakeJob::new("b", Ok(jobs::Status::Done));

        let mut jobs = vec![a, b];
        run(&mut jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        assert_eq!(my_a_spy.calls.load(Ordering::Relaxed), 1);
        assert_eq!(my_b_spy.calls.load(Ordering::Relaxed), 1);
        assert!(my_a_spy.time.is_some());
        assert!(my_b_spy.time.is_some());
    }

    #[test]
    fn run_executes_ordered_jobs() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (b, b_spy) = FakeJob::new("b", Ok(jobs::Status::Done));
        a.needs.push(String::from("b"));

        let mut jobs = vec![a, b];
        run(&mut jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        assert_eq!(my_a_spy.calls.load(Ordering::Relaxed), 1);
        assert_eq!(my_b_spy.calls.load(Ordering::Relaxed), 1);
        assert!(my_a_spy.time.is_some());
        assert!(my_b_spy.time.is_some());
        assert!(my_a_spy.time.expect("a") > my_b_spy.time.expect("b"));
    }

    #[test]
    fn run_does_not_execute_ordered_job_when_needs_are_not_done() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (b, b_spy) = FakeJob::new("b", Err(jobs::Error::Other(String::from("something bad"))));
        a.needs.push(String::from("b"));

        let mut jobs = vec![a, b];
        run(&mut jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        assert_eq!(my_a_spy.calls.load(Ordering::Relaxed), 0);
        assert_eq!(my_b_spy.calls.load(Ordering::Relaxed), 1);
        assert!(my_a_spy.time.is_none());
        assert!(my_b_spy.time.is_some());
    }

    #[test]
    fn run_does_not_execute_ordered_job_when_some_needs_are_not_done() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (mut b, b_spy) =
            FakeJob::new("b", Err(jobs::Error::Other(String::from("something bad"))));
        let (c, c_spy) = FakeJob::new("c", Ok(jobs::Status::Done));
        a.needs.push(String::from("b"));
        a.needs.push(String::from("c"));
        b.needs.push(String::from("c"));

        let mut jobs = vec![a, b, c];
        run(&mut jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        let my_c_spy = c_spy.lock().unwrap();
        assert_eq!(my_a_spy.calls.load(Ordering::Relaxed), 0);
        assert_eq!(my_b_spy.calls.load(Ordering::Relaxed), 1);
        assert_eq!(my_c_spy.calls.load(Ordering::Relaxed), 1);
        assert!(my_a_spy.time.is_none());
        assert!(my_b_spy.time.is_some());
        assert!(my_c_spy.time.is_some());
    }
}
