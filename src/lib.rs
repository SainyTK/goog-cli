pub mod auth;
pub mod cli;
pub mod commands;
pub mod docs;
pub mod drive;
pub mod mail;
pub mod sheets;

#[cfg(test)]
mod cli_tests;
#[cfg(test)]
mod docs_tests;
#[cfg(test)]
mod drive_tests;
#[cfg(test)]
mod mail_tests;
#[cfg(test)]
mod sandcastle_tests;
#[cfg(test)]
mod sheets_tests;
#[cfg(test)]
mod test_support;
