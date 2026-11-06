use embassy_executor::Spawner;
use rmk::split::PeripheralMatrixConfig;
use rmk::types::constants::SPLIT_PERIPHERALS_NUM;
use yip::{DeviceContext, YipSession};

/// Matrix placement and optional firmware update policy for one YIP peripheral.
pub struct PeripheralConfig {
    matrix: PeripheralMatrixConfig,
    #[cfg(feature = "dfu_split")]
    policy: rmk::split::central::UpdatePolicy,
}

impl PeripheralConfig {
    pub const fn new(rows: u8, cols: u8, row_offset: u8, col_offset: u8) -> Self {
        Self {
            matrix: PeripheralMatrixConfig {
                rows,
                cols,
                row_offset,
                col_offset,
            },
            #[cfg(feature = "dfu_split")]
            policy: rmk::split::central::UpdatePolicy::MatchHash,
        }
    }

    #[cfg(feature = "dfu_split")]
    pub fn with_update_policy(mut self, policy: rmk::split::central::UpdatePolicy) -> Self {
        self.policy = policy;
        self
    }
}

#[embassy_executor::task(pool_size = SPLIT_PERIPHERALS_NUM)]
async fn session_task(id: usize, session: YipSession, config: PeripheralConfig) {
    crate::transport::run_esb_peripheral_manager(
        id,
        session,
        config.matrix,
        #[cfg(feature = "dfu_split")]
        config.policy,
    )
    .await;
}

/// Initialize and run the central link and its sleep manager once per firmware.
/// Array order determines peripheral IDs. The RMK configuration must reserve
/// at least that many peripheral slots.
pub async fn run_central<const N: usize>(
    spawner: Spawner,
    device: DeviceContext<yip::central::YipInterruptHandler>,
    peripherals: [PeripheralConfig; N],
) {
    assert!((1..=7).contains(&N), "YIP requires 1..=7 peripherals");
    assert!(
        N <= SPLIT_PERIPHERALS_NUM,
        "RMK peripheral capacity is smaller than the YIP configuration"
    );
    let sessions = yip::central::init::<N>(device);
    for (id, (session, config)) in sessions.into_iter().zip(peripherals).enumerate() {
        spawner.spawn(session_task(id, session, config).expect("YIP session task pool exhausted"));
    }
    yip::central::start();
    crate::sleep::run_sleep_manager().await;
}

/// Initialize and run a peripheral link, including reconnect and sleep handling.
pub async fn run_peripheral(device: DeviceContext<yip::peripheral::YipInterruptHandler>, id: u8) {
    assert!(id < 7, "YIP peripheral ID must be below 7");
    let session = yip::peripheral::init(device, id);
    yip::peripheral::start();
    crate::transport::run_esb_peripheral(session).await;
}
