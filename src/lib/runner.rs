#![deny(clippy::all)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use crate::jobs::{self, is_result_done, is_result_settled, Execute, Status};

// TODO: detect number of CPUs
const MAX_THREADS: usize = 2;

pub fn run(jobs: Vec<(impl Execute + Send + 'static)>) {
    let mut results = HashMap::<String, jobs::Result>::new();
    // ensure every job has a registered Status
    jobs.iter().for_each(|job| {
        if job.needs().is_empty() {
            results.insert(job.name(), Ok(Status::Pending));
        } else {
            results.insert(job.name(), Ok(Status::Blocked));
        }
    });

    let jobs_arc = Arc::new(Mutex::new(jobs));
    let results_arc = Arc::new(Mutex::new(results));
    let mut handles = Vec::<thread::JoinHandle<_>>::with_capacity(MAX_THREADS);
    for _ in 0..MAX_THREADS {
        let my_jobs_arc = jobs_arc.clone();
        let my_results_arc = results_arc.clone();

        let handle = thread::spawn(move || {
            loop {
                let current_job;
                {
                    // acquire locks
                    let mut my_jobs = my_jobs_arc.lock().unwrap();
                    let mut my_results = my_results_arc.lock().unwrap();

                    // move Blocked jobs with satifisfied needs over to Pending
                    for job in my_jobs.iter() {
                        let name = job.name();
                        if !is_equal_status(my_results.get(&name).unwrap(), &Status::Blocked) {
                            continue;
                        }
                        if job
                            .needs()
                            .iter()
                            .all(|n| is_result_done(my_results.get(n).unwrap()))
                        {
                            my_results.insert(name, Ok(Status::Pending));
                        }
                    }

                    // check exit/terminate condition for thread
                    if is_all_settled(&my_results) {
                        return; // nothing left to do
                    }
                    // there must be at least one available job

                    // cherry-pick first available job
                    let index = match my_jobs.iter().enumerate().find(|(_, job)| {
                        let name = job.name();
                        // this .unwrap() is fine, as all jobs have a registered Status
                        match my_results.get(&name).unwrap() {
                            Ok(Status::Pending) => true,
                            _ => false,
                        }
                    }) {
                        Some((i, _)) => i,
                        None => {
                            // the only remaining jobs must already be InProgress
                            // nothing left to do
                            return;
                        }
                    };
                    current_job = my_jobs.remove(index);
                    let name = current_job.name();
                    my_results.insert(name.clone(), Ok(Status::InProgress));
                    println!("job: {}: {:?}", &name, my_results.get(&name).unwrap());

                    // release/drop locks
                }

                // execute job
                let name = current_job.name();
                let result = current_job.execute();

                // record result of job
                {
                    // acquire locks
                    let mut my_results = my_results_arc.lock().unwrap();

                    my_results.insert(name.clone(), result);
                    println!("job: {}: {:?}", &name, my_results.get(&name).unwrap());
                    // release/drop locks
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("worker thread failed");
    }
}

fn is_all_settled(results: &HashMap<String, jobs::Result>) -> bool {
    results.iter().all(|(_, result)| is_result_settled(result))
}

fn is_equal_status(result: &jobs::Result, status: &Status) -> bool {
    match result {
        Ok(s) => s == status,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    struct FakeJob {
        name: String,
        needs: Vec<String>,
        result: jobs::Result,
        sleep: Duration,
        spy_arc: Arc<Mutex<FakeJobSpy>>,
    }
    impl Default for FakeJob {
        fn default() -> Self {
            Self {
                name: String::new(),
                needs: Vec::<String>::new(),
                result: Ok(jobs::Status::Done),
                sleep: Duration::from_millis(0),
                spy_arc: Arc::new(Mutex::new(FakeJobSpy {
                    calls: 0,
                    time: None,
                })),
            }
        }
    }
    impl FakeJob {
        fn new<S>(name: S, result: jobs::Result) -> (Self, Arc<Mutex<FakeJobSpy>>)
        where
            S: AsRef<str>,
        {
            let job = FakeJob {
                name: String::from(name.as_ref()),
                result,
                ..Default::default()
            };
            let spy_arc = job.spy_arc.clone();

            (job, spy_arc)
        }
    }
    impl Execute for FakeJob {
        fn execute(&self) -> jobs::Result {
            thread::sleep(self.sleep);
            let mut my_spy = self.spy_arc.lock().unwrap();
            my_spy.calls += 1;
            my_spy.time = Some(Instant::now());
            self.result.clone()
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn needs(&self) -> Vec<String> {
            self.needs.clone()
        }
    }

    struct FakeJobSpy {
        calls: usize,
        time: Option<Instant>,
    }
    impl FakeJobSpy {
        fn assert_called_once(&self) {
            assert_eq!(self.calls, 1);
            assert!(self.time.is_some());
        }

        fn assert_never_called(&self) {
            assert_eq!(self.calls, 0);
            assert!(self.time.is_none());
        }
    }

    #[test]
    fn run_executes_unordered_jobs() {
        let (a, a_spy) = FakeJob::new("a", Ok(jobs::Status::NoChange(String::from("a"))));
        let (b, b_spy) = FakeJob::new("b", Ok(jobs::Status::Done));

        let jobs = vec![a, b];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        my_a_spy.assert_called_once();
        my_b_spy.assert_called_once();
    }

    #[test]
    fn run_executes_unordered_jobs_concurrently() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (mut b, b_spy) = FakeJob::new("b", Ok(jobs::Status::Done));
        a.sleep = Duration::from_millis(500);
        b.sleep = Duration::from_millis(500);

        let jobs = vec![a, b];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        my_a_spy.assert_called_once();
        my_b_spy.assert_called_once();
        // assert that both jobs finished very recently,
        // that they had to have been executed concurrently
        assert!(my_a_spy.time.expect("a").elapsed() < Duration::from_millis(50));
        assert!(my_b_spy.time.expect("b").elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn run_executes_ordered_jobs() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (b, b_spy) = FakeJob::new("b", Ok(jobs::Status::NoChange(String::from("b"))));
        a.needs.push(String::from("b"));

        let jobs = vec![a, b];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        my_a_spy.assert_called_once();
        my_b_spy.assert_called_once();
        // assert that "a" finished after "b"
        assert!(my_a_spy.time.expect("a") > my_b_spy.time.expect("b"));
    }

    #[test]
    fn run_does_not_execute_ordered_job_when_needs_are_not_done() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        let (b, b_spy) = FakeJob::new("b", Err(jobs::Error::Other(String::from("something bad"))));
        a.needs.push(String::from("b"));

        let jobs = vec![a, b];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        my_a_spy.assert_never_called();
        my_b_spy.assert_called_once();
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

        let jobs = vec![a, b, c];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        let my_b_spy = b_spy.lock().unwrap();
        let my_c_spy = c_spy.lock().unwrap();
        my_a_spy.assert_never_called();
        my_b_spy.assert_called_once();
        my_c_spy.assert_called_once();
    }
}
