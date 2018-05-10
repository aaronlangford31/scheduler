use super::waiter::{Waiter, WaitResult};
use std::marker::Send;
use std::sync::mpsc::{channel, Sender};
use std::time::SystemTime;

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
    elapsed: u64,
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
            elapsed: 0,
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
        let timer = SystemTime::now();

        let (state, result) = (self._tick)();
        self.state = state;
        self.result = result;

        match timer.elapsed() {
            Ok(elapsed) => {
                self.elapsed += (elapsed.as_secs() * 1_000_000_000) + elapsed.subsec_nanos() as u64;
            }
            Err(_err) => {
                self.state = TaskState::Error;
            }
        }
    }

    fn get_state(&self) -> &TaskState {
        &self.state
    }

    fn complete(self: Box<Self>) {
        let this = *self;
        match this.result {
            Some(result) => match this.send_result_channel {
                Some(channel) => match channel.send(WaitResult::new(result, this.elapsed, this.ticks)) {
                    Ok(_) => (),
                    Err(_err) => println!("Error sending result: channel failure"),
                },
                None => println!("Error sending result: channel was None"),
            },
            None => println!("Error sending result: called complete when result is empty"),
        }
    }
}
