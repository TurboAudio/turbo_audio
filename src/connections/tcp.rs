use ring_channel::*;
use std::{
    io::Write,
    net::TcpStream,
    num::NonZeroUsize,
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct TcpConnection {
    data_queue: ring_channel::RingSender<Vec<u8>>,
    connection_thread: Option<JoinHandle<Result<(), TcpConnectionError>>>,
}

enum TcpConnectionError {
    ConnectionFailed(ConnectionAttemptError),
    UnableToReconnect(ConnectionAttemptError, std::io::Error),
}

enum ConnectionAttemptError {
    Unreachable(std::net::SocketAddr),
    ConfigurationFailed(std::io::Error),
}

impl TcpConnection {
    pub fn new(ip: std::net::SocketAddr) -> Self {
        let (tx, handle) = TcpConnection::start_connection_thread(ip);
        Self {
            data_queue: tx,
            connection_thread: handle.into(),
        }
    }

    pub fn send_data(&mut self, packet: Vec<u8>) -> Result<Option<Vec<u8>>, SendError<Vec<u8>>> {
        self.data_queue.send(packet)
    }

    fn start_connection_thread(
        ip: std::net::SocketAddr,
    ) -> (
        ring_channel::RingSender<Vec<u8>>,
        JoinHandle<Result<(), TcpConnectionError>>,
    ) {
        let buffer_size: NonZeroUsize = NonZeroUsize::new(64).unwrap();
        let (tx, mut rx) = ring_channel::<Vec<u8>>(buffer_size);
        let connection_thread = thread::spawn(move || -> Result<(), TcpConnectionError> {
            let mut disconnect_error = None;
            loop {
                let mut connection = TcpConnection::attempt_connection(ip, None, None).map_err(
                    |connection_attempt_error| match disconnect_error {
                        Some(error) => {
                            TcpConnectionError::UnableToReconnect(connection_attempt_error, error)
                        }
                        None => TcpConnectionError::ConnectionFailed(connection_attempt_error),
                    },
                )?;
                loop {
                    match rx.recv() {
                        Ok(data) => {
                            if let Err(e) = connection.write_all(&data) {
                                disconnect_error = Some(e);
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
    ) -> Result<TcpStream, ConnectionAttemptError> {
        let max_connection_attempts = max_connection_attempts.unwrap_or(20);
        let connection_timeout = connection_timeout.unwrap_or(Duration::from_secs(3));
        for _ in 0..max_connection_attempts {
            let stream = TcpStream::connect_timeout(&ip, connection_timeout);
            match stream {
                Ok(stream) => {
                    stream
                        .set_write_timeout(Some(Duration::from_millis(100)))
                        .map_err(ConnectionAttemptError::ConfigurationFailed)?;
                    return Ok(stream);
                }
                Err(_) => continue,
            }
        }
        Err(ConnectionAttemptError::Unreachable(ip))
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        if let Some(connection) = std::mem::replace(&mut self.connection_thread, None) {
            if let Err(e) = connection.join() {
                eprintln!("Error in connection thread {:?}", e);
            }
            println!("Connection thread joined.");
        }
    }
}
