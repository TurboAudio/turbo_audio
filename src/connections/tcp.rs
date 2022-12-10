use ring_channel::*;
use std::{
    io::Write,
    net::TcpStream,
    num::NonZeroUsize,
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct TcpConnection {
    pub data_queue: ring_channel::RingSender<Vec<u8>>,
    connection_thread: JoinHandle<()>,
}

impl TcpConnection {
    pub fn new(ip: std::net::SocketAddr) -> TcpConnection {
        let (tx, handle) = TcpConnection::start_connection_thread(ip);
        TcpConnection {
            data_queue: tx,
            connection_thread: handle,
        }
    }

    pub fn join(self) {
        self.connection_thread
            .join()
            .expect("Error when trying to join connection thread");
    }

    fn start_connection_thread(
        ip: std::net::SocketAddr,
    ) -> (ring_channel::RingSender<Vec<u8>>, JoinHandle<()>) {
        const SEND_BUFFER_SIZE: usize = 100;
        let (tx, mut rx) = ring_channel::<Vec<u8>>(NonZeroUsize::new(SEND_BUFFER_SIZE).unwrap());
        let connection_thread = thread::spawn(move || {
            const MAX_RECONNECTION_ATTEMPTS: i32 = 5;
            for reconnect_attempt in 0..MAX_RECONNECTION_ATTEMPTS {
                let connection = TcpConnection::attempt_connection(ip);
                if connection.is_none() {
                    if reconnect_attempt == 0 {
                        return;
                    } else {
                        continue;
                    }
                }

                let mut connection = connection.unwrap();
                if connection
                    .set_write_timeout(Some(Duration::from_millis(100)))
                    .is_err()
                {
                    return;
                }

                let mut connection_failed = false;
                while let Ok(data) = rx.recv() {
                    if connection.write_all(&data).is_err() {
                        connection_failed = true;
                        break;
                    }
                }

                if !connection_failed {
                    break;
                }
            }
        });

        (tx, connection_thread)
    }

    fn attempt_connection(ip: std::net::SocketAddr) -> Option<TcpStream> {
        const MAX_CONNECTION_ATTEMPTS: i32 = 5;
        let mut attempt_count = 0;
        loop {
            if attempt_count == MAX_CONNECTION_ATTEMPTS {
                break None;
            }

            let stream = TcpStream::connect_timeout(&ip, Duration::from_secs(5));
            if stream.is_err() {
                attempt_count += 1;
                continue;
            }

            break Some(stream.unwrap());
        }
    }
}
