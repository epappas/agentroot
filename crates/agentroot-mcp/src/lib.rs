//! Agentroot MCP Server
//!
//! Model Context Protocol server for integration with AI assistants.

mod protocol;
mod resources;
mod server;
mod tools;

pub use server::start_server;
