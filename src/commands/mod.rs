pub mod auth;
pub mod calendar;
pub mod docs;
pub mod drive;
pub mod mail;
pub mod sheets;
pub mod slides;

#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod calendar_tests;
#[cfg(test)]
mod docs_tests;
#[cfg(test)]
mod drive_tests;
#[cfg(test)]
mod mail_tests;
#[cfg(test)]
mod sheets_tests;
#[cfg(test)]
mod slides_tests;
