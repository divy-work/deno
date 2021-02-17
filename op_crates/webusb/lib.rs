// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

// #![deny(warnings)]

use deno_core::error::AnyError;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;

pub use rusb; // Re-export rusb

/// Execute this crates' JS source files.
pub fn init(isolate: &mut JsRuntime) {
  let files = vec![(
    "deno:op_crates/webusb/01_webusb.js",
    include_str!("01_webusb.js"),
  )];
  for (url, source_code) in files {
    isolate.execute(url, source_code).unwrap();
  }
}

pub struct UsbConfiguration {
  // Index of String Descriptor describing this configuration.
  configuration_name: u8,
  // The configuration number. Should corresspond to bConfigurationValue (https://www.beyondlogic.org/usbnutshell/usb5.shtml#ConfigurationDescriptors)
  configuration_value: u8,
}

pub struct UsbDevice {
  configuration: UsbConfiguration,
}

pub fn op_webusb_get_devices(
  _state: &mut OpState,
  _args: Value,
  _zero_copy: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError> {
  let devices = rusb::devices().unwrap();
  let mut usbdevices: Vec<UsbDevice> = vec![];

  for device in devices.iter() {
    let config_descriptor = device.active_config_descriptor().unwrap();
    let configuration = UsbConfiguration {
      configuration_name: config_descriptor.description_string_index().unwrap(),
      configuration_value: config_descriptor.number(),
    };

    usbdevices.push(UsbDevice { configuration });
  }

  Ok(json!({}))
}
