use super::executor::Executor;
use super::task::Iterable;
use crossbeam_deque::Stealer;

pub trait CpuPool {
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()>;
}

pub struct WorkStealingCpuPool {
    workers: Vec<(Executor, Stealer<Box<Iterable>>)>,
}

impl WorkStealingCpuPool {
    pub fn new(n_threads: usize) -> WorkStealingCpuPool {
        let mut pool = WorkStealingCpuPool {
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
                .for_each(|&(_, ref stealer)| executor_a.send_stealer(stealer.clone()).unwrap());
        }
        pool
    }
}

impl CpuPool for WorkStealingCpuPool {
    // Finds the least busy executor and queues the task into it's work queue
    // Right now, the least busy executor is the one with the least tasks
    // scheduled, but that number is possibly incorrect because Executors
    // receive tasks on a channel, which is not counted in "task_count".
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()> {
        // get executor with min tasks
        let trgt = self
            .workers
            .iter()
            .map(|&(ref executor, _)| executor)
            .min_by_key(|executor| executor.count_tasks());
        match trgt {
            Some(executor) => {
                executor.schedule(task);
                Ok(executor.get_cpu())
            }
            None => Err(()),
        }
    }
}

pub struct SegregatedCpuPool {
    workers: Vec<Executor>,
}

impl SegregatedCpuPool {
    pub fn new(n_threads: usize) -> SegregatedCpuPool {
        let mut pool = SegregatedCpuPool {
            workers: Vec::with_capacity(n_threads),
        };
        for i in 0..n_threads {
            let (executor, _) = Executor::new(i, 0);
            pool.workers.push(executor);
        }
        pool
    }
}

impl CpuPool for SegregatedCpuPool {
    // Finds the least busy executor and queues the task into it's work queue
    // Right now, the least busy executor is the one with the least tasks
    // scheduled.
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()> {
        // update tasks counts

        // get executor with min tasks
        let trgt = self
            .workers
            .iter()
            .min_by_key(|executor| executor.count_tasks());
        match trgt {
            Some(executor) => {
                executor.schedule(task);
                Ok(executor.get_cpu())
            }
            None => Err(()),
        }
    }
}
// TODO: Implement Drop for CPU Pool
