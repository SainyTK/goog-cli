pub mod artifacts;
#[allow(
    dead_code,
    reason = "managed IDs are consumed by the upcoming deck compiler slice"
)]
pub mod identity;
pub mod inspect;
#[allow(
    dead_code,
    reason = "Deck Source ingestion is consumed by the upcoming deck check compiler slice"
)]
pub mod source;

#[cfg(test)]
mod artifacts_tests;

#[cfg(test)]
mod identity_tests;

#[cfg(test)]
mod inspect_tests;

#[cfg(test)]
mod source_tests;
