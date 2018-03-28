use super::executor::Executor;
use super::task::Iterable;
use std::sync::Arc;

pub struct CpuPool {
    workers: Vec<Executor>
}

impl CpuPool {
    pub fn new(n_threads: usize) -> CpuPool {
        let mut pool = CpuPool {
            workers: Vec::with_capacity(n_threads)
        };
        
        for i in 0..n_threads {
            let executor = Executor::new(i);
            pool.workers.push(executor);
        }
        pool
    }

    /// Finds the least busy executor and queues the task into it's work queue
    /// Right now, the least busy executor is the one with the least tasks
    /// scheduled, but there could be room for improvement depending on how
    /// this measures in benchmarks.
    pub fn schedule(&self, task: Arc<Iterable>) -> bool {
        // get executor with min tasks
        let trgt = self.workers
            .iter()
            .min_by_key(|executor| executor.task_count());
        match trgt {
            Some(executor) => {
                executor.schedule(task);
                println!("Scheduled work to executor {}", executor.get_cpu());
                true
            }
            None => false,
        }
    }
}

// TODO: Implement Drop for CPU Pool
