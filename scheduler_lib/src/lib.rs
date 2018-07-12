#![feature(asm)]

extern crate crossbeam_deque;
extern crate libc;
extern crate rand;
extern crate time;

pub mod cpupool;
pub mod cycles;
pub mod dispatcher;
pub mod executor;
pub mod task;
pub mod waiter;
