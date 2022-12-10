use self::{tcp::TcpConnection, terminal::UsbConnection};

pub mod tcp;
pub mod terminal;

pub enum Connection {
    Tcp(TcpConnection),
    Usb(UsbConnection),
}
