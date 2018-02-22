use std::time::SystemTime;

pub enum TaskState {
  Unstarted,
  Complete,
  Incomplete,
  Error,
}

pub trait Tickable {
  fn tick(&mut self);
  fn get_state(&self) -> &TaskState;
}

pub struct Task<R> {
  // reference to the function, or work this thing needs to do.
  // a way to call poll on that thing. maybe need a Runnable? Why do you need a separate object for the actual function?
  // Poll needs to simply return status, Tick needs to actually advance the thing.
  ticks: u32,
  elapsed: u64,
  state: TaskState,
  result: Option<R>,
  _tick: FnMut() -> (TaskState, Option<R>),
}

impl<R> Task<R> {
  fn new(func: FnMut() -> TaskState) -> Task<R> {
    let task = Task {
      _tick: func,
      ticks: 0,
      elapsed: 0,
      state: TaskState::Unstarted,
      result: None
    };
    task
  }

  // TODO: Verify the ownership here is correct.
  fn get_result(&self) -> Option<R> {
    self.result
  }
}

impl<R> Tickable for Task<R> {
  fn tick(&mut self) {
    self.ticks += 1;
    let timer = SystemTime::now();
    
    let (state, result)  = self._tick();
    self.state = state;
    self.result = result;

    self.elapsed += (timer.duration.as_sec() * 1e6) + (timer.duration.subsec_micros);
  }

  fn get_state(&self) -> &TaskState {
    &self.state
  }
  
}

// I think that the work of advancing the task should be seperated
// from access to the state of the task... More to come on this.
