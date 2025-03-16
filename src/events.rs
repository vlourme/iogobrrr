use crate::{
    bindings::*,
    io_uring::{ConnInfo, ConnType, IoUring},
};
use std::{io, os::unix::io::RawFd};

pub enum Event {
    /// Multishot Accept: Accept multiple connections at once
    MultishotAccept(MultishotAcceptEvent),
    /// Accept: Accept a single connection
    Accept(AcceptEvent),
    /// Read: Read data from a socket
    Read(ReadEvent),
    /// Poll Add: Add a file descriptor to the poll set
    PollMultishot(PollAddEvent),
    /// Send: Send data to a socket
    Send(SendEvent),
    /// Close: Close a socket
    Close(CloseEvent),
}

pub struct MultishotAcceptEvent {
    pub listener: RawFd,
    pub addr: *mut sockaddr,
    pub addrlen: *mut socklen_t,
    pub flags: libc::c_int,
    pub sqe: Option<*mut io_uring_sqe>,
}

pub struct AcceptEvent {
    pub socket: RawFd,
    pub addr: *mut sockaddr,
    pub addrlen: *mut socklen_t,
    pub flags: libc::c_int,
}

pub struct ReadEvent {
    pub socket: RawFd,
    pub buffer: *mut u8,
    pub length: usize,
}

pub struct PollAddEvent {
    pub fd: RawFd,
    pub events: libc::c_short,
}

pub struct SendEvent {
    pub socket: RawFd,
    pub buffer: *const u8,
    pub length: usize,
    pub flags: u32,
}

pub struct CloseEvent {
    pub socket: RawFd,
}

impl IoUring {
    pub fn add_multishot_accept(&mut self, event: MultishotAcceptEvent) -> io::Result<()> {
        let sqe = self.get_sqe().ok_or(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Cannot add multishot accept event: no SQE available",
        ))?;

        unsafe {
            io_uring_prep_multishot_accept(
                sqe,
                event.listener,
                event.addr,
                event.addrlen,
                event.flags,
            );
        }

        self.set_sqe_data(
            sqe,
            ConnInfo {
                fd: event.listener,
                conn_type: ConnType::Accept,
            },
        );

        self.submit()?;
        Ok(())
    }

    /// Push an event to the SQE
    ///
    /// `add_event` should only contain `prep` functions
    /// with no return value.
    ///
    /// A optional `conn_info` can be provided to
    /// update the SQE data.
    pub fn add_event(&mut self, event: Event, conn_info: Option<ConnInfo>) -> io::Result<()> {
        let sqe = self.get_sqe().ok_or(io::Error::new(
            io::ErrorKind::WouldBlock,
            "No SQE available",
        ))?;

        match event {
            Event::PollMultishot(event) => {
                unsafe { io_uring_prep_poll_multishot(sqe, event.fd, event.events as _) };
            }
            Event::Accept(event) => unsafe {
                io_uring_prep_accept(sqe, event.socket, event.addr, event.addrlen, event.flags);
            },
            Event::MultishotAccept(event) => unsafe {
                io_uring_prep_multishot_accept(
                    event.sqe.unwrap_or(sqe),
                    event.listener,
                    event.addr,
                    event.addrlen,
                    event.flags,
                )
            },
            Event::Read(event) => unsafe {
                io_uring_prep_read(
                    sqe,
                    event.socket,
                    event.buffer as *mut libc::c_void,
                    event.length as u32,
                    0,
                );
            },
            Event::Send(event) => unsafe {
                io_uring_prep_send(
                    sqe,
                    event.socket,
                    event.buffer as *const libc::c_void,
                    event.length,
                    event.flags as libc::c_int,
                )
            },
            Event::Close(event) => unsafe {
                io_uring_prep_close(sqe, event.socket);
            },
        };

        if let Some(conn_info) = conn_info {
            self.set_sqe_data(sqe, conn_info);
        }

        self.submissions += 1;

        Ok(())
    }
}
