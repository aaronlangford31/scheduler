use super::waiter::{WaitResult, Waiter};
use cycles::rdtsc;
use std::marker::Send;
use std::sync::mpsc::{channel, Sender};

pub enum TaskState {
    Unstarted,
    Complete,
    Incomplete,
    Error,
}

pub trait Iterable: Send {
    fn tick(&mut self);
    fn get_state(&self) -> &TaskState;
    fn complete(self: Box<Self>);
    fn mark_stolen(&mut self);
}

pub struct Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send,
    R: Send,
{
    // reference to the function, or work this thing needs to do.
    // a way to call poll on that thing. maybe need a Runnable? Why do you need a separate object for the actual function?
    // Poll needs to simply return status, Tick needs to actually advance the thing.
    ticks: u32,
    n_steals: usize,
    cpu_time: u64,
    birthday: u64,
    state: TaskState,
    result: Option<R>,
    send_result_channel: Option<Sender<WaitResult<R>>>,
    _tick: F,
}

impl<F, R> Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send,
    R: Send,
{
    pub fn new(func: F) -> Task<F, R> {
        let task = Task {
            _tick: func,
            ticks: 0,
            n_steals: 0,
            cpu_time: 0,
            birthday: rdtsc(),
            state: TaskState::Unstarted,
            result: None,
            send_result_channel: None,
        };
        task
    }

    pub fn waiter(&mut self) -> Result<Waiter<WaitResult<R>>, ()> {
        match self.send_result_channel {
            Some(_) => Err(()),
            None => {
                let (sender, receiver) = channel();
                self.send_result_channel = Some(sender);

                let waiter = Waiter::new(receiver);
                Ok(waiter)
            }
        }
    }
}

impl<F, R> Iterable for Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send,
    R: Send,
{
    fn tick(&mut self) {
        self.ticks += 1;
        let start = rdtsc();

        let (state, result) = (self._tick)();
        self.state = state;
        self.result = result;

        let end = rdtsc();
        self.cpu_time += end - start;
    }

    fn get_state(&self) -> &TaskState {
        &self.state
    }

    fn complete(self: Box<Self>) {
        let this = *self;
        match this.result {
            Some(result) => match this.send_result_channel {
                Some(channel) => {
                    let total_time = rdtsc() - this.birthday;
                    match channel.send(WaitResult::new(
                        result,
                        this.cpu_time,
                        total_time,
                        this.ticks,
                        this.n_steals,
                    )) {
                        Ok(_) => (),
                        Err(_err) => println!("Error sending result: channel failure"),
                    }
                }
                None => println!("Error sending result: channel was None"),
            },
            None => println!("Error sending result: called complete when result is empty"),
        }
    }

    fn mark_stolen(&mut self) {
        self.n_steals += 1;
    }
}
