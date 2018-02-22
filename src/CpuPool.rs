use std::sync::Arc;
use super::Task::{Task};
use super::Executor::Executor;

pub struct CpuPool {
  workers: Vec<Executor>
}

impl CpuPool {
  pub fn new(n_threads: usize) -> CpuPool {
    let mut pool = CpuPool { workers: Vec::with_capacity(n_threads) };
    for i in 0..n_threads {
      pool.workers.push(Executor::new());
    }

    pool
  }
  
  /// Finds the least busy executor and queues the task into it's work queue
  /// Right now, the least busy executor is the one with the least tasks
  /// scheduled, but there could be room for improvement depending on how
  /// this measures in benchmarks.
  pub fn schedule<R>(&self, task: Arc<Task<R>>) -> bool {
    // get executor with min tasks
    let trgt = self.workers.iter().min_by_key(|executor| executor.task_count());
    match trgt {
      Some(executor) => {
        executor.schedule(task);
        true
      }
      None => false
    }
  }
}


