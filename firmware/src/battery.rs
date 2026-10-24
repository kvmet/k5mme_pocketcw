//! Battery and supply voltage from ADC1: the battery divider on PA1 and the internal 1.2 V
//! reference, which gives the supply voltage the ADC measures against.

use at32f4xx_pac::at32f403a as pac;

use crate::clock;

const BATTERY_CHANNEL: u8 = 1;
const REFERENCE_CHANNEL: u8 = 17;
/// Typical internal reference voltage from the AT32F403A datasheet.
const REFERENCE_MV: u32 = 1200;
const FULL_SCALE: u32 = 4095;
/// 239.5 ADC clocks: the battery divider has a high source impedance.
const LONGEST_SAMPLE: u8 = 0b111;
const SOFTWARE_TRIGGER: u8 = 0b111;
const SAMPLES: u32 = 8;

#[derive(Clone, Copy, Default)]
pub struct Reading {
    /// ADC counts at PA1, against the supply.
    pub raw: u16,
    pub supply_mv: u16,
}

pub struct Adc {
    adc: pac::ADC1,
}

/// Calibration or a conversion did not finish in time.
#[derive(Debug)]
pub struct Timeout;

/// Waits up to 1 ms for `done`: calibration takes about 5 us, a conversion 15 us.
fn wait(done: impl Fn() -> bool) -> Result<(), Timeout> {
    for _ in 0..1000 {
        if done() {
            return Ok(());
        }
        clock::delay_us(1);
    }
    Err(Timeout)
}

impl Adc {
    /// Takes ADC1, with its clock already on, and calibrates it.
    pub fn new(adc: pac::ADC1) -> Result<Adc, Timeout> {
        adc.spt2().modify(|_, w| unsafe { w.cspt1().bits(LONGEST_SAMPLE) });
        adc.spt1().modify(|_, w| unsafe { w.cspt17().bits(LONGEST_SAMPLE) });
        adc.osq1().modify(|_, w| unsafe { w.oclen().bits(0) });
        adc.ctrl2().modify(|_, w| unsafe { w.octesel_l().bits(SOFTWARE_TRIGGER).octesel_h().clear_bit().octen().set_bit().itsrven().set_bit().adcen().set_bit() });
        // The ADC needs two ADC clocks after power-on before calibration.
        clock::delay_us(10);
        adc.ctrl2().modify(|_, w| w.adcalinit().set_bit());
        wait(|| adc.ctrl2().read().adcalinit().bit_is_clear())?;
        adc.ctrl2().modify(|_, w| w.adcal().set_bit());
        wait(|| adc.ctrl2().read().adcal().bit_is_clear())?;
        Ok(Adc { adc })
    }

    pub fn read(&mut self) -> Result<Reading, Timeout> {
        let raw = self.average(BATTERY_CHANNEL)?;
        let reference = self.average(REFERENCE_CHANNEL)?.max(1);
        let supply_mv = REFERENCE_MV * FULL_SCALE / reference;
        Ok(Reading { raw: raw as u16, supply_mv: supply_mv as u16 })
    }

    fn average(&mut self, channel: u8) -> Result<u32, Timeout> {
        self.adc.osq3().modify(|_, w| unsafe { w.osn1().bits(channel) });
        let mut sum = 0;
        for _ in 0..SAMPLES {
            sum += self.convert()?;
        }
        Ok(sum / SAMPLES)
    }

    fn convert(&mut self) -> Result<u32, Timeout> {
        self.adc.ctrl2().modify(|_, w| w.ocswtrg().set_bit());
        wait(|| self.adc.sts().read().occe().bit_is_set())?;
        // Reading the data clears the end-of-conversion flag.
        Ok(self.adc.odt().read().odt().bits() as u32)
    }
}
