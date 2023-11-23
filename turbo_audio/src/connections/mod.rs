use self::{tcp::TcpConnection, usb::UsbConnection};

pub mod tcp;
pub mod usb;

pub enum Connection {
    Tcp(TcpConnection),
    Usb(UsbConnection),
}
