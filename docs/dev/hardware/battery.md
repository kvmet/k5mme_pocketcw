# Battery

## The reading

PA1 is ADC1 channel 1: the battery through a 2:1 divider, read against the 3.3 V supply. PocketCW also reads the internal 1.2 V reference on channel 17, which gives the supply voltage the ADC measures against. Each reading averages 8 samples at the longest sample time.

## Vendor behavior

V5.03.13 shows battery volts as counts times 0.0016169, for a 2500 mAh cell, and never reads a charge status. Confirmed.

| Counts | Volts | Vendor icon |
| - | - | - |
| 2037 | 3.29 | 1 segment |
| 2345 | 3.79 | 2 segments |
| 2400 | 3.88 | 3 segments |
| 2470 | 3.99 | 4 segments |
| below 1768 | 2.86 | Shuts down |

## PocketCW gauge

`logic/src/battery.rs` uses the vendor scale, but divider and regulator tolerances move the scale by a few percent on each unit. So the gauge learns each unit's reading at full charge and scales to it.

| Rule | Value |
| - | - |
| Full cell | 4.2 V, about 2597 counts at the vendor scale. |
| Learned full point | Within about 3 % of the design value. It takes a reading higher than the old full point, once 120 s have passed with no USB power, so charger surface charge has gone. Saved at 0x780. |
| USB detect | At startup, a reading 75 counts or more above the full point. Later, a jump of 60 counts or more up (USB in) or down (USB out) in one poll. With USB in, PA1 reads about 4.5 % above a full cell. |
| Percent | From a curve for a lithium cell under light load: 0 % at 3.4 V, 50 % near 3.82 V, 100 % at 4.2 V. |
| Low | Below 3.4 V, off USB. The status line says `BATTERY LOW: PLUG IN USB`, and the icon shows a red outline. |
| Empty | Below 3.2 V, or a supply below 3.1 V, for 10 polls in a row. The status line says `BATTERY EMPTY: POWERING OFF`, and 2 s later the device saves and powers off. |

The UI polls the battery once a second.

When the regulator drops out, the supply falls with the battery, and a reading against the supply stays steady. A supply reading under 98 % of the highest seen marks a dropout. The gauge then scales the battery reading by the supply ratio. The ratio of two reference readings cancels the reference's own error.

None of the low and empty behavior has been checked on a board with a draining battery.
