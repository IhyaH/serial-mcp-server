//! MCP tool implementations
//!
//! Serial communication MCP tools using rust-sdk standard patterns

pub mod serial_handler;
pub mod types;

#[cfg(test)]
mod tests;

// Export the main handler and types
pub use serial_handler::*;
pub use types::*;