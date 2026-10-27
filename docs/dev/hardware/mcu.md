# MCU and clock

## The chip

- Artery AT32F403A, 48 pins, 256 KB flash. The flash size register at 0x1FFFF7E0 reads 256. The debug IDCODE is 0x70050242.
- RAM is 96 KB for this part family. PocketCW uses 64 KB until the size is checked.
- The 96 bit unique ID at 0x1FFFF7E8 shows on the SYSTEM screen.

## Option bytes

On the 2.64 unit, the low bytes from 0x1FFFF800 up read `A5 FF F2 64 E0 FF FF FF FE FF 44 79`.

- FAP = 0xA5, so flash read protection is off. A running app or an SWD probe can read the bootloader.
- Data0 and Data1 hold the board version. See [Board variants](board-variants.md).
- WRP0 = 0xE0 write-protects the bootloader. See [Flash layout and bootloader](flash-layout.md).
- EOPB0 = 0xFE. It likely selects 224 KB of SRAM with 128 KB of zero wait flash, 0x08000000 to 0x08020000; this is not checked against Artery's documents. Code past 0x08020000 runs much slower: the sample interrupt measured 2661 cycles with its code there and 1092 with it inside the first 128 KB, on board 2.64. `firmware/memory.x` puts the `.fast_text` section right after the vector table for this.

PocketCW reads Data1 at 0x1FFFF806.

## Clock

The board has an 8 MHz crystal (HEXT).

- The vendor app runs the PLL from the crystal times 9, for 72 MHz (SystemClock_Config at 0x080185E0, HSE_VALUE 8 MHz at 0x08010E38). It hangs when the crystal does not start within 100 ms (0x0801861C). So every board that runs the vendor firmware has a working crystal.
- The vendor firmware is built with Keil MDK on the ST STM32F1 HAL, with the medium density startup file and soft float. Its timer prescalers assume 72 MHz.
- The clock source that the bootloader leaves running is not known.

PocketCW (`firmware/src/clock.rs`) also runs at 72 MHz.

1. It switches to the internal 8 MHz clock (HICK) while it sets up the PLL.
2. It starts the crystal, then runs the PLL from HEXT / 2 times 18. Each start has 100 ms.
3. If the crystal or the PLL does not start, it runs the PLL from HICK / 2 times 18.
4. If that fails too, it stays on HICK at 8 MHz.

APB1 and APB2 run at 36 MHz. Timers on these buses run at 72 MHz.

The CLOCK row of the SYSTEM screen reads the PLL source back from CFG.PLLRCS: XTAL, or HICK in red as a fault.

## Debug access

- The vendor app sets AFIO MAPR SWJ_CFG to 111 at 0x08011054. This likely turns SWD off while the vendor app runs, so a probe would need connect under reset.
- PocketCW sets SWJ_CFG to 010: JTAG off, SWD on. This frees PA15, PB3 and PB4. It also remaps SPI1 to PB3, PB4 and PB5.
