#!/usr/bin/env python3
"""Drive the device over the USB serial port: press buttons, key and play text, read what it
decodes and plays, read and change settings, save screenshots, and export stats and timing.

usage: python3 -I tools/pocketcw.py [--port PORT] STEP...

Steps run in order:
  F1 F2 F3 F4 LEFT RIGHT CLICK   press and release that button
  shot FILE.png                  save the screen
  wait MS                        pause, for screens that change on their own
  send TEXT                      key TEXT and wait until it is sent
  expect REGEX                   wait up to 60 s for an event line that matches
  listen MS                      show event lines for MS
  shots DIR                      save every screen into DIR; start on KEYER
  info                           show the firmware version, screen and panel
  get NAME                       show a setting or memory; get all shows every one
  set NAME VALUE                 change a setting or memory, as the screen writes it
  play TEXT                      sound TEXT as a receive drill does and wait until it ends
  stop                           stop what send and play started
  stats letters FILE.csv         save each letter's tries, accuracy and mastery
  stats progress FILE.csv        save each practice session
  edges MS FILE.csv              save each key and paddle line change for MS, with the keyer
                                 settings, and report lost lines

Event lines from the device show as they arrive, after "< ".

examples:
  python3 -I tools/pocketcw.py F4 RIGHT CLICK shot timing.png
  python3 -I tools/pocketcw.py send "CQ CQ DE K5MME K" listen 5000
  python3 -I tools/pocketcw.py set SPEED 25 set MODE "IAMBIC A" get all

USB OUT must not be OFF, so the device is on USB. PORT defaults to the only
/dev/cu.usbmodem* (macOS) or /dev/ttyACM* (Linux) port.

As a module, Device does the same from Python:
  d = Device(); d.press('F4'); d.send('CQ'); d.shot('cq.png'); d.close()
"""
import collections
import csv
import glob
import os
import re
import select
import struct
import sys
import termios
import time
import tty
import zlib

BUTTONS = ('F1', 'F2', 'F3', 'F4', 'LEFT', 'RIGHT', 'CLICK')
# Lines the device sends unasked; anything else before a screenshot header is left over from an
# unfinished screenshot. EDGE ON also sends VALUE lines unasked.
EVENTS = ('KEY', 'HEARD', 'PLAY', 'PLAYED', 'SENT', 'BUSY', 'EDGE', 'VALUE')
# Lines that answer a command.
REPLIES = ('INFO', 'VALUE', 'OK', 'ERROR', 'LEVEL', 'LETTER', 'MIXUP', 'SESSION', 'END')
# The longest SEND or PLAY text, from MAX_TEXT in logic/src/player.rs.
MAX_SEND = 48
SHOT_TIMEOUT_S = 3
REPLY_TIMEOUT_S = 3
EXPECT_TIMEOUT_S = 60


class Event:
    """One event line: KEY A is name KEY and text A. A word gap is KEY and one space."""

    def __init__(self, line):
        self.line = line
        self.name, _, self.text = line.partition(' ')

    def __repr__(self):
        return self.line


