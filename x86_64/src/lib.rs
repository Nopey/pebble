/*
 * Copyright (C) 2017, Isaac Woods.
 * See LICENCE.md
 */

#![no_std]

#![feature(lang_items)]
#![feature(asm)]
#![feature(const_fn)]
#![feature(naked_functions)]
#![feature(core_intrinsics)]
#![feature(alloc)]
#![feature(type_ascription)]
#![feature(allocator_api)]

/*
 * `rlibc` just provides intrinsics that are linked against, and so the compiler doesn't pick up
 * that it's actually used, so we suppress the warning.
 */
#[allow(unused_extern_crates)] extern crate rlibc;

                extern crate volatile;
                extern crate spin;
                extern crate alloc;
#[macro_use]    extern crate bitflags;
                extern crate bit_field;
                extern crate hole_tracking_allocator as allocator;
#[macro_use]    extern crate log;
#[macro_use]    extern crate kernel;
                extern crate pebble_syscall_common;
                extern crate xmas_elf;

#[macro_use]    mod registers;
#[macro_use]    mod vga_buffer;
#[macro_use]    mod serial;
                mod panic;
                mod memory;
                mod interrupts;
                mod gdt;
                mod idt;
                mod tlb;
                mod tss;
                mod i8259_pic;
                mod apic;
                mod port;
                mod multiboot2;
                mod acpi;
                mod process;

pub use panic::panic_fmt;

use memory::paging::PhysicalAddress;
use acpi::AcpiInfo;
use kernel::{Architecture,process::ProcessId};
use gdt::Gdt;
use tss::Tss;
use process::Process;

struct X86_64
{
}

impl Architecture for X86_64
{
    type MemoryAddress = memory::paging::VirtualAddress;

    fn clear_screen(&self)
    {
        vga_buffer::WRITER.lock().clear_buffer();
    }
}

pub static mut TSS : Tss = Tss::new();

#[no_mangle]
pub extern fn kstart(multiboot_address : PhysicalAddress) -> !
{
    use multiboot2::BootInformation;

    serial::initialise();
    log::set_logger(&serial::SERIAL_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    info!("Kernel connected to COM1");

    /*
     * We are passed the *physical* address of the Multiboot struct, so we offset it by the virtual
     * offset of the whole kernel.
     */
    let boot_info = unsafe { BootInformation::load(multiboot_address.into()) };
    let mut memory_controller = memory::init(&boot_info);

    /*
     * We can create and install a TSS and new GDT.
     *
     * Allocate a 4KiB stack for the double-fault handler. Using a separate stack for double-faults
     * avoids a triple fault happening when the guard page of the normal stack is hit (after a stack
     * overflow), which would otherwise:
     *      Page Fault -> Page Fault -> Double Fault -> Page Fault -> Triple Fault
     */
    let double_fault_stack = memory_controller.alloc_stack(1).expect("Failed to allocate stack");
    unsafe
    {
        TSS.interrupt_stack_table[tss::DOUBLE_FAULT_IST_INDEX] = double_fault_stack.top();
        TSS.set_kernel_stack(memory::get_kernel_stack_top());
    }
    let gdt_selectors = Gdt::install(unsafe { &mut TSS });

    /*
     * We now find and parse the ACPI tables. This also initialises the local APIC and IOAPIC, as
     * they are detailed by the MADT.
     */
    let acpi_info = AcpiInfo::new(&boot_info, &mut memory_controller);
    interrupts::init(&gdt_selectors);
    apic::LOCAL_APIC.lock().enable_timer(6);
    unsafe { asm!("sti"); }

    // TODO: parse ELF and start process properly
    let module_tag = boot_info.modules().nth(0).unwrap();
    info!("Running module: {}", module_tag.name());

    let mut process = Process::new(ProcessId(0),
                                   module_tag.start_address(),
                                   module_tag.end_address(),
                                   &mut memory_controller);
    unsafe { process.drop_to_usermode(gdt_selectors, &mut memory_controller); }

    // let virtual_address = module_tag.start_address().into_kernel_space();
    // unsafe { enter_usermode(virtual_address, gdt_selectors); }

    /*
     * Pass control to the kernel proper.
     */
    // let arch = X86_64 { };
    // kernel::kernel_main(arch);

    loop { }
}

#[lang = "eh_personality"]
extern fn eh_personality()
{
}