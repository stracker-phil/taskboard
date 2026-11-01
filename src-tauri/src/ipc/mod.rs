//! Single-instance handoff: a second `taskboard` process sends its request over a Unix socket
//! (mode 0600, same user only) to the running app and exits.
//!
//! One JSON line per connection. A bare `IpcMessage` (the one-way form) is acknowledged with
//! the line `ok` before it is handled; an `IpcRequest` is answered with an `IpcReply` line
//! once it is handled.

mod client;
mod protocol;
mod server;
#[cfg(test)]
mod tests;

pub use client::{Forward, request, try_forward};
pub use protocol::{HandlerResult, IpcError, IpcMessage, IpcReply, IpcRequest, MAX_MESSAGE_BYTES};
pub use server::{bind, serve};
