pub mod artifacts;
pub mod inspect;
#[allow(
    dead_code,
    reason = "Deck Source ingestion is consumed by the upcoming deck check compiler slice"
)]
pub mod source;

#[cfg(test)]
mod artifacts_tests;

#[cfg(test)]
mod inspect_tests;

#[cfg(test)]
mod source_tests;
