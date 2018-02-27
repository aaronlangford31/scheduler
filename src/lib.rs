extern crate crossbeam_deque;

mod cpupool;
mod executor;
mod task;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
