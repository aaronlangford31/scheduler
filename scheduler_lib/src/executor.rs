use super::task::{Iterable, TaskState};
use crossbeam_deque::{Deque, Stealer, Steal};
use libc::{cpu_set_t, pthread_setaffinity_np, sched_getcpu, CPU_SET, CPU_ZERO};
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::sync::Arc;
use std::thread;

pub struct Executor {
    cpu: usize,
    thread: thread::JoinHandle<()>,
    work_queue: Deque<Arc<Iterable>>,
    work_stealer: Stealer<Arc<Iterable>>
}

impl Executor {
    pub fn new(cpu: usize) -> Executor {
        let work_queue = Deque::<Arc<Iterable>>::new();
        let work_stealer = work_queue.stealer();
        let local_stealer = work_queue.stealer();

        let t_handle = thread::spawn(move || {
            // Spin, receive tasks, and take stuff from queue
            loop {
                match local_stealer.steal() {
                    Steal::Data(mut task) => {
                        // Arc will not yield a mutable reference
                        // unless this is the only reference to
                        // the task. Perhaps this task
                        match Arc::get_mut(&mut task) {
                            Some(mut_task) => {
                              mut_task.tick();
                            },
                            None => ()
                        };
                        match task.get_state() {
                            &TaskState::Incomplete => {
                                // currently there is some difficulty with 
                                // allowing the thread to push it's task back 
                                // to the externally observable work queue. 
                                // crossbeam's Deque cannot be shared between 
                                // threads, rather it allows for many stealers
                                // to be created and moved to many different 
                                // threads.
                            }
                            &TaskState::Unstarted => {
                                // this is unexpected, and may be an error
                                println!("A task was started, but it's state remained unstarted");

                            }
                            &TaskState::Complete => {
                                println!("Executor {} completed a task.", cpu);
                                // maybe send a message that this is done?
                            }
                            &TaskState::Error => {
                                println!("A task was started, but resulted in an error");
                                // current philosophy: errors should be handled
                                // by the publisher of the task. Might be worth
                                // adding some layers here to make the reason
                                // for failure easy to determine.
                            }
                        };
                        continue;
                    },
                    Steal::Empty => {
                        // TODO: could use some observability on how many times
                        // popped on an empty queue.
                        continue;
                    },
                    Steal::Retry => {
                        continue;
                    }
                };
            }
        });

        // set thread affinity
        let tid = t_handle.as_pthread_t();
        unsafe {
            let mut cpuset: cpu_set_t = mem::uninitialized();
            CPU_ZERO(&mut cpuset);
            CPU_SET(cpu, &mut cpuset);
            pthread_setaffinity_np(tid, mem::size_of::<cpu_set_t>(), &mut cpuset);
        };

        let executor = Executor {
            cpu,
            thread: t_handle,
            work_queue,
            work_stealer
        };
        executor
    }

    pub fn schedule(&self, task: Arc<Iterable>) {
        self.work_queue.push(task);
    }

    pub fn task_count(&self) -> usize {
        self.work_queue.len()
    }

    pub fn get_cpu(&self) -> usize {
        self.cpu
    }
}

// TODO: Implement Drop for Executor
