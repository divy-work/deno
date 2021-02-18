// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

// #![deny(warnings)]

use deno_core::error::AnyError;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::JsRuntime;
use deno_core::Resource;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use serde::{Serialize, Deserialize};
use rusb::{DeviceHandle, UsbContext};
use std::borrow::Cow;

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

#[derive(Serialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsbConfiguration {
  // Index of String Descriptor describing this configuration.
  configuration_name: Option<u8>,
  // The configuration number. Should corresspond to bConfigurationValue (https://www.beyondlogic.org/usbnutshell/usb5.shtml#ConfigurationDescriptors)
  configuration_value: u8,
  // TODO: implement USBInterfaces
}

#[derive(Serialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
  configuration: Option<UsbConfiguration>,
  // TODO: Implement configurations using https://docs.rs/rusb/0.7.0/rusb/struct.Device.html#method.config_descriptor
  device_class: u8,
  device_subclass: u8,
  device_protocol: u8,
  device_version_major: u8,
  device_version_minor: u8,
  device_version_subminor: u8,
  // Need to open USB to get manufacturer_name https://docs.rs/rusb/0.7.0/rusb/struct.DeviceHandle.html#method.read_manufacturer_string_ascii
  // manufacturer_name: String,
  product_id: u16,
  // Need to open USB to get product_name https://docs.rs/rusb/0.7.0/rusb/struct.DeviceDescriptor.html#method.product_string_index
  // product_name: String,

  // Need to open USB to get serial_number https://docs.rs/rusb/0.7.0/rusb/struct.DeviceDescriptor.html#method.serial_number_string_index
  // serial_number: String,
  usb_version_major: u8,
  usb_version_minor: u8,
  usb_version_subminor: u8,
  vendor_id: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Args {
  rid: u32,
}

impl Resource for UsbDevice {
  fn name(&self) -> Cow<str> {
    "usbDevice".into()
  }
}


pub fn op_webusb_get_devices(
  state: &mut OpState,
  _args: Value,
  _zero_copy: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError> {
  let devices = rusb::devices().unwrap();
  
  #[derive(Serialize)]
  struct Device {
    usbdevice: UsbDevice,
    rid: u32,
  }

  let mut usbdevices: Vec<Device> = vec![];
  for device in devices.iter() {
    let config_descriptor = device.active_config_descriptor();
    let device_descriptor = device.device_descriptor().unwrap();
    let device_version = device_descriptor.device_version();
    let usb_version = device_descriptor.usb_version();

    let configuration = match config_descriptor {
      Ok(config_descriptor) => Some(UsbConfiguration {
        configuration_name:  config_descriptor.description_string_index(),
        configuration_value: config_descriptor.number(),
      }),
      Err(_) => None,
    };
    
    let usbdevice = UsbDevice {
      configuration,
      device_class: device_descriptor.class_code(),
      device_subclass: device_descriptor.sub_class_code(),
      device_protocol: device_descriptor.protocol_code(),
      device_version_major: device_version.major(),
      device_version_minor: device_version.minor(),
      device_version_subminor: device_version.sub_minor(),
      product_id: device_descriptor.product_id(),
      usb_version_major: usb_version.major(),
      usb_version_minor: usb_version.minor(),
      usb_version_subminor: usb_version.sub_minor(),
      vendor_id: device_descriptor.vendor_id(),
    };
    let rid = state.resource_table.add(usbdevice);
    usbdevices.push(Device { usbdevice, rid });
  }

  Ok(json!(usbdevices))
}
