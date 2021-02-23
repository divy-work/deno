// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

// #![deny(warnings)]

use deno_core::error::bad_resource_id;
use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::serde_json::json;
use deno_core::serde_json::Value;
use deno_core::AsyncRefCell;
use deno_core::BufVec;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ZeroCopyBuf;
use libusb1_sys::constants::*;
use libusb1_sys::libusb_alloc_transfer;
use libusb1_sys::libusb_fill_iso_transfer;
use libusb1_sys::libusb_submit_transfer;
use libusb1_sys::libusb_transfer_cb_fn;
use rusb::request_type;
use rusb::{Context, Device, DeviceHandle, UsbContext};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub use rusb; // Re-export rusb

static EP_DIR_IN: u8 = 0x80;
static EP_DIR_OUT: u8 = 0x0;

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

#[derive(Serialize, Clone)]
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
  manufacturer_name: Option<String>,
  product_id: u16,
  product_name: Option<String>,
  serial_number: Option<String>,
  usb_version_major: u8,
  usb_version_minor: u8,
  usb_version_subminor: u8,
  vendor_id: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenArgs {
  rid: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaimInterfaceArgs {
  rid: u32,
  interface_number: u8,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectConfigurationArgs {
  rid: u32,
  configuration_value: u8,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectAlternateInterfaceArgs {
  rid: u32,
  interface_number: u8,
  alternate_setting: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Direction {
  In,
  Out,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearHaltArgs {
  rid: u32,
  direction: Direction,
  endpoint_number: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferInArgs {
  rid: u32,
  length: usize,
  endpoint_number: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferOutArgs {
  rid: u32,
  data: Vec<u8>,
  endpoint_number: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum WebUSBRequestType {
  Standard,
  Class,
  Vendor,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum WebUSBRecipient {
  Device,
  Interface,
  Endpoint,
  Other,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum WebUSBTransferStatus {
  Completed,
  TransferError,
  Timeout,
  Stall,
  Disconnect,
  Babble,
  Cancelled,
}

impl WebUSBTransferStatus {
  pub fn from_libusb_status(status: i32) -> Self {
    match status {
      LIBUSB_TRANSFER_COMPLETED => WebUSBTransferStatus::Completed,
      LIBUSB_TRANSFER_ERROR => WebUSBTransferStatus::TransferError,
      // Should never happen but no harm to keep it.
      LIBUSB_TRANSFER_TIMED_OUT => WebUSBTransferStatus::Timeout,
      LIBUSB_TRANSFER_STALL => WebUSBTransferStatus::Stall,
      LIBUSB_TRANSFER_NO_DEVICE => WebUSBTransferStatus::Disconnect,
      LIBUSB_TRANSFER_OVERFLOW => WebUSBTransferStatus::Babble,
      LIBUSB_TRANSFER_CANCELLED => WebUSBTransferStatus::Completed,
      // Unreachable but we'll settle for a TransferError.
      _ => WebUSBTransferStatus::TransferError,
    }
  }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupArgs {
  request_type: WebUSBRequestType,
  recipient: WebUSBRecipient,
  request: u8,
  value: u16,
  index: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlTransferOutArgs {
  rid: u32,
  setup: SetupArgs,
  data: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsoTransferInArgs {
  rid: u32,
  endpoint_number: u8,
  packet_lengths: Vec<i32>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IsochronousPacket {
  data: Vec<u8>,
  status: WebUSBTransferStatus,
}

impl IsochronousPacket {
  pub fn new(data: Vec<u8>, status: WebUSBTransferStatus) -> Self {
    Self { data, status }
  }
}

#[derive(Clone)]
struct IsochronousPackets(Vec<IsochronousPacket>);

impl IsochronousPackets {
  pub fn new() -> Self {
    Self(vec![])
  }

  pub fn push(&mut self, packet: IsochronousPacket) {
    self.0.push(packet);
  }

  pub fn packets(&mut self) -> Vec<IsochronousPacket> {
    self.0.clone()
  }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IsochronousTransferOutResult {
  packets: Vec<IsochronousPacket>,
  data: Vec<u8>,
}

impl IsochronousTransferOutResult {
  pub fn new(packets: Vec<IsochronousPacket>, data: Vec<u8>) -> Self {
    Self { packets, data }
  }
}

pub struct UsbResource {
  device: Device<Context>,
}

pub struct UsbHandleResource {
  handle: AsyncRefCell<DeviceHandle<Context>>,
}

impl Resource for UsbHandleResource {
  fn name(&self) -> Cow<str> {
    "usbDeviceHandle".into()
  }
}

impl Resource for UsbResource {
  fn name(&self) -> Cow<str> {
    "usbDevice".into()
  }
}

pub async fn op_webusb_open_device(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: OpenArgs = serde_json::from_value(args)?;
  let rid = args.rid;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let handle = resource.device.open()?;
  let rid = state.borrow_mut().resource_table.add(UsbHandleResource {
    handle: AsyncRefCell::new(handle),
  });
  Ok(json!({ "rid": rid }))
}

pub async fn op_webusb_reset(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  // Note: Reusing `OpenArgs` struct here. The rid is for the device handle.
  let args: OpenArgs = serde_json::from_value(args)?;
  let rid = args.rid;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.reset()?;
  Ok(json!({}))
}

pub async fn op_webusb_close_device(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  // Note: Reusing `OpenArgs` struct here. The rid is for the device handle.
  let args: OpenArgs = serde_json::from_value(args)?;
  let rid = args.rid;

  let resource = state
    .borrow_mut()
    .resource_table
    .take::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  drop(handle);
  Ok(json!({}))
}

pub async fn op_webusb_select_configuration(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: SelectConfigurationArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let configuration_value = args.configuration_value;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.set_active_configuration(configuration_value)?;
  Ok(json!({}))
}

pub async fn op_webusb_transfer_out(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: TransferOutArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let endpoint_number = args.endpoint_number;

  // Ported from the Chromium codebase.
  // https://chromium.googlesource.com/chromium/src/+/master/services/device/usb/usb_device_handle_impl.cc#789
  let endpoint_addr = EP_DIR_IN | endpoint_number;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;

  let mut transfer_type: Option<rusb::TransferType> = None;
  let cnf = handle
    .device() // -> Device<T>
    .active_config_descriptor()?; // -> ConfigDescriptor<T>
  let interfaces = cnf.interfaces(); // -> Interfaces<'a>

  for interface in interfaces {
    for descriptor in interface.descriptors() {
      // InterfaceDescriptor in Vec<Interface<'a>>
      let endpoint_desc = descriptor
        .endpoint_descriptors()
        .find(|s| &s.address() == &endpoint_addr);
      if endpoint_desc.is_none() {
        continue;
      }
      transfer_type = Some(endpoint_desc.unwrap().transfer_type());
      // find the address of a Endpoint in every EndpointDescriptor of every InterfaceDescriptor.
    }
  }

  match transfer_type {
    Some(t) => {
      let data = args.data;
      let bytes_written = match t {
        rusb::TransferType::Bulk => {
          handle.write_bulk(endpoint_number, &data, Duration::new(0, 0))?
        }
        rusb::TransferType::Interrupt => {
          handle.write_interrupt(endpoint_number, &data, Duration::new(0, 0))?
        }
        _ => return Ok(json!({})),
      };
      Ok(json!({ "bytesWritten": bytes_written }))
    }
    None => Ok(json!({})),
  }
}

pub async fn op_webusb_transfer_in(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: TransferInArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let endpoint_number = args.endpoint_number;

  // Ported from the Chromium codebase.
  // https://chromium.googlesource.com/chromium/src/+/master/services/device/usb/usb_device_handle_impl.cc#789
  let endpoint_addr = EP_DIR_IN | endpoint_number;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;

  let mut transfer_type: Option<rusb::TransferType> = None;
  let cnf = handle
    .device() // -> Device<T>
    .active_config_descriptor()?; // -> ConfigDescriptor<T>
  let interfaces = cnf.interfaces(); // -> Interfaces<'a>

  for interface in interfaces {
    for descriptor in interface.descriptors() {
      // InterfaceDescriptor in Vec<Interface<'a>>
      let endpoint_desc = descriptor
        .endpoint_descriptors()
        .find(|s| &s.address() == &endpoint_addr);
      if endpoint_desc.is_none() {
        continue;
      }
      transfer_type = Some(endpoint_desc.unwrap().transfer_type());
      // find the address of a Endpoint in every EndpointDescriptor of every InterfaceDescriptor.
    }
  }

  match transfer_type {
    Some(t) => {
      let mut data = Vec::with_capacity(args.length);
      match t {
        rusb::TransferType::Bulk => {
          handle.read_bulk(endpoint_number, &mut data, Duration::new(0, 0))?
        }
        rusb::TransferType::Interrupt => handle.read_interrupt(
          endpoint_number,
          &mut data,
          Duration::new(0, 0),
        )?,
        _ => return Ok(json!({})),
      };
      Ok(json!({ "data": data }))
    }
    None => Ok(json!({})),
  }
}

extern "system" fn noop_transfer_cb(_: *mut libusb1_sys::libusb_transfer) {}

pub async fn op_webusb_iso_transfer_in(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: IsoTransferInArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let endpoint_addr = EP_DIR_IN | args.endpoint_number;
  let packet_lengths = args.packet_lengths;
  let length: i32 = packet_lengths.iter().sum();
  let num_packets = packet_lengths.len() as i32;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  unsafe {
    let mut buffer: Vec<u8> = vec![];

    let mut transfer = libusb_alloc_transfer(num_packets);

    let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
    libusb_fill_iso_transfer(
      transfer,
      handle.as_raw(),
      endpoint_addr,
      &mut buffer[0],
      length,
      num_packets,
      noop_transfer_cb,
      p,
      0,
    );

    for (i, packet) in packet_lengths.iter().enumerate() {
      (*transfer).iso_packet_desc[i].length = *packet as u32;
    }

    let rv = libusb_submit_transfer(transfer);
    if rv != LIBUSB_SUCCESS {
      // TODO: handle error
    }
    let mut packets = IsochronousPackets::new();
    for pkt in (*transfer).iso_packet_desc.iter() {
      let status = WebUSBTransferStatus::from_libusb_status(pkt.status);
      let packet = IsochronousPacket::new(vec![], status);
      packets.push(packet)
    }

    let result = IsochronousTransferOutResult::new(packets.packets(), buffer);
    Ok(json!(result))
  }
}

pub async fn op_webusb_control_transfer_in(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  // Re-using ControlTransferOut struct here.
  let args: ControlTransferOutArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let setup = args.setup;
  let buf = args.data;

  let req = match setup.request_type {
    WebUSBRequestType::Standard => rusb::RequestType::Standard,
    WebUSBRequestType::Class => rusb::RequestType::Class,
    WebUSBRequestType::Vendor => rusb::RequestType::Vendor,
  };

  let recipient = match setup.recipient {
    WebUSBRecipient::Device => rusb::Recipient::Device,
    WebUSBRecipient::Interface => rusb::Recipient::Interface,
    WebUSBRecipient::Endpoint => rusb::Recipient::Endpoint,
    WebUSBRecipient::Other => rusb::Recipient::Other,
  };

  let req_type = request_type(rusb::Direction::In, req, recipient);

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  // http://libusb.sourceforge.net/api-1.0/group__libusb__syncio.html
  // For unlimited timeout, use value `0`.
  let b = handle.write_control(
    req_type,
    setup.request,
    setup.value,
    setup.index,
    &buf,
    Duration::new(0, 0),
  )?;
  Ok(json!({}))
}

pub async fn op_webusb_control_transfer_out(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: ControlTransferOutArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let setup = args.setup;
  let buf = args.data;

  let req = match setup.request_type {
    WebUSBRequestType::Standard => rusb::RequestType::Standard,
    WebUSBRequestType::Class => rusb::RequestType::Class,
    WebUSBRequestType::Vendor => rusb::RequestType::Vendor,
  };

  let recipient = match setup.recipient {
    WebUSBRecipient::Device => rusb::Recipient::Device,
    WebUSBRecipient::Interface => rusb::Recipient::Interface,
    WebUSBRecipient::Endpoint => rusb::Recipient::Endpoint,
    WebUSBRecipient::Other => rusb::Recipient::Other,
  };

  let req_type = request_type(rusb::Direction::Out, req, recipient);

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  // http://libusb.sourceforge.net/api-1.0/group__libusb__syncio.html
  // For unlimited timeout, use value `0`.
  let b = handle.write_control(
    req_type,
    setup.request,
    setup.value,
    setup.index,
    &buf,
    Duration::new(0, 0),
  )?;
  Ok(json!({}))
}

pub async fn op_webusb_clear_halt(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: ClearHaltArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let direction: Direction = args.direction;

  let mut endpoint = args.endpoint_number;

  match direction {
    Direction::In => endpoint |= EP_DIR_IN,
    Direction::Out => endpoint |= EP_DIR_OUT,
  };

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.clear_halt(endpoint)?;
  Ok(json!({}))
}

pub async fn op_webusb_select_alternate_interface(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: SelectAlternateInterfaceArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let interface_number = args.interface_number;
  let alternate_setting = args.alternate_setting;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.set_alternate_setting(interface_number, alternate_setting)?;
  Ok(json!({}))
}

macro_rules! handle_err_to_none {
  ($e: expr) => {
    match $e {
      Err(_) => None,
      Ok(n) => Some(n),
    }
  };
}

pub async fn op_webusb_release_interface(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: ClaimInterfaceArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let interface_number = args.interface_number;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.release_interface(interface_number)?;
  Ok(json!({}))
}

pub async fn op_webusb_claim_interface(
  state: Rc<RefCell<OpState>>,
  args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let args: ClaimInterfaceArgs = serde_json::from_value(args)?;
  let rid = args.rid;
  let interface_number = args.interface_number;

  let resource = state
    .borrow()
    .resource_table
    .get::<UsbHandleResource>(rid)
    .ok_or_else(bad_resource_id)?;

  let mut handle = RcRef::map(resource, |r| &r.handle).borrow_mut().await;
  handle.claim_interface(interface_number)?;
  Ok(json!({}))
}

pub async fn op_webusb_get_devices(
  state: Rc<RefCell<OpState>>,
  _args: Value,
  _zero_copy: BufVec,
) -> Result<Value, AnyError> {
  let devices = rusb::Context::new().unwrap().devices().unwrap();

  #[derive(Serialize)]
  struct Device {
    usbdevice: UsbDevice,
    rid: u32,
  }

  let mut usbdevices: Vec<Device> = vec![];
  let mut state = state.borrow_mut();
  for device in devices.iter() {
    let device_descriptor = device.device_descriptor().unwrap();
    let device_class = device_descriptor.class_code();
    // Do not list hubs. Ignore them.
    if device_class == 9 {
      continue;
    };
    let config_descriptor = device.active_config_descriptor();
    let device_version = device_descriptor.device_version();
    let usb_version = device_descriptor.usb_version();

    let configuration = match config_descriptor {
      Ok(config_descriptor) => Some(UsbConfiguration {
        configuration_name: config_descriptor.description_string_index(),
        configuration_value: config_descriptor.number(),
      }),
      Err(_) => None,
    };

    let handle = device.open()?;
    let manufacturer_name = handle_err_to_none!(
      handle.read_manufacturer_string_ascii(&device_descriptor)
    );
    let product_name =
      handle_err_to_none!(handle.read_product_string_ascii(&device_descriptor));
    let serial_number = handle_err_to_none!(
      handle.read_serial_number_string_ascii(&device_descriptor)
    );
    let usbdevice = UsbDevice {
      configuration,
      device_class,
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
      manufacturer_name,
      product_name,
      serial_number,
    };

    // Explicitly close the device.
    drop(handle);

    let rid = state.resource_table.add(UsbResource { device });
    usbdevices.push(Device { usbdevice, rid });
  }

  Ok(json!(usbdevices))
}
