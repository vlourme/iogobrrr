use std::{io, net::TcpListener, os::fd::AsRawFd};

use iogobrrr::events::{
    CloseEvent, Event, MultishotAcceptEvent, PollAddEvent, ReadEvent, SendEvent,
};
use iogobrrr::io_uring::{ConnInfo, ConnType, IoUring};
use iogobrrr::utils;

fn main() -> io::Result<()> {
    let mut ring = IoUring::new(8)?;

    let listener = TcpListener::bind("127.0.0.1:8080")?;
    listener.set_nonblocking(true)?;
    println!("listener: {:?}", listener);

    ring.add_event(
        Event::PollMultishot(PollAddEvent {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
        }),
        None,
    )?;

    let (addr, addrlen) = utils::get_null_addr_ptr();

    ring.add_multishot_accept(MultishotAcceptEvent {
        listener: listener.as_raw_fd(),
        addr,
        addrlen,
        flags: 0,
        sqe: None,
    })?;

    let mut buf = vec![0; 1024];
    loop {
        let cqe = match ring.wait_cqe() {
            Ok(cqe) => cqe,
            Err(_) => {
                continue;
            }
        };

        let (user_data, socket) = utils::unwrap_cqe(cqe);

        let conn_info = match utils::get_conn_info(user_data) {
            Some(conn_info) => conn_info,
            None => {
                ring.set_cqe_seen(cqe)?;
                continue;
            }
        };

        if socket < 0 {
            ring.set_cqe_seen(cqe)?;
            continue;
        }

        match conn_info.conn_type {
            ConnType::Accept => {
                ring.add_event(
                    Event::Read(ReadEvent {
                        buffer: buf.as_mut_ptr(),
                        socket,
                        length: buf.len(),
                    }),
                    Some(ConnInfo {
                        fd: socket,
                        conn_type: ConnType::Write,
                    }),
                )?;

                println!("Accepted connection: {:?}", socket);
            }
            ConnType::Write => {
                ring.add_event(
                    Event::Send(SendEvent {
                        socket: conn_info.fd,
                        buffer: buf.as_ptr(),
                        length: buf.len(),
                        flags: 0,
                    }),
                    Some(ConnInfo {
                        fd: conn_info.fd,
                        conn_type: ConnType::Read,
                    }),
                )?;

                println!("Sent data: {:?}", String::from_utf8_lossy(&buf));
            }
            ConnType::Read => {
                ring.add_event(
                    Event::Read(ReadEvent {
                        buffer: buf.as_mut_ptr(),
                        socket: conn_info.fd,
                        length: buf.len(),
                    }),
                    Some(ConnInfo {
                        fd: conn_info.fd,
                        conn_type: ConnType::Write,
                    }),
                )?;

                println!("Read data: {:?}", String::from_utf8_lossy(&buf));
            }
            ConnType::Close => {
                ring.add_event(Event::Close(CloseEvent { socket }), None)?;
                println!("Closing socket: {:?}", socket);
            }
        }

        ring.set_cqe_seen(cqe)?;
        ring.submit()?;
    }
}
