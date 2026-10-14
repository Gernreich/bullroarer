#!/usr/bin/env python3
"""The five bullroarer blades in 3D, from their cut files.

    python3 viewer.py [--out=bullroarer-3d.html]

Each blade is its cut file's own outline at 3mm, read by
lasermade-tools/turn3d.py, with its cord tied through the one hole. A
bullroarer is whirled round on that cord, and as it goes it spins about its own
long axis -- that spin is what roars -- so "Spin it" turns the blade about the
line from the hole along its length.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'lasermade-tools'))
import turn3d as t   # noqa: E402

BLADES = [('rectangular', 'BullroarerRectangular.svg'),
          ('convex tapered', 'BullroarerConvexTapered.svg'),
          ('convex narrow tapered', 'BullroarerConvexNarrowTapered.svg'),
          ('concave tapered', 'BullroarerConcaveTapered.svg'),
          ('sawtooth', 'BullroarerSawtooth.svg')]
CORD = 160


def build():
    out = {}
    for name, f in BLADES:
        ps = t.parts(os.path.join(HERE, f))
        assert len(ps) == 1 and len(ps[0]['h']) == 1, (f, len(ps))
        b = t.centred(ps[0])
        h = b['h'][0]
        hx = sum(q[0] for q in h) / len(h)
        hy = sum(q[1] for q in h) / len(h)
        x0, _, x1, _ = b['box']
        away = -1 if hx < (x0 + x1) / 2 else 1            # the cord leaves past the near end
        end = x0 if away < 0 else x1
        cord = [(hx, hy, 3.6), (hx + away * 4, hy, 4.5), (end + away * 8, hy, 1.5),
                (end + away * CORD / 2, hy, 1.5), (end + away * CORD, hy, 1.5)]
        out[name] = {'parts': [t.placed(b, t.T(0, 0, 0))], 'cords': [cord],
                     'axis': [hx, hy, 1.5, 1, 0, 0], 'ex': 0, 'view': [-0.5, 0.55, 3.4]}
    return out


def main(argv):
    o = [a.split('=', 1)[1] for a in argv if a.startswith('--out=')]
    path = o[0] if o else os.path.join(HERE, 'bullroarer-3d.html')
    t.write(path, 'Bullroarer', build(),
            'Drag to turn, scroll or pinch to zoom; pick a blade above. Each is its cut file’s own '
            'outline at 3mm with the cord tied through its hole. Whirled on the cord, a blade spins about '
            'its own long axis, and that spin is what roars: tick Spin it to see which way.',
            'bullroarer/viewer.py', spin=True)
    print(f'  wrote {os.path.relpath(path)}')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
