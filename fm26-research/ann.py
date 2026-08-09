#!/usr/bin/env python3
"""Annotate an FM26 game_db byte range as a (tag, type, value) stream.

Records are self-describing: <4-char tag stored byte-reversed><0x01><type><value>.
This walks a buffer, printing every plausible tag site with its decoded value,
so record layouts can be read directly instead of byte-hunted.
"""
import sys, struct

TAGCH = set(range(0x30, 0x7b)) | {0x20, 0x5f}


def is_tag(b, i):
    if i + 4 > len(b):
        return False
    return all(c in TAGCH for c in b[i:i + 4])


def tagname(b, i):
    return b[i:i + 4][::-1].decode('latin1')


def val(b, i, t):
    """Return (text, next_index) for a value of type byte t at i."""
    if t == 0x11:
        return (str(b[i]), i + 1)
    if t == 0x12:
        if i + 2 > len(b): return (None, None)
        return (str(struct.unpack_from('<H', b, i)[0]), i + 2)
    if t in (0x0a, 0x0b, 0x0f):
        if i + 4 > len(b): return (None, None)
        return (str(struct.unpack_from('<I', b, i)[0]), i + 4)
    if t == 0x1a:
        if i + 4 > len(b): return (None, None)
        n = struct.unpack_from('<I', b, i)[0]
        if n > 4096 or i + 4 + n > len(b): return (None, None)
        return (repr(b[i + 4:i + 4 + n].decode('utf-8', 'replace')), i + 4 + n)
    if t in (0x02, 0x03):
        return ('-', i)
    return (None, None)


def walk(b, base=0):
    i = 0
    out = []
    while i < len(b) - 6:
        if is_tag(b, i) and b[i + 4] == 0x01:
            t = b[i + 5]
            text, nxt = val(b, i + 6, t)
            if text is not None:
                out.append((base + i, tagname(b, i), t, text))
                i = nxt
                continue
        i += 1
    return out


if __name__ == '__main__':
    data = open(sys.argv[1], 'rb').read()
    base = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    for off, tag, t, v in walk(data, base):
        print(f"{off:>12}  {tag}  t={t:#04x}  {v}")
