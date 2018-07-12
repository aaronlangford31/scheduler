use super::executor::Executor;
use rand::prelude::thread_rng;
use rand::Rng;

pub trait Dispatcher {
    fn flush(self) -> Vec<Executor>;
    fn inject_fleet(&mut self, fleet: Vec<Executor>);
    fn select(&self) -> Option<&Executor>;
}

pub struct RandomDispatcher {
    fleet: Vec<Executor>,
}

impl RandomDispatcher {
    pub fn new() -> RandomDispatcher {
        RandomDispatcher { fleet: vec![] }
    }
}

impl Dispatcher for RandomDispatcher {
    fn flush(self) -> Vec<Executor> {
        self.fleet
    }

    fn inject_fleet(&mut self, fleet: Vec<Executor>) {
        self.fleet = fleet;
    }

    fn select(&self) -> Option<&Executor> {
        thread_rng().choose(&self.fleet)
    }


}

pub struct LoadAwareDispatcher {
    fleet: Vec<Executor>,
}

impl LoadAwareDispatcher {
    pub fn new() -> LoadAwareDispatcher {
        LoadAwareDispatcher { fleet: vec![] }
    }
}

impl Dispatcher for LoadAwareDispatcher {
    fn flush(self) -> Vec<Executor> {
        self.fleet
    }

    fn inject_fleet(&mut self, fleet: Vec<Executor>) {
        self.fleet = fleet;
    }

    fn select(&self) -> Option<&Executor> {
        self.fleet
            .iter()
            .min_by_key(|executor| executor.count_tasks())
    }
}
