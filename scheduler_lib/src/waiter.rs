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
    pub fn new(receive_result_channel: Receiver<T>) -> Waiter<T> {
        Waiter {
            receive_result_channel,
        }
    }

    pub fn await(&self) -> Result<T, ()> {
        match self.receive_result_channel.recv() {
            Ok(result) => Ok(result),
            Err(err) => Err(()),
        }
    }
}
