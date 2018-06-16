#![feature(asm)]

extern crate crossbeam_deque;
extern crate libc;
extern crate time;

pub mod cpupool;
pub mod cycles;
pub mod executor;
pub mod task;
pub mod waiter;