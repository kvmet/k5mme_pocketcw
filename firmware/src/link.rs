//! The Pro decoder board: its power and its link, USART1 at 115200 8N1. DMA1 channel 5 fills a
//! buffer and the idle line ends a frame, as in the vendor firmware. While off, the board is
//! unpowered and TX does not drive, so the board is not powered through its RX pin.
//!
//! DMA1 runs on its flexible request mapping. The fixed mapping also wires TMR1's overflow to
//! channel 5, and `audio` raises that request at every pulse.

use core::sync::atomic::{AtomicU16, Ordering, compiler_fence};

use at32f4xx_pac::at32f403a as pac;

use crate::clock::APB_HZ;
use crate::gpio::{self, port_a, port_b};
use crate::pins::{DECODER_POWER, DECODER_RX, DECODER_TX};

const BAUD: u32 = 115_200;
/// USART1 receive on the DMA's flexible request mapping, `DMA_FLEXIBLE_UART1_RX` in Artery's
/// AT32F403A firmware library.
const USART1_RX_REQUEST: u8 = 0x19;
/// The vendor's receive buffer size; a longer frame is cut here.
pub const FRAME_LEN: usize = 64;

/// Frames that arrived with a framing, noise or overrun error.
pub static ERRORS: AtomicU16 = AtomicU16::new(0);

pub struct Link {
    usart: pac::USART1,
    dma: pac::DMA1,
    buffer: &'static mut [u8; FRAME_LEN],
    on: bool,
}

impl Link {
    /// Needs the USART1 and DMA1 clocks on. Starts off.
    pub fn new(usart: pac::USART1, dma: pac::DMA1, buffer: &'static mut [u8; FRAME_LEN]) -> Self {
        // A pull-up keeps an idle line high when no board drives it.
        gpio::set_high(port_a(), DECODER_RX);
        gpio::configure(port_a(), DECODER_RX, gpio::INPUT_PULL);
        dma.dma_src_sel1().write(|w| unsafe { w.ch5_src().bits(USART1_RX_REQUEST).dma_flex_en().set_bit() });
        let mut link = Link { usart, dma, buffer, on: true };
        link.set_on(false);
        link
    }

    pub fn set_on(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        self.on = on;
        if on {
            gpio::set_high(port_b(), DECODER_POWER);
            gpio::configure(port_a(), DECODER_TX, gpio::PERIPHERAL_OUTPUT);
            self.usart.baudr().write(|w| unsafe { w.bits(((APB_HZ + BAUD / 2) / BAUD) as u16) });
            self.usart.ctrl3().write(|w| w.dmaren().set_bit());
            self.restart();
            self.usart.ctrl1().write(|w| w.uen().set_bit().ten().set_bit().ren().set_bit().idleien().set_bit());
        } else {
            self.usart.ctrl1().write(|w| unsafe { w.bits(0) });
            self.dma.channel5().ctrl().write(|w| unsafe { w.bits(0) });
            gpio::set_low(port_a(), DECODER_TX);
            gpio::configure(port_a(), DECODER_TX, gpio::INPUT_PULL);
            gpio::set_low(port_b(), DECODER_POWER);
        }
    }

    /// Points DMA at the start of the buffer for the next frame.
    fn restart(&mut self) {
        let channel = self.dma.channel5();
        channel.ctrl().write(|w| unsafe { w.bits(0) });
        channel.paddr().write(|w| unsafe { w.bits(self.usart.dt().as_ptr() as u32) });
        channel.maddr().write(|w| unsafe { w.bits(self.buffer.as_ptr() as u32) });
        channel.dtcnt().write(|w| unsafe { w.bits(FRAME_LEN as u16) });
        // Peripheral to memory, 8-bit both sides, memory address steps.
        channel.ctrl().write(|w| w.mincm().set_bit().chen().set_bit());
    }

    /// At the USART1 interrupt: runs `read` on the frame that the idle line ended, then
    /// receives the next. None when the interrupt was not an idle line.
    pub fn take_frame<R>(&mut self, read: impl FnOnce(&[u8]) -> R) -> Option<R> {
        let sts = self.usart.sts().read();
        // Reading the status and then the data register clears the idle and error flags.
        let _ = self.usart.dt().read();
        if !sts.idlef().bit_is_set() {
            return None;
        }
        if sts.ferr().bit_is_set() || sts.nerr().bit_is_set() || sts.roerr().bit_is_set() {
            ERRORS.fetch_add(1, Ordering::Relaxed);
        }
        let received = FRAME_LEN - self.dma.channel5().dtcnt().read().bits() as usize;
        self.dma.channel5().ctrl().write(|w| unsafe { w.bits(0) });
        // The DMA wrote the buffer behind the compiler's back.
        compiler_fence(Ordering::SeqCst);
        let result = read(&self.buffer[..received]);
        self.restart();
        Some(result)
    }

    /// Blocks until the last byte is in the transmit buffer: about 87 us a byte.
    pub fn send(&mut self, bytes: &[u8]) {
        if !self.on {
            return;
        }
        for &b in bytes {
            while self.usart.sts().read().tdbe().bit_is_clear() {}
            self.usart.dt().write(|w| unsafe { w.bits(b as u16) });
        }
    }
}
