#!/usr/bin/env python3
"""Print THIRD-PARTY.txt: the MIT notice of every crate linked into the firmware.

usage: python3 -I tools/notices.py FIRMWARE_DIR > THIRD-PARTY.txt

Every linked crate offers MIT, so MIT is the license this project takes for each.
A crate that ships no MIT license file gets the standard text under its Cargo authors.
"""
import json
import pathlib
import re
import subprocess
import sys

TARGET = 'thumbv7em-none-eabihf'
MIT_FILE = re.compile(r'^LICEN[CS]E[-_.]?MIT(\.(md|txt))?$', re.I)
PLAIN_FILE = re.compile(r'^LICEN[CS]E(\.(md|txt))?$', re.I)
MIT_TEXT = """\
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
"""


def cargo(firmware, *args):
    return subprocess.run(['cargo', *args], cwd=firmware, check=True, capture_output=True, text=True).stdout


def notice(package):
    root = pathlib.Path(package['manifest_path']).parent
    files = sorted(p for p in root.iterdir() if p.is_file())
    found = [p for p in files if MIT_FILE.match(p.name)]
    # A crate under MIT alone often names its file plain LICENSE.
    if not found and package['license'] == 'MIT':
        found = [p for p in files if PLAIN_FILE.match(p.name)]
    if found:
        return found[0].read_text().strip() + '\n'
    authors = ', '.join(package['authors']) or f"the {package['name']} authors"
    return f'Copyright (c) {authors}\n\n{MIT_TEXT}'


def main():
    if len(sys.argv) != 2:
        sys.exit('usage: notices.py FIRMWARE_DIR')
    firmware = sys.argv[1]
    tree = cargo(firmware, 'tree', '-e', 'normal,no-proc-macro', '--target', TARGET, '--prefix', 'none', '-f', '{p}')
    linked = {tuple(line.split()[:2]) for line in tree.splitlines() if line.strip()}
    meta = json.loads(cargo(firmware, 'metadata', '--format-version', '1', '--filter-platform', TARGET))
    # A local path crate has no source; those are this project's own GPL crates.
    packages = sorted(
        (p for p in meta['packages'] if (p['name'], 'v' + p['version']) in linked and p['source'] is not None),
        key=lambda p: (p['name'], p['version']),
    )
    for p in packages:
        if not re.search(r'\bMIT\b', p['license'] or ''):
            sys.exit(f"error: {p['name']} {p['version']} does not offer MIT ({p['license']})")
    print('PocketCW firmware includes the crates below. Each is used under the MIT license.\n')
    for p in packages:
        print(f"{'=' * 78}\n{p['name']} {p['version']}  ({p.get('repository') or 'crates.io'})\n{'=' * 78}\n")
        print(notice(p))


main()
