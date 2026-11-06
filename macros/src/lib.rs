use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use rmk_config::resolved::hardware::BoardConfig;
use rmk_config::{DcdcReg0Voltage, KeyboardTomlConfig, SplitConnection};
use syn::{ItemMod, parse_macro_input};

/// Configure an nRF52840 YIP central from RMK's KEYBOARD_TOML_PATH.
#[proc_macro_attribute]
pub fn central(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(Span::call_site(), "central takes no arguments")
            .to_compile_error()
            .into();
    }
    expand(None, parse_macro_input!(item as ItemMod))
}

/// Configure a YIP peripheral; the argument is its zero-based TOML array index.
#[proc_macro_attribute]
pub fn peripheral(attr: TokenStream, item: TokenStream) -> TokenStream {
    let id = parse_macro_input!(attr as syn::LitInt);
    match id.base10_parse::<usize>() {
        Ok(id) => expand(Some(id), parse_macro_input!(item as ItemMod)),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(id: Option<usize>, mut module: ItemMod) -> TokenStream {
    let result = (|| -> Result<_, String> {
        let path = std::env::var("KEYBOARD_TOML_PATH").map_err(|_| "KEYBOARD_TOML_PATH must point to keyboard.toml")?;
        let config = KeyboardTomlConfig::new_from_toml_path(&path);
        let hardware = config.hardware()?;
        if hardware.chip.chip != "nrf52840" {
            return Err("rmk-yip supports nRF52840 only".into());
        }
        if hardware.communication.ble_enabled() {
            return Err("YIP split and BLE cannot own RADIO at the same time".into());
        }
        let BoardConfig::Split(split) = hardware.board else {
            return Err("YIP requires [split] configuration".into());
        };
        if split.connection != SplitConnection::Custom {
            return Err("YIP requires split.connection = \"custom\"".into());
        }
        if !(1..=7).contains(&split.peripheral.len()) {
            return Err("YIP requires 1..=7 peripherals".into());
        }
        let (attribute, task) = if let Some(id) = id {
            if id >= split.peripheral.len() {
                return Err("YIP peripheral index is outside [[split.peripheral]]".into());
            }
            let radio_id = id as u8;
            (
                quote!(#[::rmk::macros::rmk_peripheral(id = #id)]),
                quote! {
                    let device = ::rmk_yip::nrf52840_device!(peripheral, p);
                    let split_task = ::rmk_yip::run_peripheral(device, #radio_id);
                },
            )
        } else {
            let peripherals = split
                .peripheral
                .iter()
                .map(|p| {
                    let values = [p.rows, p.cols, p.row_offset, p.col_offset].map(u8::try_from);
                    let [Ok(rows), Ok(cols), Ok(row_offset), Ok(col_offset)] = values else {
                        return Err("YIP matrix dimensions and offsets must fit in u8".to_string());
                    };
                    let config = quote!(::rmk_yip::PeripheralConfig::new(#rows, #cols, #row_offset, #col_offset));
                    #[cfg(feature = "dfu_split")]
                    let config = {
                        let policy = match p.update_policy.as_ref().unwrap_or(&rmk_config::UpdatePolicy::MatchHash) {
                            rmk_config::UpdatePolicy::MatchHash => quote!(MatchHash),
                            rmk_config::UpdatePolicy::Force => quote!(Force),
                        };
                        quote!(#config.with_update_policy(::rmk::split::central::UpdatePolicy::#policy))
                    };
                    Ok(config)
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                quote!(#[::rmk::macros::rmk_central]),
                quote! {
                    let device = ::rmk_yip::nrf52840_device!(central, p);
                    let split_task = ::rmk_yip::run_central(spawner, device, [#(#peripherals),*]);
                },
            )
        };
        let chip = hardware.chip_config;
        let reg0 = chip.dcdc_reg0.unwrap_or(true);
        let reg1 = chip.dcdc_reg1.unwrap_or(true);
        let voltage = match chip.dcdc_reg0_voltage {
            Some(DcdcReg0Voltage::V1_8) => quote!(_1V8),
            _ => quote!(_3V3),
        };
        let (_, items) = module.content.as_mut().ok_or("YIP requires an inline module")?;
        items.insert(
            0,
            syn::parse_quote! {
                #[Override(chip_init)]
                fn chip_init() {
                use ::embassy_nrf::interrupt::InterruptExt as _;
                    let mut config = ::embassy_nrf::config::Config::default();
                    config.hfclk_source = ::embassy_nrf::config::HfclkSource::ExternalXtal;
                    config.lfclk_source = ::embassy_nrf::config::LfclkSource::ExternalXtal;
                    config.dcdc.reg0_voltage = Some(::embassy_nrf::config::Reg0Voltage::#voltage);
                    config.dcdc.reg0 = #reg0;
                    config.dcdc.reg1 = #reg1;
                    let p = ::embassy_nrf::init(config);
                    #task
                }
            },
        );
        Ok(quote! {
            const _: &str = include_str!(#path);
            #[cfg(feature = "_ble")]
            compile_error!("YIP split and BLE cannot own RADIO at the same time");
            #attribute
            #module
        })
    })();
    result
        .unwrap_or_else(|e| syn::Error::new(Span::call_site(), e).to_compile_error())
        .into()
}
