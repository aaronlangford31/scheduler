use super::task::{Iterable, TaskState};
use crossbeam_deque::{Deque, Steal, Stealer};
use libc::{cpu_set_t, pthread_setaffinity_np, CPU_SET, CPU_ZERO};
use std::borrow::BorrowMut;
use std::cell::Cell;
use std::mem;
use std::os::unix::thread::JoinHandleExt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, SendError, Sender};
use std::sync::Arc;
use std::thread;

pub struct Executor {
    cpu: usize,
    busy: Arc<AtomicBool>,
    not_acked_tasks: Cell<usize>,
    thread: thread::JoinHandle<()>,
    work_channel: Sender<Box<Iterable>>,
    work_acknowledge_channel: Receiver<()>,
    work_queue_peeker: Stealer<Box<Iterable>>,
    stealer_channel: Sender<Stealer<Box<Iterable>>>,
}

impl Executor {
    pub fn new(cpu: usize, n_stealers: usize) -> (Executor, Stealer<Box<Iterable>>) {
        let work_queue = Deque::<Box<Iterable>>::new();
        let work_stealer = work_queue.stealer();
        let work_queue_peeker = work_queue.stealer();
        let (send_work_channel, receive_work_channel) = channel();
        let (send_stealer_channel, receive_stealer_channel) = channel();
        let (send_acknowlege_work_channel, receive_acknowlege_work_channel) = channel();
        let busy_flag = Arc::new(AtomicBool::new(false));
        let busy_flag_clone = busy_flag.clone();

        let t_handle = thread::spawn(move || {
            let mut inner_executor = InnerExecutor::new(
                cpu,
                busy_flag_clone,
                work_queue,
                receive_work_channel,
                receive_stealer_channel,
                send_acknowlege_work_channel,
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
            busy: busy_flag,
            not_acked_tasks: Cell::new(0),
            thread: t_handle,
            work_channel: send_work_channel,
            work_acknowledge_channel: receive_acknowlege_work_channel,
            work_queue_peeker,
            stealer_channel: send_stealer_channel,
        };
        (executor, work_stealer)
    }

    pub fn schedule(&self, task: Box<Iterable>) -> Result<(), SendError<Box<Iterable>>> {
        self.not_acked_tasks.set(self.not_acked_tasks.get() + 1);
        self.work_channel.send(task)
    }

    pub fn send_stealer(
        &self,
        stealer: Stealer<Box<Iterable>>,
    ) -> Result<(), SendError<Stealer<Box<Iterable>>>> {
        self.stealer_channel.send(stealer)
    }

    pub fn get_cpu(&self) -> usize {
        self.cpu
    }

    pub fn count_tasks(&self) -> usize {
        // check acknowledged tasks, and change local task
        // TODO: encapsulate the send/ack channels into 1 channel object
        let acked_tasks = self.work_acknowledge_channel.try_iter().count();
        self.not_acked_tasks
            .set(self.not_acked_tasks.get() - acked_tasks);

        // return the size of the work queue + size of tasks not acked yet
        // + 1 if the executor is busy.
        // HEADS UP: this count may not be precisely correct once read:
        // the underlying thread may acknowledge tasks at any point and
        // alter the size of the work queue. If work stealing is enabled,
        // then other threads may also alter the size of the work queue
        if self.busy.load(Ordering::Relaxed) {
            self.not_acked_tasks.get() + self.work_queue_peeker.len() + 1
        } else {
            self.not_acked_tasks.get() + self.work_queue_peeker.len()
        }
    }
}

// TODO: Implement Drop for Executor

struct InnerExecutor {
    cpu: usize,
    busy: Arc<AtomicBool>,
    work_queue: Deque<Box<Iterable>>,
    receive_work_channel: Receiver<Box<Iterable>>,
    receive_stealer_channel: Receiver<Stealer<Box<Iterable>>>,
    acknowlege_work_channel: Sender<()>,
    stealers: Vec<Stealer<Box<Iterable>>>,
}

impl InnerExecutor {
    fn new(
        cpu: usize,
        busy: Arc<AtomicBool>,
        work_queue: Deque<Box<Iterable>>,
        receive_work_channel: Receiver<Box<Iterable>>,
        receive_stealer_channel: Receiver<Stealer<Box<Iterable>>>,
        acknowlege_work_channel: Sender<()>,
        n_stealers: usize,
    ) -> InnerExecutor {
        let stealers = Vec::with_capacity(n_stealers);
        let mut inner_executor = InnerExecutor {
            cpu,
            busy,
            work_queue,
            receive_work_channel,
            receive_stealer_channel,
            acknowlege_work_channel,
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
            self.acknowlege_work_channel.send(()).unwrap();
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
        match self.work_queue.steal() {
            Steal::Data(mut task) => {
                self.busy.store(true, Ordering::Relaxed);
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
                self.busy.store(false, Ordering::Relaxed);
                true
            }
            Steal::Retry => false,
            Steal::Empty => false,
        }
    }

    fn steal_work(&mut self) {
        let stealer = self.stealers.iter().max_by_key(|s| s.len());
        match stealer {
            Some(stealer) => match stealer.steal() {
                Steal::Data(mut task) => {
                    task.mark_stolen();
                    self.work_queue.push(task);
                }
                Steal::Empty => {}
                Steal::Retry => {}
            },
            None => {}
        };
    }
}
