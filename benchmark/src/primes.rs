use scheduler::cycles::{rdtsc, to_seconds};
use scheduler::task::TaskState;
use std::cmp::min;

pub struct Primatizer {
    nth_prime: usize,
    counter: usize,
    last_prime: usize,
}

impl Primatizer {
    pub fn new(n: usize) -> Primatizer {
        Primatizer {
            nth_prime: n,
            counter: 1,
            last_prime: 2,
        }
    }

    pub fn step(&mut self, n_steps: usize) -> TaskState {
        let until = min(self.nth_prime, self.counter + n_steps);
        while self.counter < until {
            self.last_prime = next_prime(self.last_prime);
            self.counter += 1;
        }

        if self.counter == self.nth_prime {
            TaskState::Complete
        } else {
            // Primatizer::preempt()
            TaskState::Incomplete
        }
    }

    pub fn get_last_prime(&self) -> usize {
        self.last_prime
    }

    fn preempt() -> TaskState {
        let now = (to_seconds(rdtsc()) * 1e9) as u64;
        let until = now + 250_000; // 250 microseconds to simulate cost of preemption
        while ((to_seconds(rdtsc()) * 1e9) as u64) < until {}

        TaskState::Incomplete
    }
}

fn is_prime(n: usize) -> bool {
    match n {
        0 | 1 => false,
        2 | 3 => true,
        _ => {
            if n % 2 == 0 {
                false
            } else {
                let n_root = (n as f64).sqrt() as usize + 1;
                !(3..n_root).any(|x: usize| n % x == 0)
            }
        }
    }
}

fn next_prime(start_from: usize) -> usize {
    // increment until you find a prime
    let mut curr = start_from + 1;
    while !is_prime(curr) {
        curr += 1;
    }
    curr
}

#[test]
fn nth_prime_test() {
    let mut a = Primatizer::new(2);
    while match a.step(1) {
        TaskState::Complete => false,
        _ => true,
    } {}

    let mut b = Primatizer::new(1024);
    while match b.step(100) {
        TaskState::Complete => false,
        _ => true,
    } {}

    let mut c = Primatizer::new(1024);
    c.step(1024);
    assert_eq!(3, a.get_last_prime());
    assert_eq!(8161, b.get_last_prime());
    assert_eq!(8161, c.get_last_prime())
}

#[test]
fn is_prime_test() {
    assert_eq!(false, is_prime(15));
    assert_eq!(false, is_prime(2875));
    assert_eq!(true, is_prime(29));
}
