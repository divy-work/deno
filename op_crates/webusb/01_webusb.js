// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;

  class UsbDevice {
    #rid;
    #deviceHandleRid;
    constructor(device, rid) {
      this.device = device;
      this.#rid = rid;
      this.opened = false;
    }

    async claimInterface(interfaceNumber) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_claim_interface",
        { rid: this.#deviceHandleRid, interfaceNumber },
      );
    }

    async releaseInterface(interfaceNumber) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_release_interface",
        { rid: this.#deviceHandleRid, interfaceNumber },
      );
    }

    async selectConfiguration(configurationValue) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_select_configuration",
        { rid: this.#deviceHandleRid, configurationValue },
      );
    }

    async selectAlternateInterface(interfaceNumber, alternateSetting) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_select_alternate_interface",
        { rid: this.#deviceHandleRid, interfaceNumber, alternateSetting },
      );
    }

    async clearHalt(direction, endpointNumber) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_clear_halt",
        { rid: this.#deviceHandleRid, direction, endpointNumber },
      );
    }

    async controlTransferOut(setup, data) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_control_transfer_out",
        { rid: this.#deviceHandleRid, setup, data },
      );
    }

    async controlTransferIn(setup, data) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_control_transfer_in",
        { rid: this.#deviceHandleRid, setup, data },
      );
    }

    async transferIn(endpointNumber, length) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_transfer_in",
        { rid: this.#deviceHandleRid, endpointNumber, length },
      );
    }

    async transferOut(endpointNumber, data) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_transfer_out",
        { rid: this.#deviceHandleRid, endpointNumber, data },
      );
    }

    async isochronousTransferIn(endpointNumber, packetLengths) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_iso_transfer_in",
        { rid: this.#deviceHandleRid, endpointNumber, packetLengths },
      );
    }

    async isochronousTransferOut(endpointNumber, data, packetLengths) {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_iso_transfer_out",
        { rid: this.#deviceHandleRid, endpointNumber, data, packetLengths },
      );
    }

    async reset() {
      if (!this.opened) throw new Error("The device must be opened first.");
      return await core.jsonOpAsync(
        "op_webusb_reset",
        { rid: this.#deviceHandleRid },
      );
    }

    async open() {
      if (this.opened) throw new Error("The device is already opened.");
      const { rid } = await core.jsonOpAsync(
        "op_webusb_open_device",
        { rid: this.#rid },
      );
      this.#deviceHandleRid = rid;
      this.opened = true;
    }

    async close() {
      if (!this.opened) throw new Error("The device must be opened first.");
      await core.jsonOpAsync("op_webusb_close_device", {
        rid: this.#deviceHandleRid,
      });
      this.opened = false;
    }
  }

  async function getDevices() {
    const devices = await core.jsonOpAsync("op_webusb_get_devices", {});
    return devices.map(({ rid, usbdevice }) => new UsbDevice(usbdevice, rid));
  }

  window.usb = {
    getDevices,
    UsbDevice,
  };
  window.__bootstrap = window.__bootstrap || {};
  window.__bootstrap.usb = {
    getDevices,
    UsbDevice,
  };
})(this);
