// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
use deno_webusb::op_webusb_claim_interface;
use deno_webusb::op_webusb_clear_halt;
use deno_webusb::op_webusb_close_device;
use deno_webusb::op_webusb_control_transfer_in;
use deno_webusb::op_webusb_control_transfer_out;
use deno_webusb::op_webusb_get_devices;
use deno_webusb::op_webusb_open_device;
use deno_webusb::op_webusb_release_interface;
use deno_webusb::op_webusb_reset;
use deno_webusb::op_webusb_select_alternate_interface;
use deno_webusb::op_webusb_select_configuration;
use deno_webusb::op_webusb_transfer_in;
use deno_webusb::op_webusb_transfer_out;
use deno_webusb::op_webusb_iso_transfer_in;

pub fn init(rt: &mut deno_core::JsRuntime) {
  super::reg_json_async(rt, "op_webusb_get_devices", op_webusb_get_devices);
  super::reg_json_async(rt, "op_webusb_open_device", op_webusb_open_device);
  super::reg_json_async(
    rt,
    "op_webusb_claim_interface",
    op_webusb_claim_interface,
  );
  super::reg_json_async(
    rt,
    "op_webusb_release_interface",
    op_webusb_release_interface,
  );
  super::reg_json_async(
    rt,
    "op_webusb_select_configuration",
    op_webusb_select_configuration,
  );
  super::reg_json_async(
    rt,
    "op_webusb_select_alternate_interface",
    op_webusb_select_alternate_interface,
  );
  super::reg_json_async(rt, "op_webusb_reset", op_webusb_reset);
  super::reg_json_async(rt, "op_webusb_close_device", op_webusb_close_device);
  super::reg_json_async(rt, "op_webusb_clear_halt", op_webusb_clear_halt);
  super::reg_json_async(
    rt,
    "op_webusb_control_transfer_in",
    op_webusb_control_transfer_in,
  );
  super::reg_json_async(
    rt,
    "op_webusb_control_transfer_out",
    op_webusb_control_transfer_out,
  );
  super::reg_json_async(rt, "op_webusb_transfer_in", op_webusb_transfer_in);
  super::reg_json_async(rt, "op_webusb_transfer_out", op_webusb_transfer_out);
  super::reg_json_async(rt, "op_webusb_iso_transfer_in", op_webusb_iso_transfer_in);
}
