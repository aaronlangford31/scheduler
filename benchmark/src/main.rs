extern crate histogram;
extern crate rand;
extern crate scheduler;
extern crate statrs;

use histogram::Histogram;
use scheduler::cpupool::{CpuPool, SegregatedCpuPool, WorkStealingCpuPool};
use scheduler::cycles::{rdtsc, to_seconds};
use scheduler::task::{Task, TaskState};
use scheduler::waiter::{WaitResult, Waiter};
use std::env;

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

fn warm_up(pool: &CpuPool) {
    let waiters: Vec<(usize, Waiter<WaitResult<usize>>)> = vec![1000 as usize, 1000]
        .into_iter()
        .map(|data| {
            // create a task
            let mut task = Task::new(move || {
                let n = nth_prime(data);
                (TaskState::Complete, Some(n))
            });
            let waiter = task.waiter().unwrap();
            let boxed_task = Box::new(task);
            pool.schedule(boxed_task);
            (data, waiter)
        })
        .collect();

    let _: Vec<(usize, u64)> = waiters
        .into_iter()
        .map(|(size, waiter)| {
            let result = match waiter.await() {
                Ok(wait_result) => wait_result.get_total_time(),
                Err(_) => 0,
            };
            (size, result)
        })
        .collect();
}

fn run_benchmark(
    n_cores: usize,
    n_elephants: usize,
    task_data: Vec<(u64, usize)>,
) -> Vec<(u64, usize, f64, usize)> {
    let pool = SegregatedCpuPool::new(n_cores);

    warm_up(&pool);
    let big_task_size = 1_000_000;

    let big_tasks: Vec<Waiter<WaitResult<usize>>> = vec![big_task_size; n_elephants]
        .into_iter()
        .map(|_| {
            let mut task = Task::new(move || {
                let n = nth_prime(big_task_size);
                (TaskState::Complete, Some(n))
            });
            let waiter = task.waiter().unwrap();
            let sched_result = pool.schedule(Box::new(task));
            match sched_result {
                Ok(cpu) => println!("{}", cpu),
                Err(_) => panic!("Schedule failure")
            }

            waiter
        })
        .collect();

    let waiters: Vec<(u64, usize, Waiter<WaitResult<usize>>)> = task_data
        .into_iter()
        .map(|data| {
            // spin for a certain amount of time
            let now = (to_seconds(rdtsc()) * 1e9) as u64;
            let delay = data.0;
            let n = data.1;
            let until = now + delay;
            while ((to_seconds(rdtsc()) * 1e9) as u64) < until {}
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

    let b: Vec<f64> = big_tasks
        .into_iter()
        .map(|waiter| {
            let result = match waiter.await() {
                Ok(wait_result) => {
                    let cycles = wait_result.get_total_time();
                    println!("{} {}", to_seconds(cycles), wait_result.get_n_steals());
                    to_seconds(cycles)
                }
                Err(_) => 0.,
            };
            result
        })
        .collect();

    waiters
        .into_iter()
        .map(|(delay, size, waiter)| {
            let (result, n_steals) = match waiter.await() {
                Ok(wait_result) => {
                    let cycles = wait_result.get_total_time();
                    let steals = wait_result.get_n_steals();
                    (to_seconds(cycles), steals)
                }
                Err(_) => panic!("Error waiting for task"),
            };
            (delay, size, result, n_steals)
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

fn fixed_size_run(frequency: u64, size: usize, n_tasks: usize, n_cores: usize) {
    let freq = vec![frequency; n_tasks];
    let sizes = vec![size; n_tasks];
    let data: Vec<(u64, usize)> = freq.into_iter().zip(sizes).collect();

    let results = run_benchmark(n_cores, 0, data);

    let mut hist = Histogram::new();
    results.into_iter().for_each(|(_, _, time, _)| {
        // println!("{}", time);
        &hist.increment((time * 1e9) as u64);
    });

    println!(
        "{}\t{}",
        hist.percentile(50.0).unwrap(),
        hist.percentile(99.0).unwrap()
    );
}

fn elephant_run(frequency: u64, size: usize, n_tasks: usize, n_cores: usize) {
    let freq = vec![frequency; n_tasks];
    let sizes = vec![size; n_tasks];
    let data: Vec<(u64, usize)> = freq.into_iter().zip(sizes).collect();

    let results = run_benchmark(n_cores, 4, data);

    let mut hist = Histogram::new();
    let mut total_steals = 0;
    results.into_iter().for_each(|(_, _, time, n_steals)| {
        // println!("{}", time);
        &hist.increment((time * 1e9) as u64);
        total_steals += n_steals;
        if(n_steals > 1) { println!("Hot potato! {} {}", n_steals, (time * 1e9) as u64) }
    });

    println!(
        "{}\t{}\t{}",
        hist.percentile(50.0).unwrap(),
        hist.percentile(99.0).unwrap(),
        total_steals
    );
}

fn exp_gamma_run(
    frequency: data::F64exponentialUncertain,
    size: data::F64gammaUncertain,
    n_tasks: usize,
    n_cores: usize,
) {
    let data = data::generate_task_data(n_tasks, frequency, size)
        .into_iter()
        .map(|d: (f64, f64)| (d.0 as u64, d.1 as usize))
        .collect();
    let results = run_benchmark(n_cores, 0, data);

    let mut hist = Histogram::new();
    results.into_iter().for_each(|(_, _, time, _)| {
        &hist.increment((time * 1e9) as u64);
    });

    println!(
        "{} {} {} {} {}",
        hist.percentile(1.0).unwrap(),
        hist.percentile(25.0).unwrap(),
        hist.percentile(50.0).unwrap(),
        hist.percentile(75.0).unwrap(),
        hist.percentile(99.0).unwrap()
    );
}

fn print_header() {
    println!("Cores\t# of Tasks\tNth Prime\tTime");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        println!(
            "Usage: {} <delay (ns): u64> <nth_prime: usize> <n_tasks: usize> <n_cores: usize>",
            args[0]
        );
        return;
    }
    //    let frequency = data::F64exponentialUncertain {
    //        rate: args[1].parse().unwrap(),
    //        scale: args[2].parse().unwrap(),
    //    };
    //    let size = data::F64gammaUncertain {
    //        shape: args[3].parse().unwrap(),
    //        rate: args[4].parse().unwrap(),
    //        scale: args[5].parse().unwrap(),
    //    };
    let n_tasks: usize = args[3].parse().unwrap();
    let n_cores: usize = args[4].parse().unwrap();

    elephant_run(
        args[1].parse().unwrap(),
        args[2].parse().unwrap(),
        n_tasks,
        n_cores,
    );
}
