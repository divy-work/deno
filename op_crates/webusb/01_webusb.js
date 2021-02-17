// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  
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
