pub mod error;

pub use error::DriveError;

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
pub const DRIVE_SCOPES: &[&str] = &[DRIVE_SCOPE];
