pub mod auth;
pub mod cli;
pub mod commands;
pub mod docs;
pub mod drive;
pub mod mail;

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