class Device:
    def __init__(self, port=None, echo=False):
        self.fd = os.open(port or find_port(), os.O_RDWR | os.O_NOCTTY)
        tty.setraw(self.fd)
        termios.tcflush(self.fd, termios.TCIOFLUSH)
        self.echo = echo
        self.data = b''
        self.events = collections.deque()

    def close(self):
        os.close(self.fd)

    def press(self, *buttons):
        for b in buttons:
            if b not in BUTTONS:
                raise ValueError(f'no button {b}')
            self._write(f'PRESS {b}\n')

    def shot(self, path=None):
        """The screen as (width, height, 8-bit RGB bytes); saved as a PNG when `path` is given."""
        self._write('SHOT\n')
        deadline = time.monotonic() + SHOT_TIMEOUT_S
        while True:
            line = self._line(deadline)
            if line is None:
                raise TimeoutError(f'no screenshot in {SHOT_TIMEOUT_S} s')
            if line.startswith('SHOT '):
                break
            self._event(line)
        header = line.split()
        if len(header) != 4 or header[3] != 'RGB666':
            raise RuntimeError(f'the device answered: {line}')
        width, height = int(header[1]), int(header[2])
        size = width * height * 3
        while len(self.data) < size:
            if not self._read(deadline):
                raise TimeoutError(f'screenshot stopped at {len(self.data)} of {size} bytes')
        # Each channel sits in the top 6 bits of its byte; the top bits repeat below to fill 8.
        rgb = bytes(b & 0xFC | b >> 6 for b in self.data[:size])
        self.data = self.data[size:]
        if path:
            with open(path, 'wb') as f:
                f.write(png(width, height, rgb))
        return width, height, rgb

    def send(self, text, timeout=EXPECT_TIMEOUT_S):
        """Keys `text` at the keyer speed and waits until it is sent. Returns what the device
        decoded meanwhile."""
        if len(text) > MAX_SEND:
            raise ValueError(f'send takes {MAX_SEND} characters at most, not {len(text)}')
        self.drain()
        self._write(f'SEND {text}\n')
        done = self.expect('SENT|BUSY', timeout)
        if done.name == 'BUSY':
            raise RuntimeError('the device is still sending')
        return copy(self.taken)

    def play(self, text, timeout=EXPECT_TIMEOUT_S):
        """Sounds `text` at CHAR SPEED, FARNSWORTH and TONE, and waits until it ends."""
        if len(text) > MAX_SEND:
            raise ValueError(f'play takes {MAX_SEND} characters at most, not {len(text)}')
        self.drain()
        self._write(f'PLAY {text}\n')
        self.expect('PLAYED$', timeout)

    def stop(self):
        self._write('STOP\n')

    def info(self):
        """{'VERSION': '0.2.0', 'SCREEN': 'KEYER', 'PANEL': 'IPS'}"""
        line = self._ask('INFO', 'INFO ')[-1]
        return dict(field.split('=', 1) for field in line.split()[1:])

    def get(self, name=None):
        """The value of setting or memory `name`, or of every one as a dict."""
        if name:
            return value(self._ask(f'GET {name}', 'VALUE ')[-1])[1]
        return dict(value(line) for line in self._ask('GET', 'END$')[:-1])

    def set(self, name, new):
        """Sets `name` to `new`, written as the screen writes it; the value the device kept."""
        return value(self._ask(f'SET {name} {new}', 'OK ')[-1])[1]

    def stats(self, kind):
        """The lines of STATS LETTERS or STATS PROGRESS, without END."""
        return self._ask(f'STATS {kind.upper()}', 'END$')[:-1]

    def edges(self, ms):
        """The rows within `ms`: (line, down, ms, seq) for each key and paddle line change, and
        (name, value, None, None) for each keyer setting the device sent, in the order they came."""
        self.drain()
        self._write('EDGE ON\n')
        events = self.listen(ms)
        self._write('EDGE OFF\n')
        rows = []
        for e in events:
            if e.name == 'EDGE':
                line, down, at_ms, seq = e.text.split()
                rows.append((line, int(down), int(at_ms), int(seq)))
            elif e.name == 'VALUE':
                name, _, new = e.text.partition(' ')
                rows.append((name, new, None, None))
        return rows

    def expect(self, pattern, timeout=EXPECT_TIMEOUT_S):
        """The first event line that matches `pattern` from its start. The lines before it are
        dropped, and kept in `taken`."""
        deadline = time.monotonic() + timeout
        self.taken = []
        while True:
            while self.events:
                event = self.events.popleft()
                if re.match(pattern, event.line):
                    return event
                self.taken.append(event)
            line = self._line(deadline)
            if line is None:
                raise TimeoutError(f'no event matched {pattern!r} in {timeout} s')
            self._event(line)

    def listen(self, ms):
        """The event lines that arrive within `ms`, after those already waiting."""
        deadline = time.monotonic() + ms / 1000
        while (line := self._line(deadline)) is not None:
            self._event(line)
        return self.drain()

    def drain(self):
        """The event lines waiting now, which are then dropped."""
        while (line := self._line(time.monotonic())) is not None:
            self._event(line)
        events = list(self.events)
        self.events.clear()
        return events

    def _ask(self, command, last):
        """Sends `command`; its reply lines up to the one that matches `last`. Event lines
        between them are kept as events."""
        self._write(command + '\n')
        deadline = time.monotonic() + REPLY_TIMEOUT_S
        lines = []
        while True:
            line = self._line(deadline)
            if line is None:
                raise TimeoutError(f'no answer to {command} in {REPLY_TIMEOUT_S} s')
            name = line.partition(' ')[0]
            if name == 'ERROR':
                raise RuntimeError(f'{command}: {line}')
            if name not in REPLIES:
                self._event(line)
                continue
            lines.append(line)
            if re.match(last, line):
                return lines

    def _write(self, line):
        os.write(self.fd, line.encode('ascii'))

    def _read(self, deadline):
        wait = max(0, deadline - time.monotonic())
        if not select.select([self.fd], [], [], wait)[0]:
            return False
        self.data += os.read(self.fd, 65536)
        return True

    def _line(self, deadline):
        """The next line, without its line end, or None at the deadline."""
        while (end := self.data.find(b'\n')) < 0:
            if not self._read(deadline):
                return None
        line, self.data = self.data[:end], self.data[end + 1:]
        return line.decode('ascii', 'replace')

    def _event(self, line):
        event = Event(line)
        if event.name not in EVENTS:
            return
        if self.echo:
            print('<', line, flush=True)
        self.events.append(event)


