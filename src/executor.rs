use crossbeam_deque::{Deque, Stealer};
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use super::task::{TaskState, Tickable};

pub struct Executor {
    thread: thread::JoinHandle<()>,
    task_channel: Sender<Arc<Tickable>>,
    work_stealer: Stealer<Arc<Tickable>>,
}

impl Executor {
    pub fn new() -> Executor {
        let queue = Deque::<Arc<Tickable>>::new();
        let stealer = queue.stealer();

        let (tx, rx) = channel();
        let t_handle = thread::spawn(move || {
            // Spin, receive tasks, and take stuff from queue
            loop {
                match rx.recv() {
                    Ok(task) => {
                        queue.push(task);
                    }
                    Err(_err) => {
                        // TODO: Handle errors. The space that needs to be
                        // explored here is the failures that can result
                        // from message passing in Rust.
                    }
                }

                match queue.pop() {
                    Some(mut task) => {
                        match Arc::get_mut(&mut task) {
                            Some(mut_task) => mut_task.tick(),
                            None => (),
                        };
                        match task.get_state() {
                            &TaskState::Incomplete => {
                                // current philosophy: tasks are round robin.
                                // this will be a fun place to experiment and
                                // get better performance.
                                queue.push(task);
                            }
                            &TaskState::Unstarted => {
                                // this is unexpected, and may be an error
                            }
                            &TaskState::Complete => {
                                // maybe send a message that this is done?
                            }
                            &TaskState::Error => {
                                // current philosophy: errors should be handled
                                // by the publisher of the task. Might be worth
                                // adding some layers here to make the reason
                                // for failure easy to determine.
                            }
                        };
                    }
                    None => {
                        // TODO: could use some observability on how many times
                        // popped on an empty queue.
                    }
                };
            }
        });

        let executor = Executor {
            thread: t_handle,
            task_channel: tx,
            work_stealer: stealer,
        };
        executor
    }

    pub fn schedule(&self, task: Arc<Tickable>) {
        self.task_channel.send(task);
    }

    pub fn task_count(&self) -> usize {
        self.work_stealer.len()
    }
}
