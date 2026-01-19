//! Agentroot MCP Server
//!
//! Model Context Protocol server for integration with AI assistants.

mod protocol;
mod server;
mod tools;
mod resources;

pub use server::start_server;
