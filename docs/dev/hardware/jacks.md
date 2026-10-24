# Key and func jacks

## Key jack

The key jack, labeled "key", takes a paddle, a straight key or a sideswiper.

| Contact | Pin | Level |
| - | - | - |
| Tip | PB1 | Active low, internal pull-up. |
| Ring | PB0 | Active low, internal pull-up. |

- The vendor default treats PB0 as dit. On the paddle of the 2.64 unit, PB0 is dah and PB1 is dit, so that unit needed the vendor swap setting. Confirmed on 2.64.
- PocketCW with PADDLES NORMAL puts dits on the tip (PB1). STRAIGHT mode keys from either contact, so a sideswiper works. Confirmed in code.
- In the vendor keyboard mode, the same pins carry the PS/2 clock and data, with EXTI0 on PB0. Confirmed.

PocketCW rules for each contact, in `logic/src/keyer.rs`. Confirmed in code.

| Rule | Value | Why |
| - | - | - |
| Debounce | 8 ms | Contacts bounce for up to about 6 to 10 ms. 8 ms is under half the 20 ms dot at 60 WPM. |
| Closed at power on | Does not key until it opens. | A mono plug, or a key held at power on, does not key. |
| Ring closed 5 s | Stops keying until it opens. | A mono plug shorts the ring. 5 s is longer than the longest character, `0` at 5 WPM. |
| Tip closed 30 s | Stops keying until it opens. | Leaves time to hold a straight key down to tune a radio. |

## Func jack

The func jack, labeled "func", keys a radio or an external keyer.

| Contact | Pin | Level |
| - | - | - |
| Tip | PA2 | High = closed. |
| Ring | PA3 | High = closed. |

- The vendor drives PA2 on every board, and PA3 only on board 2.60. Confirmed.
- PocketCW drives both on every board. With OUT TYPE STRAIGHT, one line follows the keyer output. With OUT TYPE PADDLE, the two lines follow the dit and dah contacts. OUT LINES swaps tip and ring. KEY OUT OFF holds both open. Confirmed in code.
- A memory keys the func jack only with OUT TYPE STRAIGHT, because PADDLE output follows the contacts and not the keyer. Confirmed in code.

## LED

PB2 drives the output LED, active high. The vendor lights it during every sent element, and the bootloader blinks it. PocketCW lights it while the keyer output or a memory keys. Confirmed on 2.64.
