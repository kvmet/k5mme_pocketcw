#!/usr/bin/env python3
"""Plays each clip of the content pack as the device plays it, to mark it good or bad, and fixes
a bad one with a respelling or phonemes. Marks go to review.txt and fixes to fixes.txt in the
voice's folder. A fixed clip goes to the pack's cache, so the next pack build takes it as it is.

Good and Bad mark the clip as it is now. With the list focused: G good, B bad, N note (Enter in the note marks bad), Space plays again,
Up and Down move. Enter in the respelling or phonemes speaks the fix, Keep fix saves it and
marks the clip good, and Reset puts back the word list's said. Esc goes back to the list.

    tools/pack/.venv/bin/python tools/pack/audition.py
"""

import argparse
import audioop
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PySide6.QtCore import QAbstractTableModel, QBuffer, QByteArray, QIODevice, QModelIndex, Qt, QTimer
from PySide6.QtGui import QFont, QKeySequence, QRegularExpressionValidator, QShortcut
from PySide6.QtMultimedia import QAudioFormat, QAudioSink, QMediaDevices, QtAudio
from PySide6.QtWidgets import (
    QAbstractItemView,
    QApplication,
    QComboBox,
    QFormLayout,
    QGridLayout,
    QGroupBox,
    QHBoxLayout,
    QHeaderView,
    QLabel,
    QLineEdit,
    QMainWindow,
    QPushButton,
    QTableView,
    QVBoxLayout,
    QWidget,
)

import pack
from voiceclips import OUTPUT_HZ, resample, upsample

PLAY_HZ = 48000
# Holding Up or Down plays only the clip the list stops on.
PLAY_DELAY_MS = 150
FILTERS = ("To review", "Bad", "All")
COLUMNS = ("Text", "Clip", "Mark", "Says", "Note")
# Keys under the phonemes field: the misaki phonemes a keyboard lacks or that differ from their
# letter, each with a word it sounds in. Kokoro has no g; it takes ɡ.
PHONEME_KEYS = {
    "Stress and pitch, before the vowel": [("ˈ", "stress"), ("ˌ", "light stress"), ("↗", "pitch up"), ("↘", "pitch down"), ("ː", "long"), ("…", "pause")],
    "Vowels": [
        ("i", "see"), ("ɪ", "sit"), ("A", "day"), ("ɛ", "bed"), ("æ", "cat"), ("ɑ", "father"),
        ("ɔ", "law"), ("O", "go"), ("ʊ", "book"), ("u", "you"), ("ʌ", "cup"), ("ə", "about"),
        ("ɜ", "her"), ("I", "my"), ("W", "how"), ("Y", "boy"), ("ᵊ", "button"), ("ᵻ", "roses"),
    ],
    "Consonants": [
        ("ɡ", "get"), ("ŋ", "sing"), ("ɹ", "red"), ("j", "yes"), ("ʃ", "she"), ("ʒ", "vision"),
        ("ʧ", "chin"), ("ʤ", "jam"), ("θ", "thin"), ("ð", "this"), ("T", "butter"),
    ],
}
KEY_COLUMNS = 6


@dataclass
class Row:
    text: str
    kind: str
    clip: str
    # The word list's said, or its meaning for a meaning clip.
    base: str
    said: str = ""
    spoken: str = ""
    phonemes: str = ""
    speech_key: str = ""


