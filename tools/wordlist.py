#!/usr/bin/env python3
"""Writes tools/words/common.txt: the English words in the top RANKS of wordfreq, letters only,
without single letters other than A and I, and without the words in tools/words/blocklist.txt.
Needs the wordfreq package.

    python tools/wordlist.py
"""

import importlib.metadata
from pathlib import Path

import wordfreq

# The blocklist was reviewed for CW contacts down to this rank; a blocked word leaves a gap
# instead of pulling in an unreviewed one.
RANKS = 3177
WORDS = Path(__file__).parent / "words"


def main():
    blocked = {
        line.strip() for line in (WORDS / "blocklist.txt").read_text().splitlines() if line.strip() and not line.startswith("#")
    }
    kept = []
    for word in wordfreq.top_n_list("en", RANKS):
        if word.isascii() and word.isalpha() and (len(word) > 1 or word in "ai") and word not in blocked:
            kept.append(word.upper())
    version = importlib.metadata.version("wordfreq")
    header = (
        f"# English words from the top {RANKS} of wordfreq {version} by Robyn Speer, most frequent first.\n"
        "# Data license: CC BY-SA 4.0. Built by tools/wordlist.py.\n"
    )
    (WORDS / "common.txt").write_text(header + "\n".join(kept) + "\n")
    print(f"{WORDS / 'common.txt'}: {len(kept)} words")


if __name__ == "__main__":
    main()
