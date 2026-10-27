# Power

## Power on

A dial click (PA0) powers the board up. PC13 is the power hold, high = stay on. Unless the app drives PC13 high, power drops about 1 s later, with the power-on reset flag set and the backup registers cleared. Confirmed on 2.64.

The vendor app sets PC13 high first at boot. PocketCW does the same in `power::hold`, right after the GPIO clocks start.

## Power off

The vendor long press, over 1.5 s on PA0, does this in order:

1. Stops the backlight PWM and drives PB2 low.
2. Saves the settings.
3. Drives PC13 low.
4. Sets the speaker volume and drives PB12 low.
5. Enters standby: PWR clock on, standby flag cleared, PA0 wakeup pin on, wakeup flag cleared, PDDS and SLEEPDEEP set, WFI.

PocketCW (`power::standby`) waits until the dial click reads released for 8 ms, because the wakeup pin wakes the chip at once while PA0 is high. Then it drives the backlight, the speaker amplifier, the LED, both func jack lines and PC13 low, and enters standby the same way. Before WFI it disables and clears every interrupt in the NVIC, because an enabled pending interrupt ends WFI even with interrupts masked. If standby is refused, the chip resets.

Before standby, PocketCW saves the settings, the memories, the letter statistics and the current practice session. A failed save shows on the status line for 3 s.

## Wake

A dial click wakes the chip through reset. The bootloader starts the app again, and the app must drive PC13 high to stay on. The standby flag always reads clear when the app starts, so the app cannot tell a wake from standby from a cold start.

## Fault screen

On a panic, a hard fault or an unexpected interrupt, `firmware/src/fault.rs`:

1. Drives both func jack lines and the LED low, so a radio does not stay keyed.
2. Stops the PA8 audio output.
3. Shows the fault on a dark red screen: the return addresses of a panic (see [Panic screen](../building.md#panic-screen)), the PC, LR, CFSR, HFSR and BFAR values of a hard fault, or the number of an unexpected interrupt.
4. Waits, with interrupts off, for a dial click, then enters standby.
