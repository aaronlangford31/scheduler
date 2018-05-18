extern crate rand;
extern crate scheduler;
extern crate statrs;

use scheduler::cpupool::{CpuPool, SegregatedCpuPool, WorkStealingCpuPool};
use scheduler::task::{Task, TaskState};
use scheduler::waiter::{WaitResult, Waiter};
use std::time::{Duration, Instant, SystemTime};

mod data;

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
    if n == 0 {
        return 0;
    }
    let mut counter = 1;
    let mut last_prime = 2;

    while counter != n {
        last_prime = next_prime(last_prime);
        counter += 1;
    }
    last_prime
}

fn run_benchmark(n_cores: usize, n_tasks: usize, n_primes: usize) -> Duration {
    let pool = WorkStealingCpuPool::new(n_cores);
    let mut task_waiters: Vec<Waiter<WaitResult<usize>>> = Vec::with_capacity(n_tasks);

    let timer = SystemTime::now();
    for _i in 0..n_tasks {
        let mut task = Task::new(move || {
            let n = nth_prime(n_primes);
            (TaskState::Complete, Some(n))
        });
        let waiter = task.waiter().unwrap();
        let boxed_task = Box::new(task);
        pool.schedule(boxed_task);
        task_waiters.push(waiter);
    }

    for task in task_waiters {
        match task.await() {
            Ok(_) => (),
            Err(_) => (),
        }
    }
    timer.elapsed().unwrap()
}

fn run_benchmark_uncertain(n_cores: usize, task_data: Vec<(f64, f64)>) -> Vec<(u64, usize, u64)> {
    let pool = WorkStealingCpuPool::new(n_cores);

    let waiters: Vec<(u64, usize, Waiter<WaitResult<usize>>)> = task_data
        .into_iter()
        .map(|data| {
            // spin for a certain amount of time
            let now = Instant::now();
            let delay = (data.0 * 1000.0) as u64;
            let n = (data.1 * 10000.0) as usize;
            let until = Duration::from_nanos(delay);
            while now.elapsed() < until {}
            // create a task
            let mut task = Task::new(move || {
                let n = nth_prime(n);
                (TaskState::Complete, Some(n))
            });
            let waiter = task.waiter().unwrap();
            let boxed_task = Box::new(task);
            pool.schedule(boxed_task);
            (delay, n, waiter)
        })
        .collect();

    waiters
        .into_iter()
        .map(|(delay, size, waiter)| {
            let result = match waiter.await() {
                Ok(wait_result) => {
                    let seconds = wait_result.get_total_time().as_secs();
                    let nanos = wait_result.get_total_time().subsec_nanos();
                    (seconds * 1_000_000_000) + nanos as u64
                }
                Err(_) => 0,
            };
            (delay, size, result)
        })
        .collect()
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
    print_header();
    let elapsed = run_benchmark(8, 1024, 1024);
    println!("8\t1024\t\t1024\t\t{:?}", elapsed);
}

fn uniform_load_long_task() {
    print_header();
    let elapsed = run_benchmark(8, 1024, 8192);
    println!("8\t1024\t\t8192\t\t{:?}", elapsed);
}

fn low_freq_short_task_short_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 1.0 / 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 10.0,
        rate: 2.0,
    };
    let data = data::generate_task_data(10000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn high_freq_short_task_short_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 10.0,
        rate: 2.0,
    };
    let data = data::generate_task_data(10000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn low_freq_long_task_short_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 1.0 / 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 40.0,
        rate: 2.0,
    };
    let data = data::generate_task_data(10000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn high_freq_long_task_short_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 40.0,
        rate: 2.0,
    };
    let data = data::generate_task_data(10000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn low_freq_long_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 1.0 / 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 1.0,
        rate: 0.05,
    };
    let data = data::generate_task_data(10000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn high_freq_long_tail() {
    // expect a new task once every 100 microseconds
    let exp = data::F64exponentialUncertain { rate: 100.0 };
    // expected value of size param is 5, with low tail
    let gamma = data::F64gammaUncertain {
        shape: 1.0,
        rate: 0.05,
    };
    let data = data::generate_task_data(1000, exp, gamma);
    let results = run_benchmark_uncertain(6, data);
    results
        .into_iter()
        .for_each(|r| println!("{:?},{:?},{:?}", r.0, r.1, r.2));
}

fn print_header() {
    println!("Cores\t# of Tasks\tNth Prime\tTime");
}

fn main() {
    //uniform_load_short_task();
    //low_freq_short_task_short_tail();
    //high_freq_short_task_short_tail();
    //low_freq_long_task_short_tail();
    //high_freq_long_task_short_tail();
    //low_freq_long_tail();
    high_freq_long_tail();
}