class Review:
    """The clips, their marks and the voice's fixes, kept in step with review.txt and fixes.txt."""

    def __init__(self, voice: pack.Voice):
        self.voice = voice
        self.fixes = pack.read_fixes(voice.folder)
        self.marks = pack.read_review(voice.folder)
        # Stops on a fix for a word in no list.
        pack.read_words(self.fixes)
        self.rows = [
            Row(text, kind, clip, base)
            for text, said, meaning, kind in pack.read_words()
            for clip, base in pack.clips_of(said, meaning, kind)
        ]
        for row in self.rows:
            self.update(row)

    def update(self, row: Row):
        row.said = self.fixes.get((row.text, row.clip), row.base)
        row.spoken = pack.spoken(row.clip, row.said)
        row.phonemes = self.voice.phonemes(row.spoken)
        row.speech_key = self.voice.speech_key(row.phonemes)

    def state(self, row: Row) -> str:
        """good, bad, stale or unmarked."""
        mark = self.marks.get((row.text, row.clip))
        if mark is None:
            return "unmarked"
        return mark[0] if mark[1] == row.speech_key else "stale"

    def note(self, row: Row) -> str:
        return self.marks.get((row.text, row.clip), ("", "", ""))[2]

    def mark(self, row: Row, mark: str, note: str):
        self.marks[row.text, row.clip] = (mark, row.speech_key, note)
        ordered = {(r.text, r.clip): self.marks[r.text, r.clip] for r in self.rows if (r.text, r.clip) in self.marks}
        # Marks for words in no list stay; pack.py counts them.
        ordered |= {clip: mark for clip, mark in self.marks.items() if clip not in ordered}
        pack.write_review(self.voice.folder, ordered)

    def fix(self, fixed: Row, said: str):
        """Says `said` for the row's clip from now on. A said with the phonemes of the word
        list's drops the fix."""
        phonemes = lambda said: self.voice.phonemes(pack.spoken(fixed.clip, said))
        if phonemes(said) == phonemes(fixed.base):
            self.fixes.pop((fixed.text, fixed.clip), None)
        else:
            self.fixes[fixed.text, fixed.clip] = said
        pack.write_fixes(self.voice.folder, self.fixes)
        self.update(fixed)

    def counts(self) -> str:
        states = [self.state(row) for row in self.rows]
        return f"{len(states) - states.count('unmarked') - states.count('stale')} / {len(states)} marked, {states.count('bad')} bad, {states.count('stale')} stale"


class Model(QAbstractTableModel):
    """The rows of `review` that the filter shows."""

    def __init__(self, review: Review):
        super().__init__()
        self.review = review
        self.filter = FILTERS[0]
        self.shown = self.matching()

    def matching(self) -> list[int]:
        return [i for i, row in enumerate(self.review.rows) if self.shows(row)]

    def shows(self, row: Row) -> bool:
        state = self.review.state(row)
        return self.filter == "All" or state == "bad" or (self.filter == "To review" and state in ("unmarked", "stale"))

    def set_filter(self, name: str):
        self.beginResetModel()
        self.filter = name
        self.shown = self.matching()
        self.endResetModel()

    def row(self, index: int) -> Row:
        return self.review.rows[self.shown[index]]

    def changed(self, index: int):
        """Takes the row out when it no longer matches the filter, or redraws it."""
        if self.shows(self.row(index)):
            self.dataChanged.emit(self.index(index, 0), self.index(index, len(COLUMNS) - 1))
        else:
            self.beginRemoveRows(QModelIndex(), index, index)
            del self.shown[index]
            self.endRemoveRows()

    def rowCount(self, parent=QModelIndex()):
        return 0 if parent.isValid() else len(self.shown)

    def columnCount(self, parent=QModelIndex()):
        return len(COLUMNS)

    def headerData(self, section, orientation, role=Qt.DisplayRole):
        if role == Qt.DisplayRole and orientation == Qt.Horizontal:
            return COLUMNS[section]
        return None

    def data(self, index, role=Qt.DisplayRole):
        if role != Qt.DisplayRole:
            return None
        row = self.row(index.row())
        state = self.review.state(row)
        return (row.text, row.clip, "" if state == "unmarked" else state, row.said, self.review.note(row))[index.column()]


