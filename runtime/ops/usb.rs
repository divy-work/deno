// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
use deno_webusb::op_webusb_get_devices;

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_json_sync(rt, "op_webusb_get_devices", op_webusb_get_devices);
}
