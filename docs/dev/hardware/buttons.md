# Buttons

The device has four buttons, F1 to F4, and a dial that turns and clicks. See [Pins](pins.md) for the pins.

| Input | Pin | Level |
| - | - | - |
| F1 to F4 | PA4 to PA7 | Active high, internal pull-down. |
| Dial left, dial right | PC14, PC15 | Active high, internal pull-down. |
| Dial click | PA0 | Active high. Also the standby wakeup pin. |

The dial click also closes the dial left contact: PC14 reads high while PA0 is pressed.

## PocketCW timing

`logic/src/buttons.rs` reads every input once per millisecond, in the `tick` task.

| Rule | Value |
| - | - |
| Debounce | 8 ms. |
| Press event | Sent at the press, without waiting for release. |
| Dial repeat | While the dial stays turned: a repeat after 400 ms, then every 80 ms. |
| Dial click hold | 1000 ms; the vendor uses 1500 ms. Release after the hold turns the device off. A button press or dial turn before the release cancels it. |
| Dial left after a click | Ignored while the click is down and for 50 ms after it opens. |

The click that wakes the device is still down when the app starts. `logic/src/input.rs` ignores that click until it is released.

The F buttons also send a hold event after 500 ms. `Input` turns an F press into `SoftDown`, the hold into `SoftHold`, and the release into `Soft`, the short action. The UI skips the release of a key whose press or hold already acted. A press that `Input` did not map, such as one that skipped the boot screen or cancelled a power off, maps nothing at its hold or release.

## Panel override at power on

F2 held at power on, for the first 50 ms that the app reads it, picks the other screen type. See [Display](display.md).
