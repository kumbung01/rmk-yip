#![no_std]

#[collapse_debuginfo(yes)]
macro_rules! info {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::info!($s $(, $x)*);
            #[cfg(feature = "defmt")]
            ::defmt::info!($s $(, $x)*);
            #[cfg(not(any(feature = "log", feature="defmt")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[cfg(feature = "_radio")]
#[collapse_debuginfo(yes)]
macro_rules! warn {
    ($s:literal $(, $x:expr)* $(,)?) => {
        {
            #[cfg(feature = "log")]
            ::log::warn!($s $(, $x)*);
            #[cfg(feature = "defmt")]
            ::defmt::warn!($s $(, $x)*);
            #[cfg(not(any(feature = "log", feature="defmt")))]
            let _ = ($( & $x ),*);
        }
    };
}

#[cfg(feature = "_radio")]
mod runtime;
pub mod sleep;
#[cfg(feature = "_radio")]
mod transport;
#[cfg(feature = "_radio")]
pub use runtime::{PeripheralConfig, run_central, run_peripheral};

#[cfg(feature = "nrf52840")]
mod nrf52840;

// Dependencies used by exported board resource macros.
#[doc(hidden)]
#[cfg(feature = "nrf52840")]
pub mod __private {
    pub use embassy_nrf;
    pub use yip;
}
