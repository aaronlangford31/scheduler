use crossbeam_deque::{Deque};
use std::sync::Arc;
use std::thread;
use super::Task::{Tickable, TaskState};

pub struct Executor {
  thread: thread::Thread,
  work_queue: Arc<Deque<Arc<Tickable>>>,
}

impl Executor {
  pub fn new() -> Executor {
    let queue = Arc::new(Deque::<Arc<Tickable>>::new());
    let cloned_queue = queue.clone();
    let thread = thread::spawn(move || {
      /*Spin and take stuff from queue*/
      while true {
        let task = cloned_queue.steal();
        task.tick();
        match task.get_state() {
          TaskState::Incomplete => {
            // TODO: Convert this to a retry poll with time checks
            cloned_queue.enqueue(task);
          }
          // Could handle completed/failed tasks here, but not necessary
          // Task futures will take care of that, and whoever scheduled
          // the task will be responsible for taking care of the failure
          // that is exposed through the future.
        };
      }
    });
    let executor = Executor {
      thread,
      workQueue: queue,
    };
    executor
  }

  pub fn schedule(&self, task: Arc<Tickable>) {
    self.workQueue.enqueue(task);
  }

  pub fn task_count(&self) -> usize {
    self.work_queue.len()
  }
}
