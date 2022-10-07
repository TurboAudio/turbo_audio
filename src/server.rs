use std::sync::mpsc::{Receiver, Sender, self};
use std::thread::{spawn, JoinHandle};

// TODO: Find a way to merge the 2 threads into one.
pub struct Connection {
    incomming_thread: JoinHandle<()>,
    outgoing_thread: JoinHandle<()>,
    tx: Sender<i32>,
}

impl Connection {
    // Takes channel on which to write incomming messages
    pub fn new(incomming_channel: Sender<i32>) -> Connection {
        let (tx, rx) = mpsc::channel();
        Connection {
            incomming_thread: spawn(move || {
                for incomming_message in 0..10 {
                    incomming_channel.send(incomming_message).unwrap();
                }
            }),
            outgoing_thread: spawn(move || {
                for message in rx.iter() {
                    if message == 0 {
                        return;
                    }
                    println!("Connection got {}", message);
                }
            }),
            tx,
        }
    }

    pub fn join(self) {
        self.incomming_thread.join().unwrap();
        self.outgoing_thread.join().unwrap();
    }

    pub fn send(&mut self, value: i32) {
        self.tx.send(value).unwrap();
    }
}

pub struct WebSocketServer {
    tx: Sender<i32>,
    rx: Receiver<i32>,
    connections: Vec<Connection>,
}

impl WebSocketServer {
    pub fn new() -> WebSocketServer {
        let (tx, rx) = mpsc::channel();
        WebSocketServer {
            connections: vec![],
            tx,
            rx,
        }
    }

    pub fn send_message(&mut self, value: i32) {
        for connection in self.connections.iter_mut() {
            connection.send(value);
        }
    }

    pub fn get_messages(&mut self) {
        for message in self.rx.try_iter() {
            println!("Server got {}", message);
        }
    }

    pub fn new_connection(&mut self) {
        self.connections.push(Connection::new(self.tx.clone()));
    }

    pub fn close_connections(&mut self) {
        for mut connection in self.connections.drain(..) {
            connection.send(0);
            connection.join();
        }
    }
}
