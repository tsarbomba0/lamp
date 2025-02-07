/// IoSource trait.
mod iosource;
pub(crate) use iosource::IoSource;

/// TcpStream struct.
pub(crate) use tcp_stream::TcpStream;
mod tcp_stream;

/// Trait for asychronous reads.
pub mod async_read;
pub use async_read::AsyncRead;

/// Trait for asynchronous writes.
pub mod async_write;
pub use async_write::AsyncWrite;
