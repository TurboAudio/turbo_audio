use anyhow::anyhow;
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
    connection_thread: Option<JoinHandle<anyhow::Result<()>>>,
}

impl TcpConnection {
    pub fn new(ip: std::net::SocketAddr) -> TcpConnection {
        let (tx, handle) = TcpConnection::start_connection_thread(ip);
        TcpConnection {
            data_queue: tx,
            connection_thread: Some(handle),
        }
    }

    fn start_connection_thread(
        ip: std::net::SocketAddr,
    ) -> (
        ring_channel::RingSender<Vec<u8>>,
        JoinHandle<anyhow::Result<()>>,
    ) {
        let buffer_size: NonZeroUsize = NonZeroUsize::new(64).unwrap();
        let (tx, mut rx) = ring_channel::<Vec<u8>>(buffer_size);
        let connection_thread = thread::spawn(move || -> Result<(), anyhow::Error> {
            loop {
                let mut connection = TcpConnection::attempt_connection(ip, None, None)
                    .ok_or_else(|| anyhow!("Connection failed"))?;
                connection.set_write_timeout(Some(Duration::from_millis(100)))?;

                loop {
                    match rx.recv() {
                        Ok(data) => {
                            if let Err(e) = connection.write_all(&data) {
                                eprintln!("Failed to send packet to connection. Error: {:?}", e);
                                break;
                            }
                        }
                        Err(_) => return Ok(()),
                    }
                }
            }
        });

        (tx, connection_thread)
    }

    fn attempt_connection(
        ip: std::net::SocketAddr,
        max_connection_attempts: Option<i32>,
        connection_timeout: Option<Duration>,
    ) -> Option<TcpStream> {
        let max_connection_attempts = max_connection_attempts.unwrap_or(20);
        let connection_timeout = connection_timeout.unwrap_or(Duration::from_secs(3));
        for _ in 0..max_connection_attempts {
            let stream = TcpStream::connect_timeout(&ip, connection_timeout);
            if stream.is_err() {
                continue;
            }

            return Some(stream.unwrap());
        }
        None
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        if let Some(connection) = std::mem::replace(&mut self.connection_thread, None) {
            if let Err(e) = connection.join() {
                eprintln!("Error in connection thread {:?}", e);
            }
            println!("Conn thread joined.");
        }
    }
}
