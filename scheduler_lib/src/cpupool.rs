use super::dispatcher::{Dispatcher, LoadAwareDispatcher};
use super::executor::Executor;
use super::task::Iterable;
use crossbeam_deque::Stealer;

pub trait CpuPool {
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()>;
}

pub struct WorkStealingCpuPool {
    dispatcher: Box<Dispatcher>,
}

impl WorkStealingCpuPool {
    pub fn new(n_threads: usize, n_cores: usize, mut dispatcher: Box<Dispatcher>) -> WorkStealingCpuPool {
        let mut cpu_list: Vec<usize> = Vec::with_capacity(n_threads);
        for i in 0..n_threads {
            cpu_list.push(i % n_cores);
        }
        WorkStealingCpuPool::new_from_list(cpu_list, dispatcher)
    }

    pub fn new_from_list(cpu_thread_list: Vec<usize>, mut dispatcher: Box<Dispatcher>) -> WorkStealingCpuPool {
        let n_threads = cpu_thread_list.len();
        let workers: Vec<(Executor, Stealer<Box<Iterable>>)> = cpu_thread_list
            .into_iter()
            .map(|cpu_thread_id: usize| Executor::new(cpu_thread_id, n_threads - 1))
            .collect();

        // inject stealers
        for (executor_a, _) in &workers {
            &workers
                .iter()
                .filter(|&&(ref worker, _)| worker as *const _ != executor_a as *const _)
                .for_each(|&(_, ref stealer)| {
                    executor_a.send_stealer(stealer.clone()).unwrap();
                });
        }

        let workers_for_dispatch: Vec<Executor> =
            workers.into_iter().map(|(worker, _)| worker).collect();
        dispatcher.inject_fleet(workers_for_dispatch);
        WorkStealingCpuPool {
            dispatcher,
        }
    }
}

impl CpuPool for WorkStealingCpuPool {
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()> {
        match self.dispatcher.select() {
            Some(executor) => {
                executor.schedule(task);
                Ok(executor.get_cpu())
            }
            None => Err(()),
        }
    }
}

pub struct SegregatedCpuPool {
    dispatcher: Box<Dispatcher>,
}

impl SegregatedCpuPool {
    pub fn new(n_threads: usize, mut dispatcher: Box<Dispatcher>) -> SegregatedCpuPool {
        let mut workers = Vec::with_capacity(n_threads);
        for i in 0..n_threads {
            let (executor, _) = Executor::new(i, 0);
            workers.push(executor);
        }
        dispatcher.inject_fleet(workers);
        SegregatedCpuPool {
            dispatcher,
        }
    }
}

impl CpuPool for SegregatedCpuPool {
    fn schedule(&self, task: Box<Iterable>) -> Result<usize, ()> {
        match self.dispatcher.select() {
            Some(executor) => {
                executor.schedule(task);
                Ok(executor.get_cpu())
            }
            None => Err(()),
        }
    }
}
// TODO: Implement Drop for CPU Pool
