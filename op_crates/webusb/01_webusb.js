// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  
  class UsbDevice {
    #rid
    constructor(device, rid) {
      this.device = device;
      this.#rid = rid;
      this.opened = false;
    }

    async claimInterface(interfaceNumber) {
      if(!this.opened) throw new Error("The device must be opened first.");

      return core.jsonOpSync("op_webusb_claim_interface", interfaceNumber);
    }

    async open() {
      if(this.opened) throw new Error("The device is already opened.");
      return core.jsonOpSync("op_webusb_open_device", { rid: this.#rid })
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