def copy(events):
    """The text that KEY lines spell, one space for each word gap."""
    return ''.join(e.text for e in events if e.name == 'KEY').strip()


def value(line):
    """(name, value) from VALUE SPEED 25 or OK MODE IAMBIC A."""
    _, name, new = (line + ' ').split(' ', 2)
    return name, new[:-1]


def save_letters(lines, path):
    """LETTER lines to CSV; LEVEL says how many Koch characters are in play. Shows the mixups."""
    level = next(int(line.split()[1]) for line in lines if line.startswith('LEVEL '))
    letters = [line.split()[1:] for line in lines if line.startswith('LETTER ')]
    with open(path, 'w', newline='') as f:
        out = csv.writer(f)
        out.writerow(['char', 'tries', 'percent', 'mastery', 'in_play'])
        for i, (char, tries, percent, mastery) in enumerate(letters):
            out.writerow([char, tries, percent, mastery, int(i < level)])
    for line in lines:
        if line.startswith('MIXUP '):
            _, sent, copied, count = line.split()
            print(f'mixup: sent {sent}, copied {copied}, {count} times')


def save_progress(lines, path):
    with open(path, 'w', newline='') as f:
        out = csv.writer(f)
        out.writerow(['seq', 'active_s', 'contacts', 'rx_chars', 'rx_right', 'sent_chars'])
        out.writerows(line.split()[1:] for line in lines)


def find_port():
    ports = glob.glob('/dev/cu.usbmodem*') + glob.glob('/dev/ttyACM*')
    if len(ports) != 1:
        sys.exit(f'found {len(ports)} serial ports {ports}; name one with --port')
    return ports[0]


def png(width, height, rgb):
    stride = width * 3
    rows = b''.join(b'\x00' + rgb[y * stride:(y + 1) * stride] for y in range(height))

    def chunk(kind, body):
        return struct.pack('>I', len(body)) + kind + body + struct.pack('>I', zlib.crc32(kind + body))

    return (b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(rows, 9))
            + chunk(b'IEND', b''))


# Menu rows from the top. RADIO shows only with the decoder board on, so it is left out.
MENU = ['KEYER', 'TIMING', 'RECEIVE', 'POTA', 'SOTA', 'HUNT', 'LETTERS', 'PROGRESS', 'SETTINGS', 'SYSTEM']
SETTINGS_TABS = ['keyer', 'sound', 'recv', 'qso', 'theme', 'call', 'mem']
# F2 on a drill opens the next one, in this order. The first three are answered; the ragchews
# run until one over ends.
ANSWERED = ['receive-koch', 'receive-words', 'receive-calls']
RAGCHEWS = ['ragchew', 'ragchew-quiz']
OVER_S = 45
# Long enough for a QSO to send its first text.
QSO_S = 4
# After the answer: the copy ends three word gaps after it, and the score shows.
SCORED_S = 2


