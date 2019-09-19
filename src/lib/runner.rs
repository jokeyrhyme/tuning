use crate::jobs::Execute;

pub fn run(jobs: &mut Vec<impl Execute>) {
    for job in jobs {
        job.execute();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct FakeJob {
        r#fn: Box<dyn Fn()>,
    }
    impl Execute for FakeJob {
        fn execute(&mut self) {
            (self.r#fn)();
        }
    }

    #[test]
    fn run_executes_unordered_jobs() {
        let a_calls = Arc::new(Mutex::new(0));
        let a_calls_arc = a_calls.clone();
        let a_increment = move || {
            let mut my_a_calls = a_calls_arc.lock().unwrap();
            *my_a_calls += 1;
        };
        let a = FakeJob {
            r#fn: Box::new(a_increment),
        };

        let b_calls = Arc::new(Mutex::new(0));
        let b_calls_arc = b_calls.clone();
        let b_increment = move || {
            let mut my_b_calls = b_calls_arc.lock().unwrap();
            *my_b_calls += 1;
        };
        let b = FakeJob {
            r#fn: Box::new(b_increment),
        };

        let mut jobs = vec![a, b];
        run(&mut jobs);

        assert_eq!(*a_calls.lock().unwrap(), 1);
        assert_eq!(*b_calls.lock().unwrap(), 1);
    }
}
