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
    pub fn new(n_threads: usize, n_cores: usize) -> WorkStealingCpuPool {
        let mut cpu_list: Vec<usize> = Vec::with_capacity(n_threads);
        for i in 0..n_threads {
            cpu_list.push(i % n_cores);
        }
        WorkStealingCpuPool::new_from_list(cpu_list)
    }

    pub fn new_from_list(cpu_thread_list: Vec<usize>) -> WorkStealingCpuPool {
        let n_threads = cpu_thread_list.len();
        let workers: Vec<(Executor, Stealer<Box<Iterable>>)> = cpu_thread_list
            .into_iter()
            .map(|cpu_thread_id: usize| Executor::new(cpu_thread_id, n_threads - 1))
            .collect();
        for (executor_a, _) in &workers {
            &workers
                .iter()
                .filter(|&&(ref worker, _)| worker as *const _ != executor_a as *const _)
                .for_each(|&(_, ref stealer)| {
                    executor_a.send_stealer(stealer.clone()).unwrap();
                });
        }
        WorkStealingCpuPool { workers }
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
