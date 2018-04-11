use super::task::{Iterable, TaskState};
use crossbeam_deque::{Deque, Stealer};
use libc::{cpu_set_t, pthread_setaffinity_np, sched_getcpu, CPU_SET, CPU_ZERO};
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::sync::mpsc::{channel, Sender, SendError, Receiver};
use std::thread;

pub struct Executor {
    cpu: usize,
    thread: thread::JoinHandle<()>,
    work_channel: Sender<Box<Iterable>>,
    work_stealer: Stealer<Box<Iterable>>,
}

impl Executor {
    pub fn new(cpu: usize) -> Executor {
        let work_queue = Deque::<Box<Iterable>>::new(); // this will be moved
        let work_stealer = work_queue.stealer();
        let (send_work_channel, receive_work_channel) = channel();

        let t_handle = thread::spawn(move || {
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
                                // currently there is some difficulty with
                                // allowing the thread to push it's task back
                                // to the externally observable work queue.
                                // crossbeam's Deque cannot be shared between
                                // threads, rather it allows for many stealers
                                // to be created and moved to many different
                                // threads.
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
                        // TODO: could use some observability on how many times
                        // popped on an empty queue.
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
            work_stealer,
        };
        executor
    }

    pub fn schedule(&self, task: Box<Iterable>) -> Result<(), SendError<Box<Iterable>>> {
        self.work_channel.send(task)
    }

    pub fn task_count(&self) -> usize {
        self.work_stealer.len()
    }

    pub fn get_cpu(&self) -> usize {
        self.cpu
    }
}

// TODO: Implement Drop for Executor
