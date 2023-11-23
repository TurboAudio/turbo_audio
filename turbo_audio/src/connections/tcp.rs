use ring_channel::*;
use std::{
    io::Write,
    net::TcpStream,
    num::NonZeroUsize,
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct TcpConnection {
    data_queue: Option<ring_channel::RingSender<Vec<u8>>>,
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
            data_queue: Some(tx),
            connection_thread: handle.into(),
        }
    }

    pub fn send_data(&mut self, packet: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.data_queue.as_mut().unwrap().send(packet).map(|_| ())
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
            // This loop essures we keep reconnecting if possible
            loop {
                let mut connection =
                    TcpConnection::attempt_connection(ip, None, None).map_err(|attempt_error| {
                        match disconnect_error {
                            Some(disconnect_error) => {
                                // This error comes from the last disconnect
                                TcpConnectionError::UnableToReconnect(
                                    attempt_error,
                                    disconnect_error,
                                )
                            }
                            None => TcpConnectionError::ConnectionFailed(attempt_error),
                        }
                    })?;

                // This loop sends the packets in data_queue through the TCP socket
                loop {
                    match rx.recv() {
                        Ok(data) => {
                            if let Err(e) = connection.write_all(&data) {
                                disconnect_error = Some(e);
                                // We break from this loop to allow reconnection to happen
                                log::info!("Lost connection with {ip}. Will attempt to reconnect.");
                                break;
                            }
                        }
                        // If an error occurs, the data_queue has no more sender
                        // and meaning the thread can exit correctly
                        Err(_) => {
                            log::info!("Closing connection with {ip}.");
                            return Ok(());
                        }
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
        for i in 0..max_connection_attempts {
            let stream = TcpStream::connect_timeout(&ip, connection_timeout);
            log::info!("[{i}/{max_connection_attempts}] Attempting to connect to {ip}");
            match stream {
                Ok(stream) => {
                    stream
                        .set_write_timeout(Some(Duration::from_millis(100)))
                        .map_err(ConnectionAttemptError::ConfigurationFailed)?;
                    log::info!("Connected to {ip}");
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
        log::info!("Closing tcp connection");
        drop(self.data_queue.take().unwrap());
        if let Err(e) = self.connection_thread.take().unwrap().join() {
            log::error!("Error in connection thread {:?}", e);
        }
        log::info!("Tcp connection thread joined.");
    }
}
