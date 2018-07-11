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
mod primes;

fn warm_up(pool: &CpuPool) {
    let waiters: Vec<(usize, Waiter<WaitResult<usize>>)> = vec![1000 as usize, 1000]
        .into_iter()
        .map(|data| {
            // create a task
            let mut prime_calculation = primes::Primatizer::new(data);
            let mut task = Task::new(move || {
                prime_calculation.step(data);
                (
                    TaskState::Complete,
                    Some(prime_calculation.get_last_prime()),
                )
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
    n_threads: usize,
    n_cores: usize,
    n_elephants: usize,
    task_data: Vec<(u64, usize)>,
) -> Vec<(u64, usize, f64, usize)> {
    let pool = WorkStealingCpuPool::new(n_threads, n_cores);

    warm_up(&pool);
    let big_task_size = 100_000;
    let big_task_step = 100_000;

    let big_tasks: Vec<Waiter<WaitResult<usize>>> = vec![big_task_size; n_elephants]
        .into_iter()
        .map(|_| {
            let mut prime_calculation = primes::Primatizer::new(big_task_size);
            let mut task = Task::new(move || match prime_calculation.step(big_task_step) {
                TaskState::Complete => (
                    TaskState::Complete,
                    Some(prime_calculation.get_last_prime()),
                ),
                TaskState::Incomplete => (TaskState::Incomplete, None),
                TaskState::Error => (TaskState::Error, None),
                TaskState::Unstarted => (TaskState::Unstarted, None),
            });
            let waiter = task.waiter().unwrap();
            let sched_result = pool.schedule(Box::new(task));
            match sched_result {
                Ok(cpu) => println!("{}", cpu),
                Err(_) => panic!("Schedule failure"),
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
            let mut prime_calculation = primes::Primatizer::new(n);
            let mut task = Task::new(move || {
                prime_calculation.step(n);
                (
                    TaskState::Complete,
                    Some(prime_calculation.get_last_prime()),
                )
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

fn fixed_size_run(frequency: u64, size: usize, n_tasks: usize, n_threads: usize, n_cores: usize) {
    let freq = vec![frequency; n_tasks];
    let sizes = vec![size; n_tasks];
    let data: Vec<(u64, usize)> = freq.into_iter().zip(sizes).collect();

    let results = run_benchmark(n_threads, n_cores, 0, data);

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

fn elephant_run(
    frequency: u64,
    size: usize,
    n_tasks: usize,
    n_threads: usize,
    n_cores: usize,
    n_elephants: usize,
) {
    let freq = vec![frequency; n_tasks];
    let sizes = vec![size; n_tasks];
    let data: Vec<(u64, usize)> = freq.into_iter().zip(sizes).collect();

    let results = run_benchmark(n_threads, n_cores, n_elephants, data);

    let mut hist = Histogram::new();
    let mut total_steals = 0;
    results.into_iter().for_each(|(_, _, time, n_steals)| {
        // println!("{}", time);
        &hist.increment((time * 1e9) as u64);
        total_steals += n_steals;
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
    n_threads: usize,
    n_cores: usize,
) {
    let data = data::generate_task_data(n_tasks, frequency, size)
        .into_iter()
        .map(|d: (f64, f64)| (d.0 as u64, d.1 as usize))
        .collect();
    let results = run_benchmark(n_threads, n_cores, 0, data);

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
    if args.len() != 7 {
        println!(
            "Usage: {} <delay (ns): u64> <nth_prime: usize> <n_tasks: usize> <n_threads: usize> <n_cores: usize> <n_elephants: usize>",
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
    let n_threads: usize = args[4].parse().unwrap();
    let n_cores: usize = args[5].parse().unwrap();
    let n_elephants: usize = args[6].parse().unwrap();

    elephant_run(
        args[1].parse().unwrap(),
        args[2].parse().unwrap(),
        n_tasks,
        n_threads,
        n_cores,
        n_elephants,
    );
}
