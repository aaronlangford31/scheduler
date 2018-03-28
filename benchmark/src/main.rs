#![feature(test)]
extern crate scheduler;
extern crate test;

use scheduler::cpupool::CpuPool;
use scheduler::task::{Iterable, Task, TaskState};
use std::sync::Arc;
use test::Bencher;

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

fn nth_prime(n: usize) -> usize {
    let mut counter = 1;
    let mut last_prime = 2;

    while counter != n {
        last_prime = next_prime(last_prime);
        counter += 1;
    }
    last_prime
}

fn run_benchmark(n_cores: usize, n_tasks: usize, n_primes: usize) {
    let pool = CpuPool::new(n_cores);
    let mut tasks: Vec<Arc<Iterable>> = Vec::with_capacity(n_tasks);

    for _i in 0..n_tasks {
        let task = Task::new(move || {
            let n = nth_prime(n_primes);
            println!("{}", n);
            (TaskState::Complete, Some(n))
        });
        let task_ref = Arc::new(task);

        pool.schedule(task_ref.clone());
        tasks.push(task_ref);
    }

    for task in tasks {
        let mut state = task.get_state();
        loop {
            match task.get_state() {
                &TaskState::Complete | &TaskState::Error => {
                    println!("A task has haulted");
                    break;
                }
                &TaskState::Incomplete | &TaskState::Unstarted => {}
            }
        }
    }
}

#[test]
fn nth_prime_test() {
    assert_eq!(3, nth_prime(2));
    assert_eq!(8161, nth_prime(1024));
}

#[test]
fn is_prime_test() {
    assert_eq!(false, is_prime(15));
    assert_eq!(false, is_prime(2875));
    assert_eq!(true, is_prime(29));
}

#[bench]
fn n1024_prime_benchmark(bencher: &mut Bencher) {
    bencher.iter(|| {
        nth_prime(1024);
    });
}

#[bench]
fn n2048_prime_benchmark(bencher: &mut Bencher) {
    bencher.iter(|| {
        nth_prime(8192);
    });
}

fn uniform_load_short_task() {
    run_benchmark(4, 1024, 1024);
}

fn uniform_load_long_task() {
    run_benchmark(4, 1024, 8192);
}

fn main() {
    uniform_load_short_task();
}
