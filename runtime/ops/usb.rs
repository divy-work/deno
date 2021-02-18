// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
use deno_webusb::op_webusb_claim_interface;
use deno_webusb::op_webusb_get_devices;
use deno_webusb::op_webusb_open_device;

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_json_sync(rt, "op_webusb_get_devices", op_webusb_get_devices);
  super::reg_json_sync(rt, "op_webusb_open_device", op_webusb_open_device);
  super::reg_json_async(
    rt,
    "op_webusb_claim_interface",
    op_webusb_claim_interface,
  );
}
