# Keys and jacks

The device has two jacks for Morse. The key jack, labeled "key", takes your paddle or key. The func jack, labeled "func", keys a radio or an external keyer.

The USB port can also send your keying to a computer, as typed text or for VBand. See [USB keyboard](usb.md).

## Key types

Set the key type with MODE in the KEYER section of the settings. See [Settings](settings.md).

| MODE | Key | How it keys |
| - | - | - |
| IAMBIC A | Dual-lever paddle | One lever sends dots, the other dashes. Squeeze both levers and the keyer alternates dots and dashes. When you let go of both levers, the keyer stops after the element it is sending. |
| IAMBIC B | Dual-lever paddle | The same as IAMBIC A, but when you let go of both levers after a squeeze, the keyer still sends the one element the squeeze asked for. This is the default. |
| STRAIGHT | Straight key | The key closes the tip contact, and you time every element yourself. |
| STRAIGHT | Sideswiper | Either side closes a contact, and either contact keys. You time every element yourself. |

In both iambic modes, the keyer remembers a lever you press while it sends an element of the other kind, and sends that element next.

In the iambic modes, PADDLES sets which lever sends dots: NORMAL puts dots on the tip contact, and REVERSED puts dots on the ring contact. STRAIGHT mode ignores PADDLES.

### How the device reads your sending

- In the iambic modes, the keyer times every element at your set speed, so the decoder reads your sending at that speed.
- In STRAIGHT mode, the decoder starts at your set speed and then follows the speed you actually send.

## The key jack

The key jack has two contacts, tip and ring.

- A contact that is closed when you turn the device on does not key until it opens.
- A ring contact closed for 5 seconds stops keying until it opens.
- A tip contact closed for 30 seconds stops keying until it opens. This leaves time to hold a straight key down to tune a radio.

When a contact stops keying, the status line shows a message such as `DAH STUCK ON: CHECK PLUG`. In STRAIGHT mode the message names TIP or RNG (ring) instead of DIT or DAH.

A mono plug connects the ring contact all the time. Plug a mono plug in before you turn the device on: then the ring never keys, and the tip keys normally. If you plug a mono plug in while the device is on, the ring keys for 5 seconds before it stops.

## The func jack

The func jack keys a radio. It follows these settings in the KEYER section:

| Setting | Value | What the func jack does |
| - | - | - |
| KEY OUT | ON | Your key keys the func jack on the KEYER, TIMING and RADIO screens. The header of these screens shows `TX`. This is the default. |
| KEY OUT | OFF | The func jack never closes. |
| OUT TYPE | STRAIGHT | One line closes while the keyer sends, for a radio's straight key input. OUT LINES picks TIP KEYS or RING KEYS. |
| OUT TYPE | PADDLE | Tip and ring follow your dot contact and your dash contact, for the radio's own keyer. OUT LINES picks DIT ON TIP or DIT ON RING. |

F3 (OUT) on the KEYER screen turns KEY OUT on and off.

Only your own keying, your [memories](memories.md) and a logging program over the [serial port](usb.md) key the func jack. The practice screens and the settings never let your key key it, so a drill stays off the air. The stations of the practice screens never key it. A logging program keys it on every screen while KEY OUT is ON. Memories key the func jack only when KEY OUT is ON and OUT TYPE is STRAIGHT.

The LED lights while the keyer sends and while a memory sends.
