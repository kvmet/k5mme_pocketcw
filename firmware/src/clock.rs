use at32f4xx_pac::at32f403a as pac;

/// The vendor's speed. Keying runs on a 1 ms timer, so a faster clock buys only screen speed
/// and costs battery.
pub const SYSCLK_HZ: u32 = 72_000_000;
/// The 8 MHz crystal or HICK, divided by 2.
const PLL_INPUT_HZ: u32 = 4_000_000;
/// APB1 and APB2 run at SYSCLK / 2: the bus limit is 120 MHz.
pub const APB_HZ: u32 = SYSCLK_HZ / 2;
/// Timers on a divided APB bus run at twice the bus clock.
pub const TIMER_HZ: u32 = APB_HZ * 2;

const SCLK_HICK: u8 = 0b00;
const SCLK_PLL: u8 = 0b10;
const APB_DIV_2: u8 = 0b100;
// The register value for multipliers 17..64 is the multiplier minus 1.
const PLL_MULT: u8 = (SYSCLK_HZ / PLL_INPUT_HZ - 1) as u8;
const _: () = assert!(PLL_MULT >= 16 && PLL_MULT <= 63);
const AUTO_STEP_ON: u8 = 0b11;
const HEXT_DIV_2: u8 = 0b00;

/// SCLK while `init` waits: HICK, which runs at 8 MHz into SCLK.
const HICK_HZ: u32 = 8_000_000;
/// How long the crystal and the PLL get to start. The vendor firmware allows the crystal 100 ms.
const START_TIMEOUT_US: u32 = 100_000;
const POLL_US: u32 = 100;

/// What feeds the PLL. The crystal is 8 MHz, the same input as HICK, so both give SYSCLK_HZ.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Crystal,
    Internal,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Crystal => "XTAL",
            Source::Internal => "HICK",
        }
    }
}

/// Runs the system from the PLL, whatever clock the bootloader left running: from the crystal
/// when it starts, else from HICK. If neither PLL start works the system stays on HICK at 8 MHz,
/// and `running` reports None.
pub fn init(crm: &pac::CRM) {
    crm.ctrl().modify(|_, w| w.hicken().set_bit());
    while crm.ctrl().read().hickstbl().bit_is_clear() {}
    // The PLL can only be reprogrammed while it is off, so run from HICK meanwhile.
    crm.cfg().modify(|_, w| unsafe { w.sclksel().bits(SCLK_HICK) });
    while crm.cfg().read().sclksts().bits() != SCLK_HICK {}
    // HICK reaches SCLK at 8 MHz only with this clear; the start timeouts count cycles at 8 MHz.
    crm.misc3().modify(|_, w| w.hick_to_sclk().clear_bit());
    crm.cfg().modify(|_, w| unsafe { w.ahbdiv().bits(0).apb1div().bits(APB_DIV_2).apb2div().bits(APB_DIV_2) });

    let started = (start_crystal(crm) && start_pll(crm, Source::Crystal)) || {
        crm.ctrl().modify(|_, w| w.pllen().clear_bit().hexten().clear_bit());
        start_pll(crm, Source::Internal)
    };
    if !started {
        crm.ctrl().modify(|_, w| w.pllen().clear_bit());
        return;
    }
    // Auto step ramps the bus clock up; the Artery library uses it for a switch above 108 MHz.
    crm.misc3().modify(|_, w| unsafe { w.auto_step_en().bits(AUTO_STEP_ON) });
    crm.cfg().modify(|_, w| unsafe { w.sclksel().bits(SCLK_PLL) });
    while crm.cfg().read().sclksts().bits() != SCLK_PLL {}
    crm.misc3().modify(|_, w| unsafe { w.auto_step_en().bits(0) });
}

/// What feeds the PLL that runs the system, read from the registers. None when the system is
/// not on the PLL.
pub fn running(crm: &pac::CRM) -> Option<Source> {
    let cfg = crm.cfg().read();
    (cfg.sclksts().bits() == SCLK_PLL).then(|| if cfg.pllrcs().bit_is_set() { Source::Crystal } else { Source::Internal })
}

fn start_crystal(crm: &pac::CRM) -> bool {
    // Bypass can only change while the crystal oscillator is off.
    if crm.ctrl().read().hexten().bit_is_clear() {
        crm.ctrl().modify(|_, w| w.hextbyps().clear_bit());
    }
    crm.ctrl().modify(|_, w| w.hexten().set_bit());
    wait_on_hick(|| crm.ctrl().read().hextstbl().bit_is_set())
}

/// Sets the PLL to SYSCLK_HZ from `source`, divided by 2 to PLL_INPUT_HZ, and waits for it to
/// lock.
fn start_pll(crm: &pac::CRM, source: Source) -> bool {
    crm.ctrl().modify(|_, w| w.pllen().clear_bit());
    match source {
        Source::Crystal => {
            crm.misc3().modify(|_, w| unsafe { w.hextdiv().bits(HEXT_DIV_2) });
            crm.cfg().modify(|_, w| w.pllrcs().set_bit().pllhextdiv().set_bit());
        }
        Source::Internal => {
            // The Artery library sets hickdiv whenever HICK feeds the PLL.
            crm.misc1().modify(|_, w| w.hickdiv().set_bit());
            crm.cfg().modify(|_, w| w.pllrcs().clear_bit());
        }
    }
    crm.cfg().modify(|_, w| unsafe {
        w.pllmult3_0().bits(PLL_MULT & 0x0F);
        w.pllmult5_4().bits(PLL_MULT >> 4);
        w.pllrange().bit(SYSCLK_HZ > 72_000_000)
    });
    crm.ctrl().modify(|_, w| w.pllen().set_bit());
    wait_on_hick(|| crm.ctrl().read().pllstbl().bit_is_set())
}

/// Polls `ready` for up to START_TIMEOUT_US. Counts cycles at HICK_HZ, so it is only for `init`.
fn wait_on_hick(ready: impl Fn() -> bool) -> bool {
    for _ in 0..START_TIMEOUT_US / POLL_US {
        if ready() {
            return true;
        }
        cortex_m::asm::delay(POLL_US * (HICK_HZ / 1_000_000));
    }
    ready()
}

/// Busy-waits at least `us` microseconds.
pub fn delay_us(us: u32) {
    cortex_m::asm::delay(us * (SYSCLK_HZ / 1_000_000));
}

pub fn delay_ms(ms: u32) {
    delay_us(ms * 1000);
}
