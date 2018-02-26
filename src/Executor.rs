use crossbeam_deque::Deque;
use std::sync::{Arc, mpsc};
use std::thread;
use super::Task::{TaskState, Tickable};

type AtomicTaskQueue = Arc<Deque<Arc<Tickable>>>;

pub struct Executor {
    thread: thread::JoinHandle<()>,
    work_queue: AtomicTaskQueue,
}

impl Executor {
    pub fn new() -> Executor {
        let (tx, rx) = mpsc::channel::<AtomicTaskQueue>();
        let t_handle = thread::spawn(move || {
            let q = rx.recv().unwrap();
            /*Spin and take stuff from queue*/
            while true {
                match q.pop() {
                    Some(task) => {
                        task.tick();
                        match task.get_state() {
                            &TaskState::Incomplete => {
                                // TODO: Convert this to a retry poll with time checks
                                q.push(task);
                            } // Could handle completed/failed tasks here, but not necessary
                              // Task futures will take care of that, and whoever scheduled
                              // the task will be responsible for taking care of the failure
                              // that is exposed through the future.
                        };
                    }
                };
            }
        });

        let queue = Arc::new(Deque::<Arc<Tickable>>::new());
        tx.send(queue.clone()).unwrap();

        let executor = Executor {
            thread: t_handle,
            work_queue: queue,
        };
        executor
    }

    pub fn schedule(&self, task: Arc<Tickable>) {
        self.work_queue.push(task);
    }

    pub fn task_count(&self) -> usize {
        self.work_queue.len()
    }
}
