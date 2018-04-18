use super::task::{Iterable, TaskState};
use crossbeam_deque::{Deque, Steal, Stealer};
use libc::{cpu_set_t, pthread_setaffinity_np, CPU_SET, CPU_ZERO};
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::sync::mpsc::{channel, SendError, Sender};
use std::thread;

pub struct Executor {
    cpu: usize,
    thread: thread::JoinHandle<()>,
    work_channel: Sender<Box<Iterable>>,
    stealer_channel: Sender<Stealer<Box<Iterable>>>,
    work_stealer: Stealer<Box<Iterable>>,
}

impl Executor {
    pub fn new(cpu: usize, n_stealers: usize) -> (Executor, Stealer<Box<Iterable>>) {
        let work_queue = Deque::<Box<Iterable>>::new();
        let work_stealer = work_queue.stealer();
        let local_stealer = work_queue.stealer();
        let (send_work_channel, receive_work_channel) = channel();
        let (send_stealer_channel, receive_stealer_channel) = channel();

        let t_handle = thread::spawn(move || {
            // TODO: Create an inner struct to wrap all this logic up
            let mut stealers: Vec<Stealer<Box<Iterable>>> = Vec::with_capacity(n_stealers);
            for _ in 0..n_stealers {
                match receive_stealer_channel.recv() {
                    Ok(stealer) => {
                        stealers.push(stealer);
                    }
                    Err(_) => {
                        println!("Failed to recieve stealer.");
                    }
                }
            }
            // Spin, receive tasks, and take stuff from queue
            loop {
                // Check the mailbox for new work from the outside world
                match receive_work_channel.try_recv() {
                    Ok(task) => {
                        work_queue.push(task);
                        // if there was work to consume, lets loop back around
                        // and check for some more before moving on.
                        continue;
                    }
                    Err(_) => {}
                }
                // Take an item and work on it
                match work_queue.pop() {
                    Some(mut task) => {
                        task.tick();
                        match task.get_state() {
                            &TaskState::Incomplete => {
                                work_queue.push(task);
                            }
                            &TaskState::Unstarted => {
                                // this is unexpected, and may be an error
                                println!("A task was started, but it's state remained unstarted");
                            }
                            &TaskState::Complete => {
                                task.complete();
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
                    }
                    None => {
                        let stealer = stealers
                            .iter()
                            .max_by_key(|s| s.len());
                        match stealer {
                            Some(stealer) => match stealer.steal() {
                                Steal::Data(task) => {
                                    work_queue.push(task);
                                }
                                Steal::Empty => {}
                                Steal::Retry => {}
                            },
                            None => {}
                        };
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
            work_channel: send_work_channel,
            stealer_channel: send_stealer_channel,
            work_stealer: local_stealer,
        };
        (executor, work_stealer)
    }

    pub fn schedule(&self, task: Box<Iterable>) -> Result<(), SendError<Box<Iterable>>> {
        self.work_channel.send(task)
    }

    pub fn send_stealer(
        &self,
        stealer: Stealer<Box<Iterable>>,
    ) -> Result<(), SendError<Stealer<Box<Iterable>>>> {
        self.stealer_channel.send(stealer)
    }

    pub fn task_count(&self) -> usize {
        self.work_stealer.len()
    }

    pub fn get_cpu(&self) -> usize {
        self.cpu
    }
}

// TODO: Implement Drop for Executor
