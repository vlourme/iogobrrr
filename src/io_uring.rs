use std::io;
use std::os::fd::RawFd;

use crate::bindings::*;

pub struct IoUring {
    ring: io_uring,
    pub submissions: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum ConnType {
    Accept,
    Read,
    Write,
    Close,
}

#[derive(Debug, Clone, Copy)]
pub struct ConnInfo {
    pub fd: RawFd,
    pub conn_type: ConnType,
}

impl IoUring {
    /// Create a new io_uring instance
    pub fn new(entries: u32) -> io::Result<Self> {
        let mut ring = unsafe { std::mem::zeroed() };
        let ret = unsafe { io_uring_queue_init(entries, &mut ring, 0) };
        if ret < 0 {
            return Err(io::Error::from_raw_os_error(-ret));
        }

        Ok(Self {
            ring,
            submissions: 0,
        })
    }

    pub fn set_sqe_data(&self, sqe: *mut io_uring_sqe, conn_info: ConnInfo) {
        let conn_info = Box::new(conn_info);
        unsafe {
            io_uring_sqe_set_data(sqe, Box::into_raw(conn_info) as *mut _);
        }
    }

    /// Get a SQE
    pub fn get_sqe(&mut self) -> Option<*mut io_uring_sqe> {
        let sqe = unsafe { io_uring_get_sqe(&mut self.ring) };
        if sqe.is_null() {
            return None;
        }
        Some(sqe)
    }

    pub fn set_cqe_seen(&mut self, cqe: *mut io_uring_cqe) -> io::Result<()> {
        unsafe { io_uring_cqe_seen(&mut self.ring, cqe) };
        Ok(())
    }

    /// Submit the pending SQEs
    pub fn submit(&mut self) -> io::Result<()> {
        if self.submissions == 0 {
            println!("No submissions, skipping");
            return Ok(());
        }

        let ret = unsafe { io_uring_submit(&mut self.ring) };
        if ret < 0 {
            return Err(io::Error::from_raw_os_error(-ret));
        }
        println!("Submitted (real: {}, expected: {})", ret, self.submissions);
        self.submissions = 0;
        Ok(())
    }

    /// Wait for a completion event
    pub fn wait_cqe(&mut self) -> io::Result<*mut io_uring_cqe> {
        let mut cqe: *mut io_uring_cqe = std::ptr::null_mut();
        let ret = unsafe { io_uring_wait_cqe(&mut self.ring, &mut cqe) };
        if ret < 0 {
            return Err(io::Error::from_raw_os_error(-ret));
        }
        Ok(cqe)
    }
}
