//! Configuration module for Ferrite
//!
//! This module handles user preferences and application settings,
//! including serialization/deserialization to/from JSON and
//! persistent storage to platform-specific directories.

mod persistence;
mod settings;

pub use persistence::*;
pub use settings::*;
