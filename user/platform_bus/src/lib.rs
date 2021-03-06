//! The **Platform Bus** is a concept of a single, abstract "bus" that all devices in the system hang off. These
//! devices are contributed by various **Bus Drivers**, which register devices with the Platfom Bus when they
//! enumerate their respective physical buses. **Device Drivers** can then register interest with the Platform Bus
//! for a specific class of devices using a **Filter**.
//!
//! Devices on the Platform Bus are described by Properties, which provide both generic and platform-specific
//! information. For example, a device created by the PCI bus driver will have `pci.vendor_id`, `pci.device_id`,
//! `pci.class` and `pci.sub_class` as properties. A Device Driver could use the `class` and `subclass` properties
//! to select all PCI devices of a particular type (e.g. useful for a driver for all EHCI controllers), or the
//! `vendor_id` and `device_id` properties to select a specific device (e.g. useful for a graphics driver for a
//! specific graphics card).

#![no_std]

extern crate alloc;

use alloc::{collections::BTreeMap, string::String};
use serde::{Deserialize, Serialize};

type DeviceName = String;
type PropertyName = String;

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    properties: BTreeMap<PropertyName, Property>,
}

impl Device {
    pub fn new(properties: BTreeMap<PropertyName, Property>) -> Device {
        Device { properties }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Property {
    Bool(bool),
    Integer(u64),
    String(String),
}

/// These are messages sent from Bus Drivers to the Platform Bus.
#[derive(Serialize, Deserialize, Debug)]
pub enum BusDriverMessage {
    RegisterDevice(DeviceName, Device),
    AddProperty(PropertyName, Property),
    RemoveProperty(PropertyName),
    // TODO: this could have messages to handle hot-plugging (Bus Driver tells Platform Bus a device was removed,
    // we pass that on to the Device Driver if the device was claimed by one)
}
