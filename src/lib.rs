extern crate crossbeam_deque;

mod CpuPool;
mod Executor;
mod Task;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