class Player:
    """Plays a clip as the device does: ADPCM decoded, then straight lines up to 17,578 Hz."""

    def __init__(self, status: QLabel):
        self.status = status
        self.format = QAudioFormat()
        self.format.setSampleRate(PLAY_HZ)
        self.format.setChannelCount(1)
        self.format.setSampleFormat(QAudioFormat.Int16)
        device = QMediaDevices.defaultAudioOutput()
        if not device.isFormatSupported(self.format):
            sys.exit(f"{device.description()} does not play 16-bit mono at {PLAY_HZ} Hz")
        self.sink = QAudioSink(device, self.format)
        # The sink reads these while it plays, so they live as long as the player.
        self.data = QByteArray()
        self.buffer = QBuffer()

    def play(self, clip: bytes):
        pcm = np.frombuffer(audioop.adpcm2lin(clip, 2, None)[0], "<i2")
        played = upsample(pcm, pack.FACTOR).astype(np.float32) / 32768
        self.sink.stop()
        self.buffer.close()
        self.data = QByteArray(resample(played, OUTPUT_HZ, PLAY_HZ))
        self.buffer = QBuffer(self.data)
        self.buffer.open(QIODevice.ReadOnly)
        self.sink.start(self.buffer)
        # PySide6 cannot pass the sink's state signal to Python, so errors are read here.
        if self.sink.error() != QtAudio.Error.NoError:
            self.status.setText(f"audio error: {self.sink.error().name}")

    def close(self):
        self.sink.stop()


class Table(QTableView):
    def keyboardSearch(self, search: str):
        """Letters are marks here, not a search for a row."""


