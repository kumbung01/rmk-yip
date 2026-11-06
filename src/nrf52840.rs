/// Create an nRF52840 YIP device using the example resource assignment.
///
/// Consumes RADIO, TIMER2, RTC2, EGU2, PPI_CH0..7 and PPI_GROUP0.
/// The central also consumes PPI_GROUP1. Other board peripherals stay available.
/// The board must configure its external HF/LF crystals before calling this macro.
#[macro_export]
macro_rules! nrf52840_device {
    (central, $p:ident) => {
        $crate::nrf52840_device!(@device central, $p)
            .with_central_completion_group($p.PPI_GROUP1)
    };
    (peripheral, $p:ident) => {
        $crate::nrf52840_device!(@device peripheral, $p)
    };
    (@device $role:ident, $p:ident) => {{
        $crate::__private::embassy_nrf::bind_interrupts!(struct YipIrqs {
            EGU2_SWI2 => $crate::__private::yip::$role::YipInterruptHandler;
        });
        $crate::__private::yip::DeviceContext::new(
            $p.RADIO, $p.TIMER2, $p.RTC2,
            $crate::__private::yip::ppi_channels![
                $p.PPI_CH0, $p.PPI_CH1, $p.PPI_CH2, $p.PPI_CH3,
                $p.PPI_CH4, $p.PPI_CH5, $p.PPI_CH6, $p.PPI_CH7,
            ],
            $p.PPI_GROUP0, $p.EGU2, YipIrqs, 0,
        )
    }};
}
