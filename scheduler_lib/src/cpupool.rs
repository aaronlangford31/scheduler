use super::executor::Executor;
use super::task::Iterable;
use crossbeam_deque::Stealer;

pub struct CpuPool {
    workers: Vec<(Executor, Stealer<Box<Iterable>>)>,
}

impl CpuPool {
    pub fn new(n_threads: usize) -> CpuPool {
        let mut pool = CpuPool {
            workers: Vec::with_capacity(n_threads),
        };
        for i in 0..n_threads {
            let (executor, stealer) = Executor::new(i, n_threads - 1);
            pool.workers.push((executor, stealer));
        }
        for &(ref executor_a, _) in &pool.workers {
            pool.workers
                .iter()
                .filter(|&&(ref worker, _)| worker.get_cpu() != executor_a.get_cpu())
                .for_each(|&(_, ref stealer)| {
                    executor_a.send_stealer(stealer.clone()).unwrap()
                });
        }
        pool
    }

    // Finds the least busy executor and queues the task into it's work queue
    // Right now, the least busy executor is the one with the least tasks
    // scheduled, but that number is possibly incorrect because Executors
    // receive tasks on a channel, which is not counted in "task_count".
    pub fn schedule(&self, task: Box<Iterable>) -> bool {
        // get executor with min tasks
        let trgt = self.workers
            .iter()
            .map(|&(ref executor, _)| executor)
            .min_by_key(|executor| executor.task_count());
        match trgt {
            Some(executor) => {
                executor.schedule(task);
                true
            }
            None => false,
        }
    }
}

// TODO: Implement Drop for CPU Pool
