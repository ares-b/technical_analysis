#[cfg(feature = "precision")]
mod precision;
#[cfg(feature = "precision")]
pub use precision::IndicatorValue;

#[cfg(not(feature = "precision"))]
mod no_precision;
#[cfg(not(feature = "precision"))]
pub use no_precision::IndicatorValue;
