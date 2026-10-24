//! The USB device peripheral (PA11, PA12). Its registers and packet memory match the STM32F103
//! USB block, so `stm32-usbd` drives it; the AT32 adds the D+ pull-up switch in CFG.

use at32f4xx_pac::at32f403a as pac;
use pocketcw_logic::keyboard::{NO_KEYS, Report};
use stm32_usbd::{MemoryAccess, UsbBus, UsbPeripheral};
use usb_device::UsbError;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbDeviceState, UsbVidPid};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

use crate::clock;

/// Artery's VID and the PID of its keyboard example: the chip maker's ID for this class of
/// device, as the vendor bootloader uses Artery's mass storage example ID.
pub const VID_PID: UsbVidPid = UsbVidPid(0x2E3C, 0x6040);

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

pub type Bus = UsbBusAllocator<UsbBus<Usb>>;

/// A USB keyboard that sends one report at a time.
pub struct Keyboard {
    device: UsbDevice<'static, UsbBus<Usb>>,
    // No boot subclass: usbd-hid refuses every report from a boot keyboard in Report mode.
    hid: HIDClass<'static, UsbBus<Usb>>,
    /// The last report the host took.
    sent: Report,
}

impl Keyboard {
    /// The device stays off the bus until `attach`.
    pub fn new(bus: &'static Bus, product: &'static str) -> Keyboard {
        let hid = HIDClass::new_ep_in(bus, KeyboardReport::desc(), 1);
        let device = UsbDeviceBuilder::new(bus, VID_PID)
            .strings(&[StringDescriptors::default().manufacturer("PocketCW").product(product)])
            .unwrap_or_else(|_| unreachable!("one language fits"))
            .build();
        Keyboard { device, hid, sent: NO_KEYS }
    }

    /// Answers the host; true while it has the keyboard configured.
    pub fn poll(&mut self) -> bool {
        self.device.poll(&mut [&mut self.hid]);
        let configured = self.device.state() == UsbDeviceState::Configured;
        if !configured {
            self.sent = NO_KEYS;
        }
        configured
    }

    pub fn state(&self) -> UsbDeviceState {
        self.device.state()
    }

    /// Sends `report` unless the host already has it. Ok(true) when the host took a new report,
    /// Ok(false) when it has this one or the last is still in flight.
    pub fn send(&mut self, report: Report) -> Result<bool, UsbError> {
        if report == self.sent {
            return Ok(false);
        }
        let (modifier, key) = report;
        let keys = KeyboardReport { modifier, reserved: 0, leds: 0, keycodes: [key, 0, 0, 0, 0, 0] };
        match self.hid.push_input(&keys) {
            Ok(_) => {
                self.sent = report;
                Ok(true)
            }
            Err(UsbError::WouldBlock) => Ok(false),
            Err(error) => Err(error),
        }
    }
}
