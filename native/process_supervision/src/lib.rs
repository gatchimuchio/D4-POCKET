#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(windows)]
mod windows_job;

#[cfg(windows)]
pub use windows_job::Job;

#[cfg(windows)]
pub const CREATE_SUSPENDED: u32 = 0x0000_0004;
