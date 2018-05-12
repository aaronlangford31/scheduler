use super::task::{Iterable, TaskState};
use crossbeam_deque::{Deque, Steal, Stealer};
use libc::{cpu_set_t, pthread_setaffinity_np, CPU_SET, CPU_ZERO};
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
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
            let mut inner_executor = InnerExecutor::new(
                cpu,
                work_queue,
                receive_work_channel,
                receive_stealer_channel,
                n_stealers,
            );
            inner_executor.run();
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

struct InnerExecutor {
    cpu: usize,
    work_queue: Deque<Box<Iterable>>,
    receive_work_channel: Receiver<Box<Iterable>>,
    receive_stealer_channel: Receiver<Stealer<Box<Iterable>>>,
    stealers: Vec<Stealer<Box<Iterable>>>,
}

impl InnerExecutor {
    fn new(
        cpu: usize,
        work_queue: Deque<Box<Iterable>>,
        receive_work_channel: Receiver<Box<Iterable>>,
        receive_stealer_channel: Receiver<Stealer<Box<Iterable>>>,
        n_stealers: usize,
    ) -> InnerExecutor {
        let stealers = Vec::with_capacity(n_stealers);
        let mut inner_executor = InnerExecutor {
            cpu,
            work_queue,
            receive_work_channel,
            receive_stealer_channel,
            stealers,
        };
        inner_executor.receive_stealers();
        inner_executor
    }

    fn run(&mut self) {
        loop {
            self.receive_work();
            let did_work = self.do_work();
            if !did_work {
                self.steal_work();
            }
        }
    }

    fn receive_work(&mut self) {
        // TODO: handle errors that do not have to do with no messages
        while let Ok(task) = self.receive_work_channel.try_recv() {
            self.work_queue.push(task);
        }
        if self.work_queue.len() > 0 {
            println!("received work. work queue len {}", self.work_queue.len());
        }
    }

    fn receive_stealers(&mut self) {
        for _ in 0..self.stealers.capacity() {
            match self.receive_stealer_channel.recv() {
                Ok(stealer) => {
                    self.stealers.push(stealer);
                }
                Err(_) => {
                    println!("Failed to recieve stealer.");
                }
            }
        }
    }

    fn do_work(&mut self) -> bool {
        match self.work_queue.pop() {
            Some(mut task) => {
                task.tick();
                match task.get_state() {
                    &TaskState::Incomplete => {
                        self.work_queue.push(task);
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
                true
            }
            None => false,
        }
    }

    fn steal_work(&mut self) {
        let stealer = self.stealers.iter().max_by_key(|s| s.len());
        match stealer {
            Some(stealer) => match stealer.steal() {
                Steal::Data(task) => {
                    self.work_queue.push(task);
                }
                Steal::Empty => {}
                Steal::Retry => {}
            },
            None => {}
        };
    }
}
