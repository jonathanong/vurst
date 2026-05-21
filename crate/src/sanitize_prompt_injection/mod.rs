//! Prompt injection sanitization for external content.
//!
//! Removes injection patterns, role prefixes, HTML tags, and normalizes content
//! to prevent malicious content from hijacking LLM behavior.

pub mod sanitize;

pub use sanitize::sanitize_prompt_injection_sync;
