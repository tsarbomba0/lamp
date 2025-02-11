use crate::io::read_write::{ReadFut, WriteFut};
use crate::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::reactor::reactor::Direction;
use crate::runtime::Executor;

use mio::event::Source;
use mio::net;
use mio::Interest;
use mio::Token;

//use log::info;

use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! handle_async_read {
    ($io: expr, $buf: expr, $cx: expr, $token: expr) => {
        match (&$io).read($buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Executor::get_reactor().attach_waker($cx, $token, Direction::Read);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    };
}

macro_rules! handle_async_write {
    ($io: expr, $buf: expr, $cx: expr, $token: expr) => {
        match (&$io).write($buf) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Executor::get_reactor().attach_waker($cx, $token, Direction::Write);
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
            Ok(size) => Poll::Ready(Ok(size)),
        }
    };
}

macro_rules! handle_async_flush {
    ($io: expr, $cx: expr, $token: expr) => {
        match (&$io).flush() {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                Executor::get_reactor().attach_waker($cx, $token, Direction::Write);
                Poll::Pending
            }

            Err(e) => Poll::Ready(Err(e)),
            Ok(_) => Poll::Ready(Ok(())),
        }
    };
}

/// TCP Socket connected to a listener.
pub struct TcpStream {
    io: mio::net::TcpStream,
    pub(crate) token: Token,
}

/// Impl TcpStream
impl TcpStream {
    /// Create a new TcpStream.
    pub fn new(addr: &str) -> io::Result<TcpStream> {
        let reactor = Executor::get_reactor();

        let address = match addr.parse() {
            Ok(o) => o,
            Err(_e) => return Err(io::Error::new(io::ErrorKind::NotFound, "invalid address")),
        };

        let mut tcp = net::TcpStream::connect(address)?;

        // improve connecting.
        // mio specifies that you should do more checks
        // i shall do them once day.

        let n = reactor.register(&mut tcp, Interest::READABLE | Interest::WRITABLE)?;

        Ok(Self {
            io: tcp,
            token: Token(n),
        })
    }
}

impl AsyncRead for TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'a mut [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_read!(self.io, buf, cx, self.token)
    }
}

impl AsyncRead for &TcpStream {
    /// Read x amount of bytes from this socket.
    /// It's asynchronous woo!!
    fn poll_read<'a>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'a mut [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_read!(self.io, buf, cx, self.token)
    }
}

impl AsyncReadExt for TcpStream {
    /// Read x amount of bytes into `buf` from this socket asychronously.
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self> {
        ReadFut::new(self, buf, self.token)
    }
}

impl AsyncReadExt for &TcpStream {
    /// Read x amount of bytes into `buf` from this socket asychronously.
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFut<'a, Self> {
        ReadFut::new(self, buf, self.token)
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'w [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_write!(self.io, buf, cx, self.token)
    }

    fn poll_flush<'f>(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        handle_async_flush!(self.io, cx, self.token)
    }
}

impl AsyncWrite for &TcpStream {
    fn poll_write<'w>(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &'w [u8],
    ) -> Poll<io::Result<usize>> {
        handle_async_write!(self.io, buf, cx, self.token)
    }

    fn poll_flush<'f>(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        handle_async_flush!(self.io, cx, self.token)
    }
}

impl AsyncWriteExt for TcpStream {
    /// Writes x amount of bytes from `buf` to this socket asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type.
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> WriteFut<'a, Self> {
        WriteFut::new(self, buf, self.token)
    }
}

impl AsyncWriteExt for &TcpStream {
    /// Writes x amount of bytes from `buf` to this asychronously
    /// Returns a `Future` with `io::Result<usize>` as it's Output type
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> WriteFut<'a, Self> {
        WriteFut::new(self, buf, self.token)
    }
}

impl Source for TcpStream {
    fn register(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.register(reg, token, intr)
    }

    fn reregister(
        &mut self,
        reg: &mio::Registry,
        token: Token,
        intr: mio::Interest,
    ) -> io::Result<()> {
        self.io.reregister(reg, token, intr)
    }

    fn deregister(&mut self, registry: &mio::Registry) -> io::Result<()> {
        self.io.deregister(registry)
    }
}
