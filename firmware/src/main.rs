//! CW keyer firmware for the HamCube CW Pokemon, boards 2.60 and later.
#![no_std]
#![no_main]

use core::fmt::Write;
use core::ptr::read_volatile;

use cortex_m_rt::{ExceptionFrame, exception};
use pocketcw_firmware::fault;
use rtic_monotonics::systick::prelude::*;

systick_monotonic!(Mono, 1_000);

const TAGLINE: &str = "BKG.club #388";

#[exception]
unsafe fn HardFault(frame: &ExceptionFrame) -> ! {
    // Configurable and hard fault status, then the bus fault address.
    let [cfsr, hfsr, bfar] = [0xE000_ED28u32, 0xE000_ED2C, 0xE000_ED38].map(|a| unsafe { read_volatile(a as *const u32) });
    fault::halt("hard fault", |t| {
        let _ = write!(t, "pc {:08X} lr {:08X} cfsr {cfsr:08X} hfsr {hfsr:08X} bfar {bfar:08X}", frame.pc(), frame.lr());
    })
}

#[exception]
unsafe fn DefaultHandler(irq: i16) {
    fault::halt("unexpected interrupt", |t| {
        let _ = write!(t, "irq {irq}");
    })
}

#[rtic::app(device = at32f4xx_pac::at32f403a, dispatchers = [USART2, USART3])]
mod app {
    use core::fmt::Write;
    use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};

    use at32f4xx_pac::at32f403a as pac;
    use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
    use pocketcw_firmware::{
        audio::Audio,
        backlight::Backlight,
        battery::{self, Adc},
        boot::{Boot, Check, LOG_LINES},
        clock,
        codec::Codec,
        gpio::{self, port_a, port_b, port_c},
        i2c::I2c,
        lcd::Lcd,
        link::{self, Link},
        pins::*,
        power,
        store::{self, Saved},
        theme::{Theme, palette_stripes},
        text::Text,
        ui::{Command, PanelFrom, SystemInfo, Ui},
    };
    use pocketcw_logic::{
        battery::Gauge,
        buttons::{self, Button, Buttons, Event},
        decoder::{Decoded, KeyDecoder},
        input::{Action, Input},
        panel::{self, Panel},
        board,
        keyer::{Contacts, Jack, Keyer, Mode, Output, output_lines},
        listen::Listener,
        player::Player,
        settings::Settings,
        synth::{PLAYERS, SIDETONE},
        volume::wm8978_speaker,
        ticks,
        timing::Line,
    };
    use rtic_monotonics::systick::prelude::*;
    use rtic::Mutex;
    use rtic_sync::channel::{Receiver, Sender};

    use super::{Mono, TAGLINE};

    const EVENT_QUEUE: usize = 64;

    /// What the tick task sends the UI.
    pub enum Message {
        Button(Event),
        Decoded(Decoded),
        /// The keyer output or a paddle contact changed, at `at_ms` on the tick count. `dit_ms` is
        /// the dot length decoding uses then.
        Edge { line: Line, down: bool, at_ms: u32, dit_ms: u16 },
        /// Every player has finished its text; `play` is the newest play started by then.
        Played { play: u32 },
        /// A frame from the Pro decoder board.
        Board(board::Event),
        /// A character decoded from the decoder board's tone line, heard at `wpm`.
        Heard { decoded: Decoded, wpm: u16 },
    }
    const SAVE_DELAY_MS: u32 = 2000;
    const BOOT_FRAME_MS: u32 = 10;

    const TICK_HZ: u32 = 1_000;
    const TIMER_COUNT_HZ: u32 = 1_000_000;

    static TICKS: AtomicU32 = AtomicU32::new(0);
    /// Messages lost because the queue was full; shown on screen when nonzero.
    static DROPPED_EVENTS: AtomicU16 = AtomicU16::new(0);
    static KEY_OUT: AtomicBool = AtomicBool::new(true);
    /// The radio jack takes paddle contacts instead of one keyed line; and tip and ring swapped.
    static OUT_PADDLE: AtomicBool = AtomicBool::new(false);
    static OUT_SWAP: AtomicBool = AtomicBool::new(false);
    /// Player 0 is sending a memory: it keys the radio, and touching the key stops it.
    static PLAYER_KEYS: AtomicBool = AtomicBool::new(false);
    /// Counts plays started, so a Played message sent before a new play can't end it.
    static PLAYS_STARTED: AtomicU32 = AtomicU32::new(0);
    /// Sidetone pitch for your own keying; the player sends at its own pitch.
    static USER_TONE: AtomicU16 = AtomicU16::new(700);
    /// The RADIO screen is open: decode the decoder board's tone line.
    static LISTENING: AtomicBool = AtomicBool::new(false);

    #[shared]
    struct Shared {
        keyer: Keyer,
        audio: Audio,
        /// Each sounds on its own voice, so stations can overlap.
        players: [Player; PLAYERS],
        /// Receive runs in its interrupt; the UI sends commands and switches it on and off.
        link: Link,
    }

    #[local]
    struct Local {
        timer: pac::TMR3,
        buttons: Buttons,
        decoder: KeyDecoder,
        listener: Listener,
        jack: Jack,
        messages: Sender<'static, Message, EVENT_QUEUE>,
        board_messages: Sender<'static, Message, EVENT_QUEUE>,
    }

    const FLASH_SIZE_ADDRESS: u32 = 0x1FFF_F7E0;
    const UNIQUE_ID_ADDRESS: u32 = 0x1FFF_F7E8;
    /// User system data byte Data1; its complement follows it.
    const OPTION_DATA1_ADDRESS: u32 = 0x1FFF_F806;
    const POLL_MS: u32 = 1000;
    /// How long the empty battery notice shows before power off.
    const EMPTY_NOTICE_MS: u32 = 2000;
    /// How long a failed save's status line shows before power off.
    const FAILED_SAVE_NOTICE_MS: u32 = 3000;

    /// Devices only the UI task uses.
    pub struct Devices {
        /// None when the ADC failed to calibrate.
        adc: Option<Adc>,
        backlight: Backlight,
        codec: Option<Codec>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let dp = cx.device;
        dp.CRM.apb2en().modify(|_, w| {
            w.iomux().set_bit().gpioa().set_bit().gpiob().set_bit().gpioc().set_bit();
            w.spi1().set_bit().tmr1().set_bit().adc1().set_bit().usart1().set_bit()
        });
        dp.CRM.ahben().modify(|_, w| w.dma1().set_bit());
        // The GPIO clocks must run first: writes to a port with its clock off are ignored.
        power::hold();
        dp.CRM.apb1en().modify(|_, w| w.pwc().set_bit().tmr3().set_bit().tmr4().set_bit());
        clock::init(&dp.CRM);
        let clock_source = clock::running(&dp.CRM);
        Mono::start(cx.core.SYST, clock::SYSCLK_HZ);
        // JTAG off and SWD on frees PA15, PB3 and PB4. SPI1 moves to PB3/PB4/PB5.
        dp.IOMUX.remap().modify(|_, w| unsafe { w.swjtag_mux().bits(0b010).spi1_mux0().set_bit() });

        let (a, b, c) = (port_a(), port_b(), port_c());
        // Off until the first clear: the panel shows random pixels before it.
        gpio::set_low(b, BACKLIGHT);
        gpio::configure(b, BACKLIGHT, gpio::OUTPUT);
        // PB15 and the decoder board power are low before the LCD starts, as in the vendor firmware.
        for (port, pin) in [(b, SPEAKER_AMP), (b, LED), (a, KEY_OUT_TIP), (a, KEY_OUT_RING), (b, DECODER_POWER), (b, PB15)] {
            gpio::set_low(port, pin);
            gpio::configure(port, pin, gpio::OUTPUT);
        }
        gpio::configure(a, BATTERY, gpio::ANALOG);
        for pin in [TIP, RING] {
            gpio::set_high(b, pin);
            gpio::configure(b, pin, gpio::INPUT_PULL);
        }
        gpio::set_low(b, DECODER_TONE);
        gpio::configure(b, DECODER_TONE, gpio::INPUT_PULL);
        for pin in F_KEYS.into_iter().chain([POWER_BUTTON]) {
            gpio::set_low(a, pin);
            gpio::configure(a, pin, gpio::INPUT_PULL);
        }
        for pin in [DIAL_LEFT, DIAL_RIGHT] {
            gpio::set_low(c, pin);
            gpio::configure(c, pin, gpio::INPUT_PULL);
        }

        // Settings load first: the panel type can be saved there.
        let mut bus = I2c::new();
        let (saved, eeprom_ok) = store::load(&mut bus);
        let settings = saved.settings;
        let board = unsafe { core::ptr::read_volatile(OPTION_DATA1_ADDRESS as *const u8) };
        let mut lcd = Lcd::new(dp.SPI1);
        lcd.reset();
        let panel_id = lcd.read_id();
        let (panel, panel_from) = choose_panel(&mut bus, saved.panel, panel::detect(&panel_id), board);
        lcd.init(panel);
        let _ = lcd.clear(Rgb565::BLACK);

        let codec = Codec::detect(&mut bus);
        let codec_ok = codec.is_some_and(|c| c.init(&mut bus, settings.volume));
        gpio::set(b, SPEAKER_AMP, settings.volume > 0);
        store_output(&settings);
        USER_TONE.store(settings.tone_hz, Ordering::Relaxed);

        let audio = Audio::new(dp.TMR1, a, settings.sound());
        let keyer = Keyer::new(settings.mode, settings.swap, settings.wpm);

        let timer = dp.TMR3;
        unsafe {
            timer.div().write(|w| w.bits((clock::TIMER_HZ / TIMER_COUNT_HZ - 1) as u16));
            timer.pr().write(|w| w.bits((TIMER_COUNT_HZ / TICK_HZ - 1) as u16));
        }
        timer.iden().write(|w| w.ovfien().set_bit());
        timer.ctrl1().write(|w| w.tmren().set_bit());

        let (messages, receiver) = rtic_sync::make_channel!(Message, EVENT_QUEUE);
        let board_messages = messages.clone();
        let Some(frame_buffer) = cortex_m::singleton!(: [u8; link::FRAME_LEN] = [0; link::FRAME_LEN]) else {
            unreachable!("init runs once")
        };
        let mut link = Link::new(dp.USART1, dp.DMA1, frame_buffer);
        link.set_on(settings.decoder_board);
        let flash_kb = unsafe { core::ptr::read_volatile(FLASH_SIZE_ADDRESS as *const u16) };
        let uid = [0, 4, 8].map(|offset| unsafe { core::ptr::read_volatile((UNIQUE_ID_ADDRESS + offset) as *const u32) });
        let adc = Adc::new(dp.ADC1).ok();
        let system = SystemInfo { flash_kb, board, panel, panel_from, panel_id, uid, clock: clock_source, codec, codec_ok, eeprom_ok, adc_ok: adc.is_some() };
        let devices = Devices { adc, backlight: Backlight::new(dp.TMR4, settings.brightness), codec };
        if ui::spawn(lcd, bus, saved, receiver, system, devices).is_err() {
            unreachable!("init runs once, so the UI task is not running yet")
        }

        let jack = Jack::new(jack_contacts(gpio::inputs(b)));
        let local = Local {
            timer,
            buttons: Buttons::default(),
            decoder: KeyDecoder::new(keyer.dit_ms()),
            listener: Listener::default(),
            jack,
            messages,
            board_messages,
        };
        (Shared { keyer, audio, players: [Player::new(); PLAYERS], link }, local)
    }

    /// The key jack's contacts from port B's input bits: a closed contact pulls its pin low.
    fn jack_contacts(port_b_bits: u16) -> Contacts {
        Contacts { tip: port_b_bits & (1 << TIP) == 0, ring: port_b_bits & (1 << RING) == 0 }
    }

    /// Holding F2 this long at power-on flips the panel type.
    const PANEL_FLIP_MS: u32 = 50;

    /// The saved panel type, else the detected one, else the board's. F2 held at power-on shows
    /// the other type and saves that choice; the flip still applies this boot when the save fails.
    fn choose_panel(bus: &mut I2c, saved: Option<Panel>, detected: Option<Panel>, board: u8) -> (Panel, PanelFrom) {
        let (shown, source) = panel::choose(saved, detected, board);
        let f2_held = (0..PANEL_FLIP_MS).all(|_| {
            clock::delay_ms(1);
            gpio::is_high(port_a(), F_KEYS[1])
        });
        if !f2_held {
            return (shown, PanelFrom::Chosen(source));
        }
        let flipped = shown.other();
        let (auto, auto_source) = panel::choose(None, detected, board);
        let choice = panel::saved_choice(flipped, auto);
        let from = if !store::write(bus, store::PANEL_ADDRESS, &panel::encode(choice)) {
            PanelFrom::Unsaved
        } else if choice.is_some() {
            PanelFrom::Chosen(panel::Source::Saved)
        } else {
            PanelFrom::Chosen(auto_source)
        };
        (flipped, from)
    }

    /// Sleeps between interrupts; any interrupt wakes the core.
    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    /// The next audio sample, while anything sounds. Above the tick, so drawing and keying never
    /// make the audio stutter.
    #[task(binds = TMR1_OVF_TMR10, priority = 4, shared = [audio])]
    fn sample(mut cx: sample::Context) {
        cx.shared.audio.lock(Audio::sample);
    }

    /// A frame from the decoder board ended. Replies at once, as the vendor does.
    #[task(binds = USART1, priority = 2, shared = [link], local = [board_messages])]
    fn board_frame(mut cx: board_frame::Context) {
        let event = cx.shared.link.lock(|link| {
            let event = link.take_frame(board::parse)?;
            if let Some(reply) = event.reply() {
                link.send(reply);
            }
            Some(event)
        });
        if let Some(event) = event
            && cx.local.board_messages.try_send(Message::Board(event)).is_err()
        {
            DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Every millisecond: paddle keyer, keyed outputs, the players and the voices they key, the
    /// decoders, then button debounce.
    #[task(
        binds = TMR3,
        priority = 3,
        shared = [keyer, audio, players],
        local = [
            timer, buttons, decoder, listener, jack, messages,
            listening: bool = false,
            key_down: bool = false, keyed: bool = false, key_out: (bool, bool) = (false, false), contacts: Contacts = Contacts { tip: false, ring: false },
            playing: bool = false,
        ],
    )]
    fn tick(mut cx: tick::Context) {
        cx.local.timer.ists().write(|w| unsafe { w.bits(0) });
        let now = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        let (a, b, c) = (gpio::inputs(port_a()), gpio::inputs(port_b()), gpio::inputs(port_c()));
        let high = |bits: u16, pin: u32| bits & (1 << pin) != 0;

        cx.local.jack.tick(jack_contacts(b));
        let (contacts, keying) = (cx.local.jack.closed(), cx.local.jack.keying());
        let (down, dit_ms, mode, swap) = cx.shared.keyer.lock(|keyer| (keyer.tick(keying), keyer.dit_ms(), keyer.mode(), keyer.swap));
        let copy_dit_ms = cx.local.decoder.dit_ms();

        let messages = cx.local.messages;
        let mut send = |message| {
            if messages.try_send(message).is_err() {
                DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
            }
        };

        let old = *cx.local.contacts;
        for (line, was, is) in [(Line::Tip, old.tip, contacts.tip), (Line::Ring, old.ring, contacts.ring)] {
            if was != is {
                send(Message::Edge { line, down: is, at_ms: now, dit_ms: copy_dit_ms });
            }
        }
        *cx.local.contacts = contacts;
        if down != *cx.local.key_down {
            *cx.local.key_down = down;
            send(Message::Edge { line: Line::Key, down, at_ms: now, dit_ms: copy_dit_ms });
        }

        let mut played = [None; PLAYERS];
        let mut pitches = [0; PLAYERS];
        cx.shared.players.lock(|players| {
            if (keying.tip || keying.ring) && PLAYER_KEYS.load(Ordering::Relaxed) {
                PLAYER_KEYS.store(false, Ordering::Relaxed);
                players[0].stop();
            }
            for (i, player) in players.iter_mut().enumerate() {
                (played[i], pitches[i]) = (player.tick(), player.pitch_hz());
            }
        });
        let keyed = down || (played[0] == Some(true) && PLAYER_KEYS.load(Ordering::Relaxed));
        if keyed != *cx.local.keyed {
            *cx.local.keyed = keyed;
            gpio::set(port_b(), LED, keyed);
        }
        let output = if OUT_PADDLE.load(Ordering::Relaxed) { Output::Paddle } else { Output::Straight };
        let (dit, dah) = keying.paddles(mode.dit_on_ring(swap));
        let lines = output_lines(output, OUT_SWAP.load(Ordering::Relaxed), keyed, dit, dah);
        let on = KEY_OUT.load(Ordering::Relaxed);
        let key_out = (lines.0 && on, lines.1 && on);
        if key_out != *cx.local.key_out {
            *cx.local.key_out = key_out;
            gpio::set(port_a(), KEY_OUT_TIP, key_out.0);
            gpio::set(port_a(), KEY_OUT_RING, key_out.1);
        }
        let playing = played.iter().any(Option::is_some);
        if *cx.local.playing && !playing {
            send(Message::Played { play: PLAYS_STARTED.load(Ordering::Relaxed) });
        }
        *cx.local.playing = playing;
        cx.shared.audio.lock(|audio| {
            audio.synth.key(SIDETONE, down, USER_TONE.load(Ordering::Relaxed));
            for (i, (&key, &hz)) in played.iter().zip(&pitches).enumerate() {
                audio.synth.key(SIDETONE + 1 + i, key == Some(true), hz);
            }
            audio.wake();
        });
        if let Some(decoded) = cx.local.decoder.tick(down, dit_ms, mode == Mode::Straight) {
            send(Message::Decoded(decoded));
        }
        let listening = LISTENING.load(Ordering::Relaxed);
        if listening && !core::mem::replace(cx.local.listening, listening) {
            *cx.local.listener = Listener::default();
        }
        *cx.local.listening = listening;
        if listening && let Some(decoded) = cx.local.listener.tick(high(b, DECODER_TONE)) {
            send(Message::Heard { decoded, wpm: cx.local.listener.wpm() });
        }

        let mut raw = [false; buttons::COUNT];
        for (button, pin) in [Button::F1, Button::F2, Button::F3, Button::F4].into_iter().zip(F_KEYS) {
            raw[button as usize] = high(a, pin);
        }
        raw[Button::Click as usize] = high(a, POWER_BUTTON);
        raw[Button::Left as usize] = high(c, DIAL_LEFT);
        raw[Button::Right as usize] = high(c, DIAL_RIGHT);
        cx.local.buttons.tick(raw, |event| send(Message::Button(event)));
    }

    #[task(priority = 1, shared = [keyer, audio, players, link])]
    async fn ui(
        mut cx: ui::Context,
        mut lcd: Lcd,
        mut bus: I2c,
        saved: Saved,
        mut messages: Receiver<'static, Message, EVENT_QUEUE>,
        system: SystemInfo,
        mut devices: Devices,
    ) {
        let Saved { mut settings, letters, history, memories, battery_full, .. } = saved;
        let checks = boot_checks(&system, &settings);
        let theme = Theme::new(settings.style, settings.palette, settings.boot_count);
        let stripes = palette_stripes(settings.palette, settings.boot_count);
        let tagline = Text::format(format_args!("v{} {} {TAGLINE}", env!("CARGO_PKG_VERSION"), settings.callsign()));
        let mut boot = Boot::new(settings.boot_count, &theme, stripes, settings.callsign(), tagline, checks);
        let start = Mono::now();
        loop {
            let ms = (Mono::now() - start).to_millis();
            if !boot.draw(&mut lcd, ms) {
                Mono::delay(400u32.millis()).await;
                break;
            }
            // Every message is taken each frame, so none are lost while the queue fills. The click
            // that woke the device may still be down, so only the other buttons skip.
            let mut skip = false;
            while let Ok(message) = messages.try_recv() {
                skip |= matches!(message, Message::Button(Event::Press(button)) if button != Button::Click);
            }
            if skip {
                break;
            }
            Mono::delay(BOOT_FRAME_MS.millis()).await;
        }
        while messages.try_recv().is_ok() {}

        let gauge = Gauge::new(battery_full, TICKS.load(Ordering::Relaxed));
        let mut ui = Ui::new(&settings, system, letters, history, memories, gauge);
        ui.poll(&mut lcd, read_battery(&mut devices.adc), TICKS.load(Ordering::Relaxed));
        let mut poll_at = Mono::now() + POLL_MS.millis();
        ui.draw(&mut lcd, &settings);
        let mut input = Input::new(gpio::is_high(port_a(), POWER_BUTTON));
        let mut save_at: Option<<Mono as Monotonic>::Instant> = None;
        let mut dropped_shown = 0;
        loop {
            let ui_at = ui.deadline().map(|at| Mono::now() + ticks::ms_until(TICKS.load(Ordering::Relaxed), at).millis());
            let wake_at = [save_at, ui_at, Some(poll_at)].into_iter().flatten().min();
            let received = match wake_at {
                Some(at) => Mono::timeout_at(at, messages.recv()).await,
                None => Ok(messages.recv().await),
            };
            let before = settings;
            match received {
                Ok(Ok(message)) => handle(&mut ui, &mut lcd, &mut settings, &mut input, message, save_at.is_some(), &mut bus),
                Ok(Err(_)) => unreachable!("the tick task holds the sender forever"),
                Err(_) => {
                    if save_at.is_some_and(|at| at <= Mono::now()) {
                        save(&ui, &mut lcd, &mut bus, &settings);
                        save_at = None;
                    }
                    if poll_at <= Mono::now() {
                        ui.poll(&mut lcd, read_battery(&mut devices.adc), TICKS.load(Ordering::Relaxed));
                        poll_at = Mono::now() + POLL_MS.millis();
                        save_battery(&mut ui, &mut lcd, &mut bus);
                        if ui.battery_empty() {
                            ui.status(&mut lcd, "BATTERY EMPTY: POWERING OFF");
                            Mono::delay(EMPTY_NOTICE_MS.millis()).await;
                            power_off(&mut ui, &mut lcd, &mut bus, &settings, save_at.is_some());
                        }
                    }
                    ui.timeout(&mut lcd, &settings, TICKS.load(Ordering::Relaxed));
                }
            }
            if settings != before {
                save_at = Some(Mono::now() + SAVE_DELAY_MS.millis());
                apply(&mut cx, &mut ui, &mut lcd, &mut bus, &mut devices, &before, &settings);
            }
            save_letters(&mut ui, &mut lcd, &mut bus, false);
            save_history(&mut ui, &mut lcd, &mut bus, false);
            save_memories(&mut ui, &mut lcd, &mut bus, false);
            // One lock for all of them, so the tick never sees the gap between a stop and the play
            // after it and reports that play as finished.
            cx.shared.players.lock(|players| {
                while let Some(command) = ui.take_command(&settings) {
                    match command {
                        Command::Play(p) => {
                            // Only player 0 keys the radio; anything else takes the first free player.
                            let i = if p.transmit { 0 } else { players.iter().position(|p| !p.is_active()).unwrap_or(0) };
                            if i == 0 {
                                PLAYER_KEYS.store(p.transmit, Ordering::Relaxed);
                            }
                            players[i].start(p.text.as_str(), p.keying, p.seed, p.delay_ms, p.pitch_hz);
                            PLAYS_STARTED.fetch_add(1, Ordering::Relaxed);
                        }
                        Command::Stop => {
                            PLAYER_KEYS.store(false, Ordering::Relaxed);
                            players.iter_mut().for_each(Player::stop);
                        }
                    }
                }
            });
            if let Some(command) = ui.take_board_command() {
                cx.shared.link.lock(|link| link.send(command.as_str().as_bytes()));
            }
            LISTENING.store(ui.listening(), Ordering::Relaxed);
            let band = ui.band();
            cx.shared.audio.lock(|audio| {
                audio.synth.set_band(band);
                audio.wake();
            });
            let dropped = DROPPED_EVENTS.load(Ordering::Relaxed);
            if dropped != dropped_shown {
                dropped_shown = dropped;
                let line = Text::<32>::format(format_args!("{dropped} INPUTS LOST"));
                ui.status(&mut lcd, line.as_str());
            }
        }
    }

    /// Passes one message from the tick task to the UI. `unsaved`: settings changed since the
    /// last save, so powering off saves first.
    fn handle(ui: &mut Ui, lcd: &mut Lcd, settings: &mut Settings, input: &mut Input, message: Message, unsaved: bool, bus: &mut I2c) {
        match message {
            Message::Button(event) => match input.map(event) {
                None => {}
                Some(Action::PowerHold) => ui.status(lcd, "RELEASE TO POWER OFF"),
                Some(Action::PowerOff) => power_off(ui, lcd, bus, settings, unsaved),
                Some(action) => ui.act(lcd, settings, action),
            },
            Message::Decoded(decoded) => ui.decoded(lcd, settings, decoded),
            Message::Edge { line, down, at_ms, dit_ms } => ui.edge(lcd, settings, line, down, at_ms, dit_ms),
            // A play started after the tick sent this is still sounding.
            Message::Played { play } if play == PLAYS_STARTED.load(Ordering::Relaxed) => {
                ui.played(lcd, settings, TICKS.load(Ordering::Relaxed))
            }
            Message::Played { .. } => {}
            Message::Board(event) => ui.board_event(lcd, event),
            Message::Heard { decoded, wpm } => ui.heard(lcd, decoded, wpm),
        }
    }

    /// Saves everything unsaved and enters standby. `unsaved`: settings changed since the last save.
    fn power_off(ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c, settings: &Settings, unsaved: bool) -> ! {
        let saved = [
            !unsaved || save(ui, lcd, bus, settings),
            save_letters(ui, lcd, bus, true),
            save_history(ui, lcd, bus, true),
            save_memories(ui, lcd, bus, true),
        ];
        if saved.contains(&false) {
            clock::delay_ms(FAILED_SAVE_NOTICE_MS);
        }
        lcd.sleep();
        power::standby();
    }

    /// Sends changed settings to the keyer, sound, codec and key output.
    fn apply(cx: &mut ui::Context, ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c, devices: &mut Devices, before: &Settings, after: &Settings) {
        if after.brightness != before.brightness {
            devices.backlight.set(after.brightness);
        }
        if (after.mode, after.swap, after.wpm) != (before.mode, before.swap, before.wpm) {
            cx.shared.keyer.lock(|keyer| {
                keyer.set_mode(after.mode);
                keyer.swap = after.swap;
                keyer.set_wpm(after.wpm);
            });
        }
        if after.volume != before.volume {
            // Off at volume 0, so the pulse train on the codec input can't reach the speaker.
            gpio::set(port_b(), SPEAKER_AMP, after.volume > 0);
            if !devices.codec.is_some_and(|c| c.set_volume(bus, after.volume)) {
                ui.status(lcd, "CODEC DID NOT ANSWER");
            }
        }
        if after.decoder_board != before.decoder_board {
            cx.shared.link.lock(|link| link.set_on(after.decoder_board));
            ui.board_powered(after.decoder_board);
        }
        if after.volume != before.volume && ui.board_present() {
            // The board's volume scale matches the WM8978 speaker register, 0 to 63.
            let level = wm8978_speaker(after.volume).unwrap_or(0);
            cx.shared.link.lock(|link| link.send(board::volume(level).as_str().as_bytes()));
        }
        if after.sound() != before.sound() {
            cx.shared.audio.lock(|audio| {
                audio.synth.set_sound(after.sound());
                audio.wake();
            });
        }
        store_output(after);
        USER_TONE.store(after.tone_hz, Ordering::Relaxed);
    }

    fn store_output(settings: &Settings) {
        KEY_OUT.store(settings.key_out, Ordering::Relaxed);
        OUT_PADDLE.store(settings.output == Output::Paddle, Ordering::Relaxed);
        OUT_SWAP.store(settings.output_swap, Ordering::Relaxed);
    }

    fn boot_checks(startup: &SystemInfo, settings: &Settings) -> [Check; LOG_LINES] {
        let mut lines: [Text<24>; LOG_LINES] = core::array::from_fn(|_| Text::new());
        let _ = write!(lines[0], "mcu   at32f403a {}k", startup.flash_kb);
        let source = startup.clock.map_or("fail", clock::Source::name);
        let _ = write!(lines[1], "clock pll {} MHz {source}", clock::SYSCLK_HZ / 1_000_000);
        let _ = write!(lines[2], "codec {}", startup.codec.map_or("none", Codec::name));
        let _ = write!(lines[3], "eeprom 24c16");
        let mode = match settings.mode {
            Mode::IambicA => "iambic a",
            Mode::IambicB => "iambic b",
            Mode::Straight => "straight",
        };
        let _ = write!(lines[4], "keyer {mode} {}wpm", settings.wpm);
        let _ = write!(lines[5], "adc   battery");
        let results = [
            startup.flash_kb == 256,
            startup.clock == Some(clock::Source::Crystal),
            startup.codec_ok,
            startup.eeprom_ok,
            TICKS.load(Ordering::Relaxed) > 0,
            startup.adc_ok,
        ];
        let mut lines = lines.into_iter();
        results.map(|ok| Check { text: lines.next().unwrap_or_default(), ok })
    }

    /// None when the ADC failed to calibrate or a conversion timed out.
    fn read_battery(adc: &mut Option<Adc>) -> Option<battery::Reading> {
        adc.as_mut()?.read().ok()
    }

    fn save_letters(ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c, now: bool) -> bool {
        ui.take_letters(now).is_none_or(|record| save_record(ui, lcd, bus, store::LETTERS_ADDRESS, &record, "LETTER STATS NOT SAVED"))
    }

    fn save_battery(ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c) -> bool {
        ui.take_battery().is_none_or(|record| save_record(ui, lcd, bus, store::BATTERY_ADDRESS, &record, "BATTERY CALIBRATION NOT SAVED"))
    }

    fn save_memories(ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c, now: bool) -> bool {
        ui.take_memories(now).is_none_or(|record| save_record(ui, lcd, bus, store::MEMORIES_ADDRESS, &record, "MEMORIES NOT SAVED"))
    }

    fn save_history(ui: &mut Ui, lcd: &mut Lcd, bus: &mut I2c, now: bool) -> bool {
        ui.take_history(TICKS.load(Ordering::Relaxed), now).is_none_or(|(slot, bytes)| {
            save_record(ui, lcd, bus, store::history_slot_address(slot), &bytes, "PROGRESS NOT SAVED")
        })
    }

    fn save(ui: &Ui, lcd: &mut Lcd, bus: &mut I2c, settings: &Settings) -> bool {
        report(ui, lcd, store::write_settings(bus, settings), "SETTINGS NOT SAVED")
    }

    /// Writes `record`, or shows `failure` on the status line.
    fn save_record(ui: &Ui, lcd: &mut Lcd, bus: &mut I2c, address: u16, record: &[u8], failure: &str) -> bool {
        report(ui, lcd, store::write(bus, address, record), failure)
    }

    /// Shows `failure` on the status line unless `saved`.
    fn report(ui: &Ui, lcd: &mut Lcd, saved: bool, failure: &str) -> bool {
        if !saved {
            ui.status(lcd, failure);
        }
        saved
    }
}
