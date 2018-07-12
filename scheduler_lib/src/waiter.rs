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
    cpu_time: u64,
    total_time: u64,
    ticks: u32,
    n_steals: usize,
}

impl<T> WaitResult<T>
where
    T: Send,
{
    pub fn new(
        result: T,
        cpu_time: u64,
        total_time: u64,
        ticks: u32,
        n_steals: usize,
    ) -> WaitResult<T> {
        WaitResult {
            result,
            cpu_time,
            total_time,
            ticks,
            n_steals,
        }
    }

    pub fn get_cpu_time(&self) -> u64 {
        self.cpu_time
    }

    pub fn get_total_time(&self) -> u64 {
        self.total_time
    }

    pub fn get_ticks(&self) -> u32 {
        self.ticks
    }

    pub fn get_n_steals(&self) -> usize {
        self.n_steals
    }
}
