// TODO: WPT tests
// deno-lint-ignore-file
class FakeDevice {
  constructor(deviceInit) {
    this.info_ = deviceInit;
    this.opened_ = false;
    this.currentConfiguration_ = null;
    this.claimedInterfaces_ = new Map();
  }

  async getConfiguration() {
    return {
      value: this.currentConfiguration_
        ? this.currentConfiguration_.configurationValue
        : 0,
    };
  }

  async open() {
    assert_false(this.opened_);
    this.opened_ = true;
  }

  async close() {
    assert_true(this.opened_);
    this.opened_ = false;
  }

  async setConfiguration(value) {
    assert_true(this.opened_);

    let selectedConfiguration = this.info_.configurations.find(
      (configuration) => configuration.configurationValue == value,
    );
    this.currentConfiguration_ = selectedConfiguration;
    return { success: true };
  }

  async claimInterface(interfaceNumber) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    assert_false(
      this.claimedInterfaces_.has(interfaceNumber),
      "interface already claimed",
    );
    let iface = this.currentConfiguration_.interfaces.find(
      (iface) => iface.interfaceNumber == interfaceNumber,
    );

    assert_false(iface == undefined);
    this.claimedInterfaces_.set(interfaceNumber, 0);
    return { success: true };
  }

  async releaseInterface(interfaceNumber) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    assert_true(this.claimedInterfaces_.has(interfaceNumber));
    this.claimedInterfaces_.delete(interfaceNumber);
    return { success: true };
  }

  async setInterfaceAlternateSetting(interfaceNumber, alternateSetting) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    assert_true(this.claimedInterfaces_.has(interfaceNumber));

    let iface = this.currentConfiguration_.interfaces.find(
      (iface) => iface.interfaceNumber == interfaceNumber,
    );

    assert_false(iface == undefined);
    assert_true(iface.alternates.some(
      (x) => x.alternateSetting == alternateSetting,
    ));
    this.claimedInterfaces_.set(interfaceNumber, alternateSetting);
    return { success: true };
  }

  async reset() {
    assert_true(this.opened_);
    return { success: true };
  }

  async clearHalt(endpoint) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    return { success: true };
  }

  async controlTransferIn(params, length) {
    assert_true(this.opened_);

    if (
      (params.recipient == "interface" ||
        params.recipient == "endpoint") &&
      this.currentConfiguration_ == null
    ) {
      return {
        status: "PERMISSION_DENIED",
      };
    }

    return {
      status: "ok",
      data: [
        length >> 8,
        length & 0xff,
        params.request,
        params.value >> 8,
        params.value & 0xff,
        params.index >> 8,
        params.index & 0xff,
      ],
    };
  }

  async controlTransferOut(params, data) {
    assert_true(this.opened_);

    if (
      (params.recipient == "interface" ||
        params.recipient == "endpoint") &&
      this.currentConfiguration_ == null
    ) {
      return {
        status: "PERMISSION_DENIED",
      };
    }

    return { status: "ok", bytesWritten: data.byteLength };
  }

  async isochronousTransferIn(endpointNumber, packetLengths, timeout) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    let data = new Array(packetLengths.reduce((a, b) => a + b, 0));
    let dataOffset = 0;
    let packets = new Array(packetLengths.length);
    for (let i = 0; i < packetLengths.length; ++i) {
      for (let j = 0; j < packetLengths[i]; ++j) {
        data[dataOffset++] = j & 0xff;
      }
      packets[i] = {
        length: packetLengths[i],
        transferredLength: packetLengths[i],
        status: "ok",
      };
    }
    return { data: data, packets: packets };
  }

  async isochronousTransferOut(endpointNumber, data, packetLengths, timeout) {
    assert_true(this.opened_);
    assert_false(this.currentConfiguration_ == null, "device configured");
    let packets = new Array(packetLengths.length);
    for (let i = 0; i < packetLengths.length; ++i) {
      packets[i] = {
        length: packetLengths[i],
        transferredLength: packetLengths[i],
        status: "ok",
      };
    }
    return { packets: packets };
  }
}

class FakeUSBDevice {
  constructor() {
    this.onclose = null;
  }

  disconnect() {
    setTimeout(() => internal.webUsbService.removeDevice(this), 0);
  }
}

class UsbTest {
  #intialized;
  #devices;

  constructor() {
    this.#intialized = false;
    this.#devices = [];
  }

  async intialize() {
    if (this.#intialized) return;
    this.#intialized = true;
  }

  async attachToContext() {
    if (!this.#initialized) {
      throw new Error("Call initialize() before attachToContext()");
    }
    return true;
  }

  async addFakeDevice(device) {
    if (!this.#initialized) {
      throw new Error("Call initialize() before addFakeDevice().");
    }
    let fakeDevice = new FakeUSBDevice(device);
    this.#devices.push(fakeDevice);
  }
}
