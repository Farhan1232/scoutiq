#!/usr/bin/env python3
"""Recursive decoder for FM26's self-describing tagged region.

Encoding:
  field  := <4-byte tag, byte-reversed> 0x01 <type> <value>
  object := 0x01 0x0a <u32 nfields> field*      (first field is conventionally
                                                 `id`, whose value is the class)
  array  := <tag> 0x01 0x0b <u32 n> object*
"""
import struct, sys, collections

TAGCH = set(range(0x30, 0x7b)) | {0x20, 0x5f}


def tag(d, i):
    return d[i:i + 4][::-1].decode('latin1')


def is_tag(d, i):
    return i + 4 <= len(d) and all(c in TAGCH for c in d[i:i + 4])


class Fail(Exception):
    pass


def read_value(d, i, t, depth):
    """Return (value, next_index)."""
    if t == 0x01:                              # plain u32
        if i + 4 > len(d): raise Fail(i)
        return struct.unpack_from('<I', d, i)[0], i + 4
    if t == 0x02:                              # 4-byte class/tag literal
        if not is_tag(d, i): raise Fail(i)
        return tag(d, i), i + 4
    if t in (0x03, 0x11, 0x18):                # single byte
        if i >= len(d): raise Fail(i)
        return d[i], i + 1
    if t == 0x12:
        if i + 2 > len(d): raise Fail(i)
        return struct.unpack_from('<H', d, i)[0], i + 2
    if t == 0x0f:                              # pair of u32
        if i + 8 > len(d): raise Fail(i)
        return struct.unpack_from('<II', d, i), i + 8
    if t == 0x1a:                              # length-prefixed string
        if i + 4 > len(d): raise Fail(i)
        n = struct.unpack_from('<I', d, i)[0]
        if n > 8192 or i + 4 + n > len(d): raise Fail(i)
        return d[i + 4:i + 4 + n].decode('utf-8', 'replace'), i + 4 + n
    if t == 0x0a:                              # embedded object
        if i + 4 > len(d): raise Fail(i)
        n = struct.unpack_from('<I', d, i)[0]
        if n > 512: raise Fail(i)
        return read_fields(d, i + 4, n, depth + 1)
    if t == 0x0b:                              # array of objects
        if i + 4 > len(d): raise Fail(i)
        n = struct.unpack_from('<I', d, i)[0]
        if n > 100000: raise Fail(i)
        out, j = [], i + 4
        for _ in range(n):
            if j + 2 > len(d) or d[j] != 0x01 or d[j + 1] != 0x0a:
                raise Fail(j)
            cnt = struct.unpack_from('<I', d, j + 2)[0]
            if cnt > 512: raise Fail(j)
            o, j = read_fields(d, j + 6, cnt, depth + 1)
            out.append(o)
        return out, j
    raise Fail(i)


def read_fields(d, i, n, depth=0):
    obj = {}
    for _ in range(n):
        if not is_tag(d, i) or i + 6 > len(d) or d[i + 4] != 0x01:
            raise Fail(i)
        name = tag(d, i)
        t = d[i + 5]
        v, i = read_value(d, i + 6, t, depth)
        # duplicate field names do occur; keep them all
        if name in obj:
            if not isinstance(obj[name], list) or not obj.get('__multi_' + name):
                obj[name] = [obj[name]]
                obj['__multi_' + name] = True
            obj[name].append(v)
        else:
            obj[name] = v
    return obj, i


def objects(d, base=0):
    """Yield (abs_offset, nfields, obj) for every top-level object that parses."""
    i = 0
    while i + 6 <= len(d):
        if d[i] == 0x01 and d[i + 1] == 0x0a:
            n = struct.unpack_from('<I', d, i + 2)[0]
            if 1 <= n <= 512 and is_tag(d, i + 6):
                try:
                    o, j = read_fields(d, i + 6, n)
                    if j > i + 6:
                        yield (base + i, n, o)
                        i = j
                        continue
                except Fail:
                    pass
        i += 1


if __name__ == '__main__':
    data = open(sys.argv[1], 'rb').read()
    base = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    classes = collections.Counter()
    fields_by_class = collections.defaultdict(collections.Counter)
    n = 0
    for off, cnt, o in objects(data, base):
        n += 1
        cls = o.get('id', '?')
        if isinstance(cls, str):
            classes[cls] += 1
            for k in o:
                if not k.startswith('__'):
                    fields_by_class[cls][k] += 1
    print('top-level objects parsed:', n)
    print('\n=== classes by frequency ===')
    for c, k in classes.most_common(40):
        print('%8d  %-6s  fields: %s' % (k, c, ' '.join(sorted(fields_by_class[c]))))