class Window(QMainWindow):
    def __init__(self, review: Review):
        super().__init__()
        self.review = review
        self.trial: tuple[str, bytes] | None = None
        self.auto_phonemes = ""
        self.setWindowTitle(f"PocketCW audition: {review.voice.folder.name}")

        self.filter = QComboBox()
        self.filter.addItems(FILTERS)
        self.progress = QLabel()
        self.model = Model(review)
        self.table = Table()
        self.table.setModel(self.model)
        self.table.setSelectionBehavior(QAbstractItemView.SelectRows)
        self.table.setSelectionMode(QAbstractItemView.SingleSelection)
        self.table.setEditTriggers(QAbstractItemView.NoEditTriggers)
        self.table.verticalHeader().hide()
        self.table.horizontalHeader().setSectionResizeMode(QHeaderView.ResizeToContents)
        self.table.horizontalHeader().setStretchLastSection(True)

        self.word = QLabel()
        self.word.setFont(QFont(self.word.font().family(), 32, QFont.Bold))
        self.says = QLabel()
        self.says.setWordWrap(True)
        self.says.setTextInteractionFlags(Qt.TextSelectableByMouse)
        self.note = QLineEdit()
        self.note.setPlaceholderText("what is wrong; Enter marks bad")
        self.respelling = QLineEdit()
        self.phonemes = QLineEdit()
        # Stress marks are a few pixels wide at the default size.
        self.phonemes.setFont(QFont(self.phonemes.font().family(), round(self.phonemes.font().pointSize() * 1.5)))
        # A tab or line break would break a row of fixes.txt or review.txt.
        for field in (self.note, self.respelling, self.phonemes):
            field.setValidator(QRegularExpressionValidator(f"[^{pack.BREAKS.pattern[1:-1]}]*", field))
        self.status = QLabel()
        self.status.setWordWrap(True)
        good = QPushButton("Good (G)")
        bad = QPushButton("Bad (B)")
        speak = QPushButton("Speak")
        play_kept = QPushButton("Play kept")
        keep = QPushButton("Keep fix")
        reset = QPushButton("Reset")

        fix = QGroupBox("Fix")
        form = QFormLayout(fix)
        form.addRow("Respelling", self.respelling)
        form.addRow("Phonemes", self.phonemes)
        buttons = QHBoxLayout()
        for button in (speak, play_kept, keep, reset):
            buttons.addWidget(button)
        form.addRow(buttons)
        right = QVBoxLayout()
        for widget in (self.word, self.says, QLabel("Note"), self.note):
            right.addWidget(widget)
        marks = QHBoxLayout()
        marks.addWidget(good)
        marks.addWidget(bad)
        right.addLayout(marks)
        for widget in (fix, self.phoneme_keys(), self.status):
            right.addWidget(widget)
        right.addStretch()
        left = QVBoxLayout()
        top = QHBoxLayout()
        top.addWidget(self.filter)
        top.addWidget(self.progress, 1)
        left.addLayout(top)
        left.addWidget(self.table)
        body = QHBoxLayout()
        body.addLayout(left, 3)
        body.addLayout(right, 2)
        central = QWidget()
        central.setLayout(body)
        self.setCentralWidget(central)
        self.resize(1100, 700)

        self.player = Player(self.status)
        self.play_timer = QTimer(singleShot=True, interval=PLAY_DELAY_MS)
        self.play_timer.timeout.connect(self.play_kept)
        self.filter.currentTextChanged.connect(self.set_filter)
        self.table.selectionModel().currentRowChanged.connect(self.show_row)
        mark_good = lambda: self.mark("good", "")
        mark_bad = lambda: self.mark("bad", self.note.text())
        self.note.returnPressed.connect(mark_bad)
        good.clicked.connect(mark_good)
        bad.clicked.connect(mark_bad)
        self.respelling.textEdited.connect(self.respelled)
        self.respelling.returnPressed.connect(self.speak)
        self.phonemes.returnPressed.connect(self.speak)
        speak.clicked.connect(self.speak)
        play_kept.clicked.connect(self.play_kept)
        keep.clicked.connect(self.keep)
        reset.clicked.connect(self.reset)
        for key, action in (
            ("G", mark_good),
            ("B", mark_bad),
            ("N", self.note.setFocus),
            ("Space", self.play_kept),
        ):
            shortcut = QShortcut(QKeySequence(key), self.table)
            shortcut.setContext(Qt.WidgetShortcut)
            shortcut.activated.connect(action)
        QShortcut(QKeySequence("Escape"), self).activated.connect(self.table.setFocus)
        self.select(0)
        self.table.setFocus()

    def closeEvent(self, event):
        self.player.close()
        super().closeEvent(event)

    def current(self) -> Row | None:
        index = self.table.currentIndex()
        return self.model.row(index.row()) if index.isValid() else None

    def select(self, index: int):
        self.progress.setText(self.review.counts())
        if self.model.rowCount() == 0:
            self.show_row(QModelIndex())
            return
        target = self.model.index(min(index, self.model.rowCount() - 1), 0)
        # Removing the current row can leave the table there already, with no signal.
        if target == self.table.currentIndex():
            self.show_row(target)
        else:
            self.table.setCurrentIndex(target)

    def set_filter(self, name: str):
        self.model.set_filter(name)
        self.select(0)

    def phoneme_keys(self) -> QGroupBox:
        """A key for each of PHONEME_KEYS, which types it in the phonemes field."""
        keys = QGroupBox("Phoneme keys")
        grid = QGridLayout(keys)
        at = 0
        for group, phonemes in PHONEME_KEYS.items():
            grid.addWidget(QLabel(group), at, 0, 1, KEY_COLUMNS)
            at += 1
            for i, (phoneme, example) in enumerate(phonemes):
                key = QPushButton(f"{phoneme}  {example}")
                # The phonemes field keeps its cursor while a key is clicked.
                key.setFocusPolicy(Qt.NoFocus)
                key.clicked.connect(lambda _=False, phoneme=phoneme: self.type_phoneme(phoneme))
                grid.addWidget(key, at + i // KEY_COLUMNS, i % KEY_COLUMNS)
            at += -(-len(phonemes) // KEY_COLUMNS)
        return keys

    def type_phoneme(self, phoneme: str):
        self.phonemes.insert(phoneme)
        self.phonemes.setFocus()

    def show_row(self, index, previous=None):
        self.trial = None
        self.status.clear()
        row = self.model.row(index.row()) if index.isValid() else None
        if row is None:
            self.word.setText("Nothing to review")
            self.says.clear()
            return
        self.word.setText(row.text)
        self.says.setText(f"Says: {row.spoken}\nPhonemes: {row.phonemes}\nMark: {self.review.state(row)}")
        self.note.setText(self.review.note(row))
        fixed = pack.PHONEMES.fullmatch(row.said)
        self.respelling.setText(row.base if fixed else row.said)
        self.auto_phonemes = self.said_phonemes(self.respelling.text())
        self.phonemes.setText(fixed[1] if fixed else self.auto_phonemes)
        self.play_timer.start()

    def said_phonemes(self, said: str) -> str:
        """The phonemes of `said` as the row's clip says it, with its periods, so that phonemes
        edited from them keep these."""
        row = self.current()
        if row is None:
            return ""
        return self.review.voice.phonemes(pack.spoken(row.clip, said))

    def clip(self, phonemes: str) -> bytes | None:
        """The clip for `phonemes`, spoken now when it is not in the cache."""
        QApplication.setOverrideCursor(Qt.WaitCursor)
        try:
            return self.review.voice.clip(phonemes)[0]
        except ValueError as error:
            self.status.setText(str(error))
            return None
        finally:
            QApplication.restoreOverrideCursor()

    def play_kept(self):
        row = self.current()
        if row is not None and (clip := self.clip(row.phonemes)) is not None:
            self.player.play(clip)

    def mark(self, mark: str, note: str):
        row = self.current()
        if row is None:
            return
        at = self.table.currentIndex().row()
        self.review.mark(row, mark, note)
        self.model.changed(at)
        # A row the filter took out leaves the next one at the same place.
        self.select(at if at >= self.model.rowCount() or self.model.row(at) is not row else at + 1)
        self.table.setFocus()

    def respelled(self, text: str):
        """Keeps the phonemes in step with the respelling until they are edited by hand."""
        in_step = self.phonemes.text() == self.auto_phonemes
        self.auto_phonemes = self.said_phonemes(text)
        if in_step:
            self.phonemes.setText(self.auto_phonemes)

    def tried_said(self) -> str:
        """The said the fix fields give: the respelling, or the phonemes once edited by hand."""
        phonemes = self.phonemes.text().strip()
        return self.respelling.text().strip() if phonemes == self.auto_phonemes else f"/{phonemes}/"

    def speak(self):
        row = self.current()
        said = self.tried_said()
        if row is None:
            return
        if said in ("", "//"):
            self.status.setText("Nothing to speak")
            return
        spoken = pack.spoken(row.clip, said)
        clip = self.clip(self.review.voice.phonemes(spoken))
        if clip is not None:
            self.trial = (said, clip)
            self.status.setText(f"Trying: {spoken}")
            self.player.play(clip)

    def keep(self):
        row = self.current()
        if row is None or self.trial is None:
            self.status.setText("Speak a fix first")
            return
        self.review.fix(row, self.trial[0])
        self.mark("good", "")

    def reset(self):
        """Puts the word list's said in the fields and speaks it; Keep then drops the fix."""
        row = self.current()
        if row is None:
            return
        self.respelling.setText(row.base)
        self.auto_phonemes = self.said_phonemes(row.base)
        self.phonemes.setText(self.auto_phonemes)
        self.speak()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--voice", default=pack.VOICE, help=f"folder in tools/pack/voices (default {pack.VOICE})")
    parser.add_argument("--models", type=Path, default=pack.HERE / "models", help="folder with the model files")
    parser.add_argument("--cache", type=Path, default=pack.CACHE, help="the pack's clip cache")
    args = parser.parse_args()
    app = QApplication(sys.argv[:1])
    window = Window(Review(pack.Voice(args.voice, args.models, args.cache)))
    window.show()
    sys.exit(app.exec())


if __name__ == "__main__":
    main()
