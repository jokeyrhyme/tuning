use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
};

use thiserror::Error as ThisError;

use crate::jobs::{self, is_result_done, is_result_settled, Execute, Status};

// TODO: detect number of CPUs
const MAX_THREADS: usize = 2;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Job {
        #[from]
        source: jobs::Error,
    },
}

// pub type Result = std::result::Result<(), Error>;

// TODO: consider extracting the concern of println!ing Status
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

                    // move jobs with false "when" over to Skipped
                    for job in my_jobs.iter() {
                        let name = job.name();
                        if !job.when() {
                            my_results.insert(name.clone(), Ok(Status::Skipped));
                        }
                    }

                    // move Blocked jobs with satifisfied needs over to Pending
                    for job in my_jobs.iter() {
                        let name = job.name();
                        if is_equal_status(my_results.get(&name).unwrap(), &Status::Blocked)
                            && job
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
                        is_equal_status(my_results.get(&name).unwrap(), &Status::Pending)
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
                    println!(
                        "job: {}: {}",
                        &name,
                        jobs::result_display(my_results.get(&name).unwrap())
                    );

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
                    println!(
                        "job: {}: {}",
                        &name,
                        jobs::result_display(my_results.get(&name).unwrap())
                    );
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
        when: bool,
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
                when: true,
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
            result_clone(&self.result)
        }
        fn name(&self) -> String {
            self.name.clone()
        }
        fn needs(&self) -> Vec<String> {
            self.needs.clone()
        }
        fn when(&self) -> bool {
            self.when
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
    fn run_does_not_execute_job_with_false_when_or_needs_job_with_false_when() {
        let (mut a, a_spy) = FakeJob::new("a", Ok(jobs::Status::Done));
        a.when = false;
        let (mut b, b_spy) = FakeJob::new("b", Ok(jobs::Status::Done));
        b.needs.push(String::from("a"));

        let jobs = vec![a, b];
        run(jobs);

        let my_a_spy = a_spy.lock().unwrap();
        my_a_spy.assert_never_called();
        let my_b_spy = b_spy.lock().unwrap();
        my_b_spy.assert_never_called();
    }

    #[test]
    fn run_executes_unordered_jobs() {
        const MAX_COUNT: usize = 10;
        let mut jobs = Vec::<FakeJob>::with_capacity(MAX_COUNT);
        let mut spy_arcs = Vec::<Arc<Mutex<FakeJobSpy>>>::with_capacity(MAX_COUNT);
        for i in 0..MAX_COUNT {
            let (job, spy_arc) = FakeJob::new(
                format!("{}", i),
                match i % 2 {
                    0 => Ok(jobs::Status::Done),
                    _ => Ok(jobs::Status::NoChange(format!("{}", i))),
                },
            );
            jobs.push(job);
            spy_arcs.push(spy_arc);
        }

        run(jobs);

        for spy_arc in spy_arcs {
            let spy = spy_arc.lock().unwrap();
            spy.assert_called_once();
        }
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
        assert!(my_a_spy.time.expect("a").elapsed() < Duration::from_millis(100));
        assert!(my_b_spy.time.expect("b").elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn run_executes_jobs_with_complex_needs() {
        const MAX_COUNT: usize = 100;
        let mut jobs = Vec::<FakeJob>::with_capacity(MAX_COUNT);
        let mut spy_arcs = Vec::<Arc<Mutex<FakeJobSpy>>>::with_capacity(MAX_COUNT);
        for i in 0..MAX_COUNT {
            let (mut job, spy_arc) = FakeJob::new(
                format!("{}", i),
                match i % 2 {
                    0 => Ok(jobs::Status::Done),
                    _ => Ok(jobs::Status::NoChange(format!("{}", i))),
                },
            );
            match i % 10 {
                2 => {
                    job.needs = vec![format!("{}", i + 2)];
                }
                3 => {
                    job.needs = vec![format!("{}", i - 3)];
                }
                4 => {
                    job.needs = vec![format!("{}", i + 3)];
                }
                7 => {
                    job.needs = vec![String::from("99")];
                }
                _ => { /* noop */ }
            }
            jobs.push(job);
            spy_arcs.push(spy_arc);
        }

        run(jobs);

        for i in 0..MAX_COUNT {
            let spy_arc = &spy_arcs[i];
            let spy = spy_arc.lock().unwrap();
            spy.assert_called_once();
            match i % 10 {
                2 => {
                    let spyx4_arc = &spy_arcs[i + 2];
                    let spyx4 = spyx4_arc.lock().unwrap();
                    // jobs ending in 2 should all run after the next job ending in 4
                    assert!(spy.time.expect("x4") > spyx4.time.expect("x7"));
                }
                3 => {
                    let spyx0_arc = &spy_arcs[i - 3];
                    let spyx0 = spyx0_arc.lock().unwrap();
                    // jobs ending in 3 should all run after the previous job ending in 0
                    assert!(spy.time.expect("x3") > spyx0.time.expect("x7"));
                }
                4 => {
                    let spyx7_arc = &spy_arcs[i + 3];
                    let spyx7 = spyx7_arc.lock().unwrap();
                    // jobs ending in 4 should all run after the next job ending in 7
                    assert!(spy.time.expect("x4") > spyx7.time.expect("x7"));
                }
                7 => {
                    let spy99_arc = &spy_arcs[99];
                    let spy99 = spy99_arc.lock().unwrap();
                    // jobs ending in 7 should all run after job #99
                    assert!(spy.time.expect("x7") > spy99.time.expect("99"));
                }
                _ => { /* noop */ }
            }
        }
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
        let (b, b_spy) = FakeJob::new("b", Err(jobs::Error::SomethingBad));
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
        let (mut b, b_spy) = FakeJob::new("b", Err(jobs::Error::SomethingBad));
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

    fn result_clone(result: &jobs::Result) -> jobs::Result {
        match result {
            Ok(s) => Ok(s.clone()),
            Err(_) => Err(jobs::Error::SomethingBad),
        }
    }
}
