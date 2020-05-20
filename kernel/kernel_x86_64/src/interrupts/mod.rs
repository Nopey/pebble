mod exception;
mod pci;

use acpi::{interrupt::InterruptModel, Acpi};
use aml::{value::Args as AmlArgs, AmlContext, AmlName, AmlValue};
use bit_field::BitField;
use core::time::Duration;
use hal::memory::PhysicalAddress;
use hal_x86_64::{
    hw::{
        cpu::CpuInfo,
        gdt::KERNEL_CODE_SELECTOR,
        i8259_pic::Pic,
        idt::{wrap_handler, wrap_handler_with_error_code, Idt, InterruptStackFrame},
        local_apic::LocalApic,
        registers::write_msr,
    },
    kernel_map,
};
use log::warn;
use pci::PciResolver;
use pebble_util::InitGuard;

global_asm!(include_str!("syscall.s"));
extern "C" {
    fn syscall_handler() -> !;
}

#[no_mangle]
extern "C" fn rust_syscall_entry(number: usize, a: usize, b: usize, c: usize, d: usize, e: usize) -> usize {
    kernel::syscall::handle_syscall::<crate::PlatformImpl>(number, a, b, c, d, e)
}

/// This should only be accessed directly by the bootstrap processor.
///
/// The IDT is laid out like so:
/// |------------------|-----------------------------|
/// | Interrupt Vector |            Usage            |
/// |------------------|-----------------------------|
/// |       00-1f      | Intel Reserved (Exceptions) |
/// |       20-2f      | i8259 PIC Interrupts        |
/// |       30-??      | IOAPIC Interrupts           |
/// |        ..        |                             |
/// |        fe        | Local APIC timer
/// |        ff        | APIC spurious interrupt     |
/// |------------------|-----------------------------|
static mut IDT: Idt = Idt::empty();

static LOCAL_APIC: InitGuard<LocalApic> = InitGuard::uninit();

/*
 * These constants define the IDT's layout. Refer to the documentation of the `IDT` static for
 * the full layout.
 */
const LEGACY_PIC_VECTOR: u8 = 0x20;
const FREE_VECTORS_START: u8 = 0x30;
const APIC_TIMER_VECTOR: u8 = 0xfe;
const APIC_SPURIOUS_VECTOR: u8 = 0xff;

pub struct InterruptController {}

impl InterruptController {
    pub fn init(acpi_info: &Acpi, aml_context: &mut AmlContext) -> InterruptController {
        Self::install_syscall_handler();
        Self::install_exception_handlers();

        unsafe {
            IDT.load();
        }

        match acpi_info.interrupt_model.as_ref().unwrap() {
            InterruptModel::Apic(info) => {
                if info.also_has_legacy_pics {
                    unsafe { Pic::new() }.remap_and_disable(LEGACY_PIC_VECTOR, LEGACY_PIC_VECTOR + 8);
                }

                /*
                 * Initialise `LOCAL_APIC` to point at the right address.
                 * TODO: we might need to map it separately or something so we can set custom flags on the
                 * paging entry (do we need to set NO_CACHE on it?)
                 */
                LOCAL_APIC.initialize(unsafe {
                    LocalApic::new(kernel_map::physical_to_virtual(
                        PhysicalAddress::new(info.local_apic_address as usize).unwrap(),
                    ))
                });

                /*
                 * Tell ACPI that we intend to use the APICs instead of the legacy PIC.
                 */
                aml_context
                    .invoke_method(
                        &AmlName::from_str("\\_PIC").unwrap(),
                        AmlArgs { arg_0: Some(AmlValue::Integer(1)), ..Default::default() },
                    )
                    .expect("Failed to invoke \\_PIC method");

                /*
                 * Resolve all the PCI info.
                 * XXX: not sure this is the right place to do this just yet.
                 */
                let pci_info = PciResolver::resolve(acpi_info.pci_config_regions.as_ref().unwrap(), aml_context);

                // /*
                //  * Map the local APIC's configuration space into the kernel address space.
                //  */
                // arch.kernel_page_table
                //     .lock()
                //     .mapper()
                //     .map_to(
                //         Page::contains(kernel_map::LOCAL_APIC_CONFIG),
                //         Frame::contains(PhysicalAddress::new(info.local_apic_address as usize).unwrap()),
                //         EntryFlags::PRESENT | EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE |
                // EntryFlags::NO_CACHE,         &arch.physical_memory_manager,
                //     )
                //     .unwrap();

                /*
                 * Install handlers for the spurious interrupt and local APIC timer, and then
                 * enable the local APIC.
                 * Install a spurious interrupt handler and enable the local APIC.
                 */
                unsafe {
                    IDT[APIC_TIMER_VECTOR]
                        .set_handler(wrap_handler!(local_apic_timer_handler), KERNEL_CODE_SELECTOR);
                    IDT[APIC_SPURIOUS_VECTOR].set_handler(wrap_handler!(spurious_handler), KERNEL_CODE_SELECTOR);
                    LOCAL_APIC.get().enable(APIC_SPURIOUS_VECTOR);
                }

                InterruptController {}
            }

            _ => panic!("Unsupported interrupt model!"),
        }
    }

