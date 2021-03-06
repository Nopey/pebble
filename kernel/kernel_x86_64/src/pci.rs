use acpi::PciConfigRegions;
use alloc::collections::BTreeMap;
use aml::{pci_routing::PciRoutingTable, AmlContext, AmlName};
use core::ptr;
use hal::memory::PhysicalAddress;
use hal_x86_64::kernel_map;
use kernel::pci::{PciDevice, PciInfo};
use log::info;
use pci_types::{ConfigRegionAccess, PciAddress, PciHeader};

#[derive(Clone)]
pub struct EcamAccess(PciConfigRegions);

impl EcamAccess {
    pub fn new(regions: PciConfigRegions) -> EcamAccess {
        EcamAccess(regions)
    }
}

impl ConfigRegionAccess for EcamAccess {
    fn function_exists(&self, address: PciAddress) -> bool {
        self.0.physical_address(address.segment(), address.bus(), address.device(), address.function()).is_some()
    }

    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        let physical_address = self
            .0
            .physical_address(address.segment(), address.bus(), address.device(), address.function())
            .unwrap();
        let ptr = (kernel_map::physical_to_virtual(PhysicalAddress::new(physical_address as usize).unwrap())
            + offset as usize)
            .ptr();
        ptr::read_volatile(ptr)
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        let physical_address = self
            .0
            .physical_address(address.segment(), address.bus(), address.device(), address.function())
            .unwrap();
        let ptr = (kernel_map::physical_to_virtual(PhysicalAddress::new(physical_address as usize).unwrap())
            + offset as usize)
            .mut_ptr();
        ptr::write_volatile(ptr, value)
    }
}

pub struct PciResolver<A>
where
    A: ConfigRegionAccess,
{
    access: A,
    info: PciInfo,
}

impl<A> PciResolver<A>
where
    A: ConfigRegionAccess,
{
    pub fn resolve(access: A) -> PciInfo {
        let mut resolver = Self { access, info: PciInfo { devices: BTreeMap::new() } };

        /*
         * If the device at 0:0:0:0 has multiple functions, there are multiple PCI host controllers, so we need to
         * check all the functions.
         */
        if PciHeader::new(PciAddress::new(0, 0, 0, 0)).has_multiple_functions(&resolver.access) {
            for bus in 0..8 {
                resolver.check_bus(bus);
            }
        } else {
            resolver.check_bus(0);
        }

        resolver.info
    }

    fn check_bus(&mut self, bus: u8) {
        for device in 0..32 {
            self.check_device(bus, device);
        }
    }

    fn check_device(&mut self, bus: u8, device: u8) {
        let address = PciAddress::new(0, bus, device, 0);
        if self.access.function_exists(address) {
            self.check_function(bus, device, 0);

            let header = PciHeader::new(address);
            if header.has_multiple_functions(&self.access) {
                /*
                 * The device is multi-function. We need to check the rest.
                 */
                for function in 1..8 {
                    self.check_function(bus, device, function);
                }
            }
        }
    }

    fn check_function(&mut self, bus: u8, device: u8, function: u8) {
        let address = PciAddress::new(0, bus, device, function);
        if self.access.function_exists(address) {
            let header = PciHeader::new(address);
            let (vendor_id, device_id) = header.id(&self.access);
            let (revision, class, sub_class, interface) = header.revision_and_class(&self.access);

            if vendor_id == 0xffff {
                return;
            }

            // TODO: check if the function is a PCI-to-PCI bridge, and if so, call check_bus on the secondary bus
            // number (from the bridge's config space)

            info!(
                "Found PCI device (bus={}, device={}, function={}): (vendor = {:#x}, device = {:#x})",
                bus, device, function, vendor_id, device_id
            );

            self.info
                .devices
                .insert(address, PciDevice { vendor_id, device_id, revision, class, sub_class, interface });
        }
    }
}
