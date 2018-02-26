use std::marker::{Send, Sync};
use std::time::SystemTime;

pub enum TaskState {
    Unstarted,
    Complete,
    Incomplete,
    Error,
}

pub trait Tickable: Send + Sync {
    fn tick(&mut self);
    fn get_state(&self) -> &TaskState;
}

pub struct Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send + Sync,
    R: Send + Sync,
{
    // reference to the function, or work this thing needs to do.
    // a way to call poll on that thing. maybe need a Runnable? Why do you need a separate object for the actual function?
    // Poll needs to simply return status, Tick needs to actually advance the thing.
    ticks: u32,
    elapsed: u64,
    state: TaskState,
    result: Option<R>,
    _tick: F,
}

impl<F, R> Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send + Sync,
    R: Send + Sync,
{
    fn new(func: F) -> Task<F, R> {
        let task = Task {
            _tick: func,
            ticks: 0,
            elapsed: 0,
            state: TaskState::Unstarted,
            result: None,
        };
        task
    }

    // TODO: Verify the ownership here is correct.
    fn get_result(&self) -> Option<R> {
        self.result
    }
}

impl<F, R> Tickable for Task<F, R>
where
    F: FnMut() -> (TaskState, Option<R>) + Send + Sync,
    R: Send + Sync,
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
            Err(err) => {
                self.state = TaskState::Error;
            }
        }
    }

    fn get_state(&self) -> &TaskState {
        &self.state
    }
}

// I think that the work of advancing the task should be seperated
// from access to the state of the task... More to come on this.
