# rmk-yip

`rmk-yip` connects YIP radio sessions to RMK's split keyboard services.
It owns packet serialization, connection tracking, session tasks, and sleep/wake
handling. YIP owns the radio protocol; board settings stay in RMK's TOML configuration.
The current hardware helper supports nRF52840.

## Board integration

For nRF52840 boards with external HF/LF crystals, use the configuration attributes:

```rust
#![no_main]
#![no_std]

#[rmk_yip::central]
mod keyboard_central {}
```

The peripheral binaries use `#[rmk_yip::peripheral(0)]` and
`#[rmk_yip::peripheral(1)]` on their modules. Do not also add RMK's central or
peripheral attribute: these wrappers invoke it after supplying chip initialization.
They read the same `KEYBOARD_TOML_PATH` as RMK. Peripheral count, rows, columns,
and offsets come from `[[split.peripheral]]`, in array order. DCDC settings come
from `[chip.nrf52840]`, including RMK's chip defaults. Set `dcdc_reg0`,
`dcdc_reg1`, and `dcdc_reg0_voltage` explicitly to preserve board-specific settings.
With `dfu_split`, each peripheral's `update_policy` is also used.
BLE and non-custom split configurations are rejected. Keep matrix pins, keymaps,
USB settings and subscriber capacities in the board's TOML file.

The wrappers live in the companion `macros` crate in this repository and use
`rmk-config` to parse the configuration. This checkout still uses local paths to
RMK and YIP; adjust those dependencies when moving it to another directory.

### Manual initialization

Depend on `rmk-yip` with `nrf52840` and either `defmt` or `log`. The board does
not need a direct YIP dependency when using the provided resource helper.
Keep the Embassy executor platform and time-driver features in the board manifest.

Set `split.connection = "custom"` in `keyboard.toml`. After initializing
`p` with external HF/LF crystals, create the future inside `Override(chip_init)`:

```rust
let device = rmk_yip::nrf52840_device!(central, p);
let split_task = rmk_yip::run_central(
    spawner,
    device,
    [
        rmk_yip::PeripheralConfig::new(1, 4, 1, 0),
        rmk_yip::PeripheralConfig::new(1, 4, 2, 0),
    ],
);
```

`PeripheralConfig::new` takes rows, columns, row offset, and column offset.
Array order assigns peripheral IDs, starting at zero. Keep these placements
consistent with `[[split.peripheral]]`. The session count comes from the array;
the Embassy task pool uses RMK's `SPLIT_PERIPHERALS_NUM` configuration constant.
The runner checks that the array has 1–7 entries and fits that pool.

A peripheral supplies its ID:

```rust
let device = rmk_yip::nrf52840_device!(peripheral, p);
let split_task = rmk_yip::run_peripheral(device, 0);
```

RMK polls `split_task` alongside the generated matrix, processors, and USB tasks.
Call one runner per firmware; it initializes and starts the radio when first polled.

## Hardware resources

`nrf52840_device!` binds EGU2_SWI2 and consumes RADIO, TIMER2, RTC2, EGU2,
PPI_CH0–7, and PPI_GROUP0. The central also uses PPI_GROUP1. Other fields of
`p` remain available to the board. DCDC settings and external crystal selection
stay in the board's initialization code. For another resource assignment, a board
can construct a compatible `yip::DeviceContext` and pass it to the same runners.

## RMK configuration

This crate uses the local RMK public split driver API and `custom` connection hook.
It does not add YIP-specific code to RMK. Reserve one additional subscriber for
`keyboard`, `pointing`, and `sleep_state` in the board's `[event.*]` settings for
the sleep manager. Other subscribers required by the board still apply.

`dfu_split` forwards RMK's split DFU support. Enable the appropriate RMK DFU
backend on the board as well. Peripheral update policies default to `MatchHash`;
use `PeripheralConfig::with_update_policy` to choose another policy.

Sleep tests use RMK's mock-clock harness and require `cargo-nextest`:

```text
cargo nextest run --lib
```
