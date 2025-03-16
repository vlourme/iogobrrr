use std::os::fd::RawFd;

use crate::{bindings::*, io_uring::ConnInfo};

/// Returns a null address and address length pointer
pub fn get_null_addr_ptr() -> (*mut sockaddr, *mut u32) {
    let addr = unsafe { std::mem::zeroed() };
    let addrlen = std::ptr::null_mut();
    (addr, addrlen)
}

/// Unwrap a CQE into a user_data and socket
pub fn unwrap_cqe(cqe: *mut io_uring_cqe) -> (u64, RawFd) {
    let user_data = unsafe { (*cqe).user_data };
    let socket = unsafe { (*cqe).res };

    (user_data, socket)
}

pub fn get_conn_info(user_data: u64) -> Option<ConnInfo> {
    if user_data == 0 {
        return None;
    }

    // Just read the data without taking ownership or freeing it
    let conn_info = unsafe { &*(user_data as *const ConnInfo) };

    // Return a copy of the data
    Some(ConnInfo {
        fd: conn_info.fd,
        conn_type: conn_info.conn_type.clone(),
    })
}