class Screens:
    """Saves every screen into `out_dir`, from KEYER back to KEYER."""

    def __init__(self, device, out_dir):
        self.d = device
        self.out_dir = out_dir
        # The menu keeps its selection between visits; None until it is known.
        self.row = None

    def shot(self, name):
        path = os.path.join(self.out_dir, name + '.png')
        self.d.shot(path)
        print(path, flush=True)

    def top_of_menu(self):
        self.d.press(*['LEFT'] * (len(MENU) - 1))
        self.row = 0

    def open(self, name):
        """From the menu: opens row `name`."""
        target = MENU.index(name)
        if self.row is None:
            self.top_of_menu()
        move = target - self.row
        self.d.drain()
        self.d.press(*(['RIGHT'] * move if move > 0 else ['LEFT'] * -move), 'CLICK')
        self.row = target

    def answer_group(self):
        """Keys back the next group the drill sends, once it has played."""
        group = self.d.expect('PLAY ').text
        self.d.expect('PLAYED$')
        self.d.send(group)
        time.sleep(SCORED_S)

    def run(self):
        d = self.d
        self.shot('keyer')
        d.press('F2')
        self.shot('keyer-mem')
        d.press('CLICK', 'F4')
        self.top_of_menu()
        self.shot('menu')

        self.open('TIMING')
        self.shot('timing')
        # TIMING goes back to KEYER, and KEYER to the menu.
        d.press('F4', 'F4')

        self.open('RECEIVE')
        for i, name in enumerate(ANSWERED):
            if i:
                d.drain()
                d.press('F2')
            self.answer_group()
            self.shot(name)
        for name in RAGCHEWS:
            d.press('F2')
            time.sleep(OVER_S)
            self.shot(name)
        d.press('F4')

        for name in ['POTA', 'SOTA', 'HUNT']:
            self.open(name)
            time.sleep(QSO_S)
            self.shot(name.lower())
            d.press('F4')

        for name in ['LETTERS', 'PROGRESS', 'SYSTEM']:
            self.open(name)
            self.shot(name.lower())
            d.press('F4')

        self.open('SETTINGS')
        for i, tab in enumerate(SETTINGS_TABS):
            if i:
                d.press('F1')
            self.shot('settings-' + tab)
        d.press('F4', 'F4')


# How many values each step takes.
STEP_VALUES = {
    'shot': 1, 'wait': 1, 'send': 1, 'expect': 1, 'listen': 1, 'shots': 1,
    'info': 0, 'get': 1, 'set': 2, 'play': 1, 'stop': 0, 'stats': 2, 'edges': 2,
}


def steps(args):
    """(step, values) for each step, checked before anything is sent."""
    out = []
    while args:
        word = args.pop(0)
        if word in BUTTONS:
            out.append((word, []))
            continue
        if word not in STEP_VALUES or len(args) < STEP_VALUES[word]:
            sys.exit(f'unknown step {word}\n\n{__doc__}')
        values = [args.pop(0) for _ in range(STEP_VALUES[word])]
        if word in ('wait', 'listen', 'edges') and not values[0].isdigit():
            sys.exit(f'{word} takes milliseconds, not {values[0]}')
        if word in ('send', 'play') and len(values[0]) > MAX_SEND:
            sys.exit(f'{word} takes {MAX_SEND} characters at most, not {len(values[0])}')
        if word == 'stats' and values[0] not in ('letters', 'progress'):
            sys.exit(f'stats takes letters or progress, not {values[0]}')
        out.append((word, values))
    return out


def run(d, plan):
    for step, values in plan:
        value = values[0] if values else None
        if step in BUTTONS:
            d.press(step)
        elif step == 'shot':
            width, height, _ = d.shot(value)
            print(f'{value}: {width} x {height}')
        elif step == 'wait':
            time.sleep(int(value) / 1000)
        elif step == 'send':
            print('copied:', d.send(value))
        elif step == 'expect':
            d.expect(value)
        elif step == 'listen':
            d.listen(int(value))
        elif step == 'shots':
            os.makedirs(value, exist_ok=True)
            Screens(d, value).run()
        elif step == 'info':
            print(' '.join(f'{k}={v}' for k, v in d.info().items()))
        elif step == 'get' and value == 'all':
            for name, new in d.get().items():
                print(f'{name} {new}')
        elif step == 'get':
            print(f'{value} {d.get(value)}')
        elif step == 'set':
            print(f'{value} {d.set(value, values[1])}')
        elif step == 'play':
            d.play(value)
        elif step == 'stop':
            d.stop()
        elif step == 'stats':
            lines = d.stats(value)
            (save_letters if value == 'letters' else save_progress)(lines, values[1])
            print(values[1])
        else:
            rows = d.edges(int(value))
            with open(values[1], 'w', newline='') as f:
                out = csv.writer(f)
                out.writerow(['line', 'value', 'ms', 'seq'])
                out.writerows(rows)
            seqs = [row[3] for row in rows if row[3] is not None]
            lost = sum((b - a - 1) % 2**32 for a, b in zip(seqs, seqs[1:]))
            print(f'{values[1]}: {len(seqs)} changes, {lost} lost')


def main():
    args = sys.argv[1:]
    port = None
    if args[:1] == ['--port'] and len(args) > 1:
        port = args[1]
        args = args[2:]
    plan = steps(args)
    if not plan:
        sys.exit(__doc__)
    d = Device(port, echo=True)
    try:
        run(d, plan)
    except (TimeoutError, RuntimeError) as e:
        sys.exit(str(e))
    finally:
        d.close()


if __name__ == '__main__':
    main()
