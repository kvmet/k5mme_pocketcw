//! The USB device peripheral (PA11, PA12). Its registers and packet memory match the STM32F103
//! USB block, so `stm32-usbd` drives it; the AT32 adds the D+ pull-up switch in CFG.

use at32f4xx_pac::at32f403a as pac;
use stm32_usbd::{MemoryAccess, UsbBus, UsbPeripheral};
use usb_device::bus::UsbBusAllocator;

use crate::clock;

/// How long the host sees no device before `attach`. USB needs only 2.5 us; the margin covers
/// hosts that poll their ports slowly.
const DETACH_MS: u32 = 20;

/// CFG.USBDIV for PLL / 1.5, which gives 48 MHz from SYSCLK_HZ.
const USB_DIV: u8 = 0b000;
const _: () = assert!(clock::SYSCLK_HZ == 72_000_000);

pub struct Usb;

unsafe impl UsbPeripheral for Usb {
    const REGISTERS: *const () = pac::USBFS::ptr() as *const ();
    // The pull-up lives in CFG.PUO, which `stm32-usbd` does not know; `attach` and `detach` drive it.
    const DP_PULL_UP_FEATURE: bool = false;
    const EP_MEMORY: *const () = 0x4000_6000 as *const ();
    const EP_MEMORY_SIZE: usize = 512;
    const EP_MEMORY_ACCESS: MemoryAccess = MemoryAccess::Word16x1;

    /// Clocks USB at 48 MHz from the PLL and resets the peripheral, which the bootloader leaves
    /// running. The bootloader's clock choices are not known, so both are set here.
    fn enable() {
        let crm = unsafe { &*pac::CRM::ptr() };
        crm.misc3().modify(|_, w| w.hick_to_usb().clear_bit());
        crm.cfg().modify(|_, w| unsafe { w.usbdiv2().bit(USB_DIV >> 2 != 0).usbdiv1_0().bits(USB_DIV & 0b11) });
        crm.apb1en().modify(|_, w| w.usb().set_bit());
        crm.apb1rst().modify(|_, w| w.usb().set_bit());
        crm.apb1rst().modify(|_, w| w.usb().clear_bit());
    }

    /// The transceiver start time after power down clears, 1 us on the STM32F103.
    fn startup_delay() {
        clock::delay_us(1);
    }
}

/// Resets the peripheral and holds it off the bus for DETACH_MS, so the host drops the bootloader's
/// drive. Build the `UsbDevice` from the result, then call `attach`.
pub fn bus() -> UsbBusAllocator<UsbBus<Usb>> {
    let bus = UsbBus::new(Usb);
    detach();
    clock::delay_ms(DETACH_MS);
    bus
}

pub fn attach() {
    registers().cfg().modify(|_, w| w.puo().clear_bit());
}

pub fn detach() {
    registers().cfg().modify(|_, w| w.puo().set_bit());
}

fn registers() -> &'static pac::usbfs::RegisterBlock {
    unsafe { &*pac::USBFS::ptr() }
}
