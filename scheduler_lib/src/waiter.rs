use std::sync::mpsc::Receiver;

pub struct Waiter<T>
where
    T: Send,
{
    receive_result_channel: Receiver<T>,
}

impl<T> Waiter<T>
where
    T: Send,
{
    pub fn new(channel: Receiver<T>) -> Waiter<T> {
        Waiter {
            receive_result_channel: channel,
        }
    }

    pub fn await(&self) -> Result<T, ()> {
        match self.receive_result_channel.recv() {
            Ok(result) => Ok(result),
            Err(_err) => Err(()),
        }
    }
}

pub struct WaitResult<T>
where
    T: Send,
{
    result: T,
    elapsed: u64,
    ticks: u32
}

impl<T> WaitResult<T>
where
    T: Send,
{
    pub fn new(result: T, elapsed: u64, ticks: u32) -> WaitResult<T> {
        WaitResult {
            result,
            elapsed,
            ticks,
        }
    }

    pub fn get_elapsed(&self) -> u64 {
        self.elapsed
    }
}