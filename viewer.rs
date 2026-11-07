//! The five bullroarer blades in 3D, from their cut files.
//!
//!     target/release/viewer [--out=bullroarer-3d.html]
//!
//! Each blade is its cut file's own outline at 3 mm, read by
//! the turn3d module in lasermade-tools, with its cord tied through the one hole. A
//! bullroarer is whirled round on that cord, and as it goes it spins about its own
//! long axis -- that spin is what roars -- so "Spin it" turns the blade about the
//! line from the hole along its length.
use lasermade_tools::turn3d::*;

const HERE: &str = env!("CARGO_MANIFEST_DIR");
const BLADES: [(&str, &str); 5] = [
    ("rectangular", "BullroarerRectangular.svg"),
    ("convex tapered", "BullroarerConvexTapered.svg"),
    ("convex narrow tapered", "BullroarerConvexNarrowTapered.svg"),
    ("concave tapered", "BullroarerConcaveTapered.svg"),
    ("sawtooth", "BullroarerSawtooth.svg"),
];
const CORD: i64 = 160;

fn build() -> Vec<(String, J)> {
    let mut out = vec![];
    for (name, f) in BLADES {
        let ps = parts(&format!("{HERE}/{f}"));
        assert!(ps.len() == 1 && ps[0].h.len() == 1, "{f} {}", ps.len());
        let b = centred(&ps[0], None);
        let h = &b.h[0];
        let hx = h.iter().fold(0.0, |s, q| s + q[0].f()) / h.len() as f64;
        let hy = h.iter().fold(0.0, |s, q| s + q[1].f()) / h.len() as f64;
        let (x0, x1) = (b.bx[0].f(), b.bx[2].f());
        // the cord leaves past the near end
        let away: i64 = if hx < (x0 + x1) / 2.0 { -1 } else { 1 };
        let end = if away < 0 { x0 } else { x1 };
        let cord = [
            [F(hx), F(hy), F(3.6)],
            [F(hx + (away * 4) as f64), F(hy), F(4.5)],
            [F(end + (away * 8) as f64), F(hy), F(1.5)],
            [F(end + (away * CORD) as f64 / 2.0), F(hy), F(1.5)],
            [F(end + (away * CORD) as f64), F(hy), F(1.5)],
        ];
        out.push((
            name.to_string(),
            design(vec![
                ("parts", J::A(vec![placed(&b, &T(0, 0, 0), STILL, "ply")])),
                ("cords", J::A(vec![cord_j(&cord)])),
                ("axis", nums(&[F(hx), F(hy), F(1.5), I(1), I(0), I(0)])),
                ("ex", J::N(I(0))),
                ("view", nums(&[F(-0.5), F(0.55), F(3.4)])),
            ]),
        ));
    }
    out
}

fn cord_j(p: &[[Num; 3]]) -> J {
    cord(p)
}

fn main() {
    let path = out_arg(&format!("{HERE}/bullroarer-3d.html"));
    write(&path, "Bullroarer", &mut build(), "Drag to turn; right-drag, shift-drag or move two fingers to slide the view; scroll or pinch to zoom in on whatever is under the pointer; double-click a part to turn round it, and Reset view to go back. Pick a blade above. Each is its cut file’s own outline at 3 mm with the cord tied through its hole. Whirled on the cord, a blade spins about its own long axis, and that spin is what roars: tick Spin it to see which way.", "bullroarer/viewer.rs", true);
    println!("  wrote {}", relpath(&path));
}
