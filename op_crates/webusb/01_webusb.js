// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  
  class UsbDevice {
    constructor(device) {
      this.device = device;
      this.opened = false;
    }

    async claimInterface(interfaceNumber) {
      if(!this.opened) throw new Error("The device must be opened first.");

      return core.jsonOpSync("op_webusb_claim_interface", interfaceNumber);
    }
  }
  function getDevices() {
      return core.jsonOpSync("op_webusb_get_devices", {});
  }

  window.usb = {
    getDevices,
  };
  window.__bootstrap = window.__bootstrap || {};
  window.__bootstrap.usb = {
    getDevices,
  };
})(this);
