#![no_std]
#![no_main]
#![feature(const_generics, alloc_error_handler)]

extern crate alloc;
extern crate rlibc;

use alloc::{collections::BTreeMap, string::ToString, vec::Vec};
use core::{convert::TryFrom, panic::PanicInfo};
use libpebble::{
    caps::{CapabilitiesRepr, CAP_EARLY_LOGGING, CAP_PADDING, CAP_PCI_BUS_DRIVER, CAP_SERVICE_USER},
    early_logger::EarlyLogger,
    syscall,
};
use linked_list_allocator::LockedHeap;
use log::info;
use pci_types::device_type::{DeviceType, UsbType};
use platform_bus::{BusDriverMessage, Device, Property};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    syscall::early_log("Hello from pci_bus!!").unwrap();
    // Initialise the heap
    const HEAP_START: usize = 0x600000000;
    const HEAP_SIZE: usize = 0x4000;
    let heap_memory_object = syscall::create_memory_object(HEAP_START, HEAP_SIZE, true, false).unwrap();
    unsafe {
        syscall::map_memory_object(heap_memory_object, libpebble::ZERO_HANDLE, 0x0 as *mut usize).unwrap();
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    log::set_logger(&EarlyLogger).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    info!("PCI bus driver is running!");

    let platform_bus_channel = syscall::subscribe_to_service("platform_bus.bus_driver").unwrap();

    let descriptors = syscall::pci_get_info_vec().expect("Failed to get PCI descriptors");
    for descriptor in descriptors {
        info!(
            "PCI device at {}: {:x}:{:x} (class = {}, sub = {})",
            descriptor.address, descriptor.vendor_id, descriptor.device_id, descriptor.class, descriptor.sub_class
        );
        let device_type = DeviceType::from((descriptor.class, descriptor.sub_class));
        info!("Device type: {:?}", device_type);
        if device_type == DeviceType::UsbController {
            info!("USB controller type: {:?}", UsbType::try_from(descriptor.interface).unwrap());
        }

        /*
         * Register the device with the Platform Bus.
         */
        {
            let name = "pci-".to_string() + &descriptor.address.to_string();
            let mut properties = BTreeMap::new();
            properties.insert("pci.vendor_id".to_string(), Property::Integer(descriptor.vendor_id as u64));
            properties.insert("pci.device_id".to_string(), Property::Integer(descriptor.device_id as u64));
            properties.insert("pci.class".to_string(), Property::Integer(descriptor.class as u64));
            properties.insert("pci.sub_class".to_string(), Property::Integer(descriptor.sub_class as u64));

            let mut bytes = Vec::new();
            ptah::to_wire(&BusDriverMessage::RegisterDevice(name, Device::new(properties)), &mut bytes).unwrap();
            syscall::send_message(platform_bus_channel, &bytes, &[]).unwrap();
        }
    }

    loop {
        syscall::yield_to_kernel();
    }
}

#[panic_handler]
pub fn handle_panic(info: &PanicInfo) -> ! {
    log::error!("PANIC: {}", info);
    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Alloc error: {:?}", layout);
}

#[used]
#[link_section = ".caps"]
pub static mut CAPS: CapabilitiesRepr<4> =
    CapabilitiesRepr::new([CAP_EARLY_LOGGING, CAP_PCI_BUS_DRIVER, CAP_SERVICE_USER, CAP_PADDING]);
