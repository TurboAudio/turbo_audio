use std::{
    io::Write,
    net::TcpStream,
    sync::mpsc::{channel, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct TcpConnection {
    pub data_queue: Option<Sender<Vec<u8>>>,
    ip: std::net::SocketAddr,
    connection_thread: Option<JoinHandle<()>>,
}

impl TcpConnection {
    pub fn new(ip: std::net::SocketAddr) -> Option<TcpConnection> {
        let mut connection = TcpConnection {
            connection_thread: None,
            data_queue: None,
            ip,
        };

        const CONNECTION_ATTEMPS: i32 = 5;
        for _ in 0..CONNECTION_ATTEMPS {
            if connection.connect() {
                return Some(connection);
            }
        }

        None
    }

    pub fn join(self) {
        if self.connection_thread.is_some() {
            self.connection_thread
                .unwrap()
                .join()
                .expect("Error when trying to join connection thread");
        }
    }

    fn connect(&mut self) -> bool {
        let (tx, rx) = channel::<Vec<u8>>();
        let connection = TcpStream::connect_timeout(&self.ip, Duration::from_secs(5));
        if connection.is_err() {
            return false;
        }

        let connection_thread = thread::spawn(move || {
            let mut connection = connection.unwrap();
            if connection
                .set_write_timeout(Some(Duration::from_millis(100)))
                .is_err()
            {
                return;
            }

            while let Ok(data) = rx.recv() {
                if connection.write_all(&data).is_err() {
                    break;
                }
            }
        });

        self.data_queue = Some(tx);
        self.connection_thread = Some(connection_thread);
        true
    }
}