    /// Enable the per-CPU timer on the local APIC, so that it ticks every `period` ms. Cannot be
    /// called before interrupt handlers are installed, because this borrows `self`.
    pub fn enable_local_timer(&mut self, cpu_info: &CpuInfo, period: Duration) {
        /*
         * TODO: currently, this relies upon being able to get the frequency from the
         * CpuInfo. We should probably build a backup to calibrate it using another timer.
         */
        match cpu_info.apic_frequency() {
            Some(apic_frequency) => {
                LOCAL_APIC.get().enable_timer(period.as_millis() as u32, apic_frequency, APIC_TIMER_VECTOR);
            }
            None => warn!("Couldn't find frequency of APIC from cpuid. Local APIC timer not enabled!"),
        }
    }

    fn install_exception_handlers() {
        macro set_handler($name: ident, $handler: path) {
            unsafe {
                IDT.$name().set_handler(wrap_handler!($handler), KERNEL_CODE_SELECTOR);
            }
        }

        macro set_handler_with_error_code($name: ident, $handler: path) {
            unsafe {
                IDT.$name().set_handler(wrap_handler_with_error_code!($handler), KERNEL_CODE_SELECTOR);
            }
        }

        set_handler!(nmi, exception::nmi_handler);
        set_handler!(breakpoint, exception::breakpoint_handler);
        set_handler!(invalid_opcode, exception::invalid_opcode_handler);
        set_handler_with_error_code!(general_protection_fault, exception::general_protection_fault_handler);
        set_handler_with_error_code!(page_fault, exception::page_fault_handler);
        set_handler_with_error_code!(double_fault, exception::double_fault_handler);
    }

    fn install_syscall_handler() {
        /*
         * On x86_64, the `syscall` instruction will always be present, so we only support that
         * for making system calls.
         *
         * Refer to the documentation comments of each MSR to understand what this code is doing.
         */
        use hal_x86_64::hw::{
            gdt::USER_COMPAT_CODE_SELECTOR,
            registers::{IA32_FMASK, IA32_LSTAR, IA32_STAR},
        };

        let mut selectors = 0_u64;
        selectors.set_bits(32..48, KERNEL_CODE_SELECTOR.0 as u64);

        /*
         * We put the selector for the Compatibility-mode code segment in here, because `sysret` expects
         * the segments to be in this order:
         *      STAR[48..64]        => 32-bit Code Segment
         *      STAR[48..64] + 8    => Data Segment
         *      STAR[48..64] + 16   => 64-bit Code Segment
         */
        selectors.set_bits(48..64, USER_COMPAT_CODE_SELECTOR.0 as u64);

        unsafe {
            write_msr(IA32_STAR, selectors);
            write_msr(IA32_LSTAR, syscall_handler as u64);
            // TODO: set this up properly (e.g. disable interrupts until we switch stacks, set up DF and such to be
            // as the kernel expects them)
            write_msr(IA32_FMASK, 0x0);
        }
    }
}

extern "C" fn local_apic_timer_handler(_: &InterruptStackFrame) {
    info!("Tick!");
    unsafe {
        LOCAL_APIC.get().send_eoi();
    }
}

extern "C" fn spurious_handler(_: &InterruptStackFrame) {}
