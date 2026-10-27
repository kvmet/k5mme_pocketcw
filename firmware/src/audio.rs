//! PA8 as a 1-bit DAC into the codec's input 1. TMR1 sends a 281 kHz pulse train, and at each
//! pulse DMA2 channel 1 loads the next duty from a buffer of two samples' pulses. The sample
//! interrupt, at each half of the buffer, refills the half just played from the synth, 17,578
//! times a second; the codec input and the headphones smooth the pulses into audio.

use core::sync::atomic::{AtomicU32, Ordering, compiler_fence};

use at32f4xx_pac::at32f403a as pac;
use cortex_m::peripheral::DWT;
use pocketcw_logic::pwm::Shaper;
use pocketcw_logic::synth::{Sound, Synth};

use crate::clock::{SYSCLK_HZ, TIMER_HZ};
use crate::gpio::{self, Port};
use crate::pins::SIDETONE as PIN;

/// Timer counts per pulse: the duty resolution.
const STEPS: u16 = 256;
/// Pulses per sample: half the buffer.
const PULSES: usize = 16;
pub const SAMPLE_HZ: f32 = (TIMER_HZ / STEPS as u32 / PULSES as u32) as f32;
/// CPU cycles in one sample: the sample interrupt's budget.
pub const SAMPLE_CYCLES: u32 = SYSCLK_HZ / SAMPLE_HZ as u32;
/// Half duty: the silent level. The pin stays here between sounds, so starting one doesn't pop.
const MID: u16 = STEPS / 2;
/// TMR1's overflow on the DMA's flexible request mapping, `DMA_FLEXIBLE_TMR1_OVERFLOW` in
/// Artery's AT32F403A firmware library.
const TMR1_OVERFLOW_REQUEST: u8 = 0x37;

pub type Buffer = [u16; 2 * PULSES];

/// The sample interrupt's longest run since `take_worst_cycles`, in CPU cycles. Needs the DWT
/// cycle counter on.
static WORST_CYCLES: AtomicU32 = AtomicU32::new(0);

/// The sample interrupt's longest run since the last call, in CPU cycles.
pub fn take_worst_cycles() -> u32 {
    WORST_CYCLES.swap(0, Ordering::Relaxed)
}

pub struct Audio {
    dma: pac::DMA2,
    /// The DMA reads one half while the sample interrupt fills the other.
    buffer: &'static mut Buffer,
    shaper: Shaper,
    /// Halves filled with silence in a row. Once both are, the DMA stops until `wake`.
    quiet_halves: u8,
    pub synth: Synth,
}

impl Audio {
    /// Needs the TMR1 and DMA2 clocks on.
    pub fn new(timer: pac::TMR1, dma: pac::DMA2, buffer: &'static mut Buffer, gpioa: &Port, sound: Sound) -> Self {
        buffer.fill(MID);
        unsafe {
            timer.div().write(|w| w.bits(0));
            timer.pr().write(|w| w.bits(STEPS - 1));
            timer.rpr().write(|w| w.bits(0));
            timer.c1dt().write(|w| w.bits(MID));
            // C1OCTRL = 110 (PWM mode A), C1OBEN = 1 (compare loads at the update event).
            timer.cm1_output().write(|w| w.bits(0b110 << 4 | 1 << 3));
            timer.brk().write(|w| w.oen().set_bit());
            dma.dma_src_sel1().write(|w| w.dma_flex_en().set_bit());
            dma.dma_src_sel0().write(|w| w.ch1_src().bits(TMR1_OVERFLOW_REQUEST));
            let channel = dma.channel1();
            channel.paddr().write(|w| w.bits(timer.c1dt().as_ptr() as u32));
            channel.maddr().write(|w| w.bits(buffer.as_ptr() as u32));
            channel.dtcnt().write(|w| w.bits(buffer.len() as u16));
        }
        timer.swevt().write(|w| w.ovfswtr().set_bit());
        timer.iden().write(|w| w.ovfden().set_bit());
        timer.cctrl().write(|w| w.c1en().set_bit());
        timer.ctrl1().write(|w| w.tmren().set_bit());
        gpio::configure(gpioa, PIN, gpio::PERIPHERAL_OUTPUT);
        Audio { dma, buffer, shaper: Shaper::new(STEPS), quiet_halves: 0, synth: Synth::new(SAMPLE_HZ, sound) }
    }

    /// Starts the DMA when the synth has something to play. Call after keying it or changing its
    /// sound.
    pub fn wake(&mut self) {
        if !self.synth.is_silent() && self.dma.channel1().ctrl().read().chen().bit_is_clear() {
            self.quiet_halves = 0;
            // Memory to TMR1's compare register, 16-bit both sides, around the buffer for good,
            // with an interrupt at each half.
            self.dma.channel1().ctrl().write(|w| {
                w.dtd().memory().mincm().set_bit().lm().set_bit().pwidth().bit16().mwidth().bit16();
                w.chpl().very_high().hdtien().set_bit().fdtien().set_bit().chen().set_bit()
            });
        }
    }

    /// The sample interrupt: fills the half of the buffer the DMA is not reading with the next
    /// sample, or with silence. Once the whole buffer is silent, the DMA stops.
    ///
    /// In `.fast_text` (memory.x), inside the first 128 KB of flash, which has no wait states;
    /// past it, the sample took 2661 of its 4096 cycles. The named section also keeps LLVM's
    /// outliner from moving parts of it to the end of `.text`.
    #[unsafe(link_section = ".fast_text.sample")]
    #[inline(never)]
    pub fn sample(&mut self) {
        let start = DWT::cycle_count();
        self.dma.clr().write(|w| w.gfc1().set_bit());
        let channel = self.dma.channel1();
        let (first, second) = self.buffer.split_at_mut(PULSES);
        let free = if channel.dtcnt().read().bits() as usize > PULSES { second } else { first };
        if self.synth.is_silent() {
            free.fill(MID);
            self.shaper = Shaper::new(STEPS);
            self.quiet_halves += 1;
            if self.quiet_halves >= 2 {
                channel.ctrl().write(|w| unsafe { w.bits(0) });
                // On the next start the DMA begins again at the buffer's start.
                channel.dtcnt().write(|w| unsafe { w.bits(2 * PULSES as u16) });
            }
        } else {
            self.quiet_halves = 0;
            self.shaper.fill(self.synth.sample(), free);
        }
        // The DMA reads the buffer behind the compiler's back.
        compiler_fence(Ordering::SeqCst);
        WORST_CYCLES.fetch_max(DWT::cycle_count().wrapping_sub(start), Ordering::Relaxed);
    }
}
