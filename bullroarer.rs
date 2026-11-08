//! The five bullroarer blades, each from a few numbers: its cut file and its
//! SolidCraft design (2026-11-08; drawn by hand in Inkscape until then).
//!
//!     target/release/bullroarer [--blade=NAME [--length=MM ...]] [--out=DIR] [--solidcraft=DIR] [--no-write]
//!
//! Without --blade it writes all five at their defaults; a parameter flag
//! needs --blade, since each blade has its own. Every blade is symmetric about
//! its long centreline with the cord hole on it, and each is drawn as it was:
//!
//!   rectangular            a rectangle
//!   convex-tapered         two mirrored cubic Béziers, point to round end
//!   concave-tapered        the same, its first control point on the centreline
//!   convex-narrow-tapered  a half-ellipse blunt end and two Béziers to a point
//!   sawtooth               a core with teeth down both edges, each edge the
//!                          other turned half a turn, as a propeller is
//!
//! A Bézier blade's width is exact: the control points' heights are scaled so
//! the curve's widest point is `width`, the ratio between them (the drawing's)
//! fixed. Their places along the blade are fractions of its length.
//!
//! The SolidCraft design is the blade as one body `laminations` × `ply`
//! thick: the README's two or three blades glued face to face are one part.
//! The curves are drawn from their formulas, so they follow every parameter,
//! and the sawtooth's teeth are a pattern whose count is the `teeth`
//! parameter.
use lasermade_tools::flat::{f as fm, outs, write, Checks, Params, Sheet, Unit::*};
use lasermade_tools::solidcraft::{n, Script};

const HERE: &str = env!("CARGO_MANIFEST_DIR");
const BED: (f64, f64) = (600.0, 305.0);
const DENSITY: f64 = 0.68; // Baltic birch, g/cm3
const MIN_EDGE: f64 = 3.0; // ply round the cord hole, as flat-part-check asks
const N: usize = 4000; // samples a curve, for areas and distances

// the drawing's ratios between a Bézier blade's two control-point heights
const CONVEX_LEAN: f64 = 59.772378 / 59.324678;
const CONCAVE_LEAN: f64 = 0.0;

pub const BLADES: [(&str, &str); 5] = [
    ("rectangular", "BullroarerRectangular"),
    ("convex-tapered", "BullroarerConvexTapered"),
    ("concave-tapered", "BullroarerConcaveTapered"),
    ("convex-narrow-tapered", "BullroarerConvexNarrowTapered"),
    ("sawtooth", "BullroarerSawtooth"),
];

fn common(v: &mut Vec<(&'static str, f64, lasermade_tools::flat::Unit, &'static str)>, hole_inset: f64, from: &'static str) {
    v.insert(0, ("ply", 3.0, Len, "the ply: one lamination"));
    v.insert(1, ("laminations", 1.0, Num, "blades glued face to face: the design's thickness is laminations × ply"));
    v.push(("hole_d", 7.0, Len, "the cord hole"));
    v.push(("hole_inset", hole_inset, Len, from));
}

fn table(blade: &str) -> Params {
    let mut v = match blade {
        "rectangular" => vec![("length", 200.0, Len, "the blade, end to end"), ("width", 42.127, Len, "the blade, across")],
        "convex-tapered" => vec![
            ("length", 150.3666, Len, "point to round end, along the centreline"),
            ("width", 89.3231, Len, "the blade at its widest"),
            ("c1_x", 0.6535, Ratio, "the first control point, along: a fraction of the length"),
            ("c2_x", 1.0236, Ratio, "the second control point, along: past 1 rounds the end"),
        ],
        "concave-tapered" => vec![
            ("length", 200.0, Len, "point to round end, along the centreline"),
            ("width", 94.5916, Len, "the blade at its widest"),
            ("c1_x", 0.3333, Ratio, "the first control point, along: a fraction of the length"),
            ("c2_x", 1.0391, Ratio, "the second control point, along: past 1 rounds the end"),
        ],
        "convex-narrow-tapered" => vec![
            ("length", 204.2912, Len, "blunt end to point"),
            ("end_w", 29.7277, Len, "the blunt end, across: where the Béziers start"),
            ("cap", 3.2839, Len, "the blunt end's half-ellipse, deep"),
            ("belly_x", 0.5246, Ratio, "the control point, along the curve's run: a fraction"),
            ("belly", 7.5933, Len, "the control point, out beyond the blunt end's width"),
        ],
        "sawtooth" => vec![
            ("length", 169.2231, Len, "the blade, end to end"),
            ("teeth", 30.0, Num, "teeth down each edge"),
            ("core", 34.4578, Len, "the blade between the teeth's roots"),
            ("depth", 16.5922, Len, "a tooth, root to tip"),
            ("lean", 0.4485, Ratio, "a tooth's tip along its pitch, from the end it starts at"),
        ],
        _ => vec![],
    };
    let (inset, from) = match blade {
        "rectangular" => (10.5833, "the cord hole's centre from the near end"),
        "convex-tapered" => (7.1665, "the cord hole's centre from the round end"),
        "concave-tapered" => (8.143, "the cord hole's centre from the round end"),
        "convex-narrow-tapered" => (7.9702, "the cord hole's centre from the blunt end"),
        _ => (8.7918, "the cord hole's centre from the near end"),
    };
    common(&mut v, inset, from);
    Params::new(&v)
}

type P = (f64, f64);

/// A piece of outline, from wherever the last one ended.
#[derive(Clone, Copy)]
enum Seg {
    Line(P),
    Cubic(P, P, P),
    /// a half-ellipse round the left: centre, rx, ry, from (cx, cy+ry) to (cx, cy-ry)
    Cap(P, f64, f64),
}

/// A blade in its own frame: x along the centreline from the left end, y
/// down (the sheet's way), the centreline at y = 0; the outline from its
/// start and the hole's centre.
struct Blade {
    start: P,
    segs: Vec<Seg>,
    hole: P,
}

fn bez(p0: P, c1: P, c2: P, p3: P, t: f64) -> P {
    let (a, b, c, d) = ((1.0 - t).powi(3), 3.0 * (1.0 - t).powi(2) * t, 3.0 * (1.0 - t) * t * t, t.powi(3));
    (a * p0.0 + b * c1.0 + c * c2.0 + d * p3.0, a * p0.1 + b * c1.1 + c * c2.1 + d * p3.1)
}

/// The height scale that makes 3(1-t)²t·lean + 3(1-t)t² peak at 1, and
/// where along t it peaks.
fn peak(lean: f64) -> (f64, f64) {
    // the derivative is (3lean-3)t² + (2-4lean)t + lean
    let (a, b, c) = (3.0 * lean - 3.0, 2.0 - 4.0 * lean, lean);
    let t = if a.abs() < 1e-12 {
        -c / b
    } else {
        let r = (b * b - 4.0 * a * c).sqrt();
        // the root inside the curve: with lean 0, t = 0 is a root too, the
        // curve's start, where it is flat but not at its height
        [(-b + r) / (2.0 * a), (-b - r) / (2.0 * a)].into_iter().find(|t| *t > 1e-9 && *t < 1.0).unwrap()
    };
    let f = 3.0 * (1.0 - t) * t * ((1.0 - t) * lean + t);
    (1.0 / f, t)
}

fn lean_of(blade: &str) -> f64 {
    if blade == "convex-tapered" {
        CONVEX_LEAN
    } else {
        CONCAVE_LEAN
    }
}

fn blade(name: &str, p: &Params) -> Blade {
    let l = p.get("length");
    match name {
        "rectangular" => {
            let h = p.get("width") / 2.0;
            Blade { start: (0.0, -h), segs: vec![Seg::Line((l, -h)), Seg::Line((l, h)), Seg::Line((0.0, h)), Seg::Line((0.0, -h))], hole: (l - p.get("hole_inset"), 0.0) }
        }
        "convex-tapered" | "concave-tapered" => {
            let lean = lean_of(name);
            let (s, _) = peak(lean);
            let h = p.get("width") / 2.0 * s;
            let (c1, c2) = ((p.get("c1_x") * l, -lean * h), (p.get("c2_x") * l, -h));
            let m = |q: P| (q.0, -q.1);
            Blade { start: (0.0, 0.0), segs: vec![Seg::Cubic(c1, c2, (l, 0.0)), Seg::Cubic(m(c2), m(c1), (0.0, 0.0))], hole: (l - p.get("hole_inset"), 0.0) }
        }
        "convex-narrow-tapered" => {
            let (cap, ry) = (p.get("cap"), p.get("end_w") / 2.0);
            let c1 = (cap + p.get("belly_x") * (l - cap), -(ry + p.get("belly")));
            Blade {
                start: (cap, -ry),
                segs: vec![Seg::Cubic(c1, (l, 0.0), (l, 0.0)), Seg::Cubic((l, 0.0), (c1.0, -c1.1), (cap, ry)), Seg::Cap((cap, 0.0), cap, ry)],
                hole: (p.get("hole_inset"), 0.0),
            }
        }
        _ => {
            let teeth = p.get("teeth") as usize;
            let (c, d, lean) = (p.get("core") / 2.0, p.get("depth"), p.get("lean"));
            let pitch = l / teeth as f64;
            let mut segs = vec![];
            // the top edge left to right, each tooth's tip lean of the way along
            for k in 0..teeth {
                let x = k as f64 * pitch;
                segs.push(Seg::Line((x + lean * pitch, -c - d)));
                segs.push(Seg::Line((x + pitch, -c)));
            }
            segs.push(Seg::Line((l, c)));
            // the bottom edge right to left: the top turned half a turn
            for k in (0..teeth).rev() {
                let x = k as f64 * pitch;
                segs.push(Seg::Line((x + (1.0 - lean) * pitch, c + d)));
                segs.push(Seg::Line((x, c)));
            }
            segs.push(Seg::Line((0.0, -c)));
            Blade { start: (0.0, -c), segs, hole: (l - p.get("hole_inset"), 0.0) }
        }
    }
}

impl Blade {
    /// The outline as points: every curve finely.
    fn points(&self) -> Vec<P> {
        let mut o = vec![self.start];
        let mut at = self.start;
        for s in &self.segs {
            match *s {
                Seg::Line(q) => {
                    o.push(q);
                    at = q;
                }
                Seg::Cubic(c1, c2, q) => {
                    o.extend((1..=N).map(|i| bez(at, c1, c2, q, i as f64 / N as f64)));
                    at = q;
                }
                Seg::Cap(c, rx, ry) => {
                    // from (cx, cy+ry) round the left to (cx, cy-ry)
                    o.extend((1..=N).map(|i| {
                        let t = std::f64::consts::PI * i as f64 / N as f64;
                        (c.0 - rx * t.sin(), c.1 + ry * t.cos())
                    }));
                    at = (c.0, c.1 - ry);
                }
            }
        }
        o.pop(); // the start again
        o
    }
    fn area(&self) -> f64 {
        let p = self.points();
        let n = p.len();
        (0..n).map(|i| p[i].0 * p[(i + 1) % n].1 - p[(i + 1) % n].0 * p[i].1).sum::<f64>().abs() / 2.0
    }
    /// The bounding box: exact for lines and the cap, and for a Bézier its
    /// ends and wherever its derivative is zero.
    fn bbox(&self) -> (P, P) {
        let (mut lo, mut hi) = (self.start, self.start);
        let mut grow = |q: P| {
            lo = (lo.0.min(q.0), lo.1.min(q.1));
            hi = (hi.0.max(q.0), hi.1.max(q.1));
        };
        let mut at = self.start;
        for s in &self.segs {
            match *s {
                Seg::Line(q) => {
                    grow(q);
                    at = q;
                }
                Seg::Cubic(c1, c2, q) => {
                    grow(q);
                    for k in 0..2 {
                        let g = |p: P| if k == 0 { p.0 } else { p.1 };
                        let (p0, p1, p2, p3) = (g(at), g(c1), g(c2), g(q));
                        let (a, b, c) = (-p0 + 3.0 * p1 - 3.0 * p2 + p3, 2.0 * (p0 - 2.0 * p1 + p2), p1 - p0);
                        let ts: Vec<f64> = if a.abs() < 1e-12 {
                            if b.abs() < 1e-12 {
                                vec![]
                            } else {
                                vec![-c / b]
                            }
                        } else {
                            let dd = b * b - 4.0 * a * c;
                            if dd < 0.0 {
                                vec![]
                            } else {
                                vec![(-b + dd.sqrt()) / (2.0 * a), (-b - dd.sqrt()) / (2.0 * a)]
                            }
                        };
                        for t in ts.into_iter().filter(|t| *t > 0.0 && *t < 1.0) {
                            grow(bez(at, c1, c2, q, t));
                        }
                    }
                    at = q;
                }
                Seg::Cap(c, rx, ry) => {
                    grow((c.0 - rx, c.1));
                    grow((c.0, c.1 - ry));
                    at = (c.0, c.1 - ry);
                }
            }
        }
        (lo, hi)
    }
    /// The least distance from the hole's centre to the outline.
    fn reach(&self) -> f64 {
        let p = self.points();
        let n = p.len();
        (0..n)
            .map(|i| {
                let (a, b) = (p[i], p[(i + 1) % n]);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let t = (((self.hole.0 - a.0) * dx + (self.hole.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                (self.hole.0 - a.0 - t * dx).hypot(self.hole.1 - a.1 - t * dy)
            })
            .fold(f64::INFINITY, f64::min)
    }
    /// The outline as SVG path data, moved by (dx, dy).
    fn d(&self, dx: f64, dy: f64) -> String {
        let pt = |q: P| format!("{},{}", fm(q.0 + dx), fm(q.1 + dy));
        let mut o = format!("M {}", pt(self.start));
        for s in &self.segs {
            match *s {
                Seg::Line(q) => o.push_str(&format!(" L {}", pt(q))),
                Seg::Cubic(c1, c2, q) => o.push_str(&format!(" C {} {} {}", pt(c1), pt(c2), pt(q))),
                Seg::Cap(c, rx, ry) => {
                    // two quarter arcs, so neither is a half-turn an importer can flip
                    o.push_str(&format!(" A {},{} 0 0 1 {} A {},{} 0 0 1 {}", fm(rx), fm(ry), pt((c.0 - rx, c.1)), fm(rx), fm(ry), pt((c.0, c.1 - ry))));
                }
            }
        }
        o.push_str(" Z");
        o
    }
}

// --- the SolidCraft design --------------------------------------------------------

/// The blade's outline into the open sketch, every point an expression; the
/// sheet's y down is the design's y up, which a symmetric blade does not see.
fn outline(s: &mut Script, name: &str) {
    let curve = |s: &mut Script, x: &str, y: &str, a: &str, b: &str| {
        s.cmd("sketch.equation_curve", format!("{{\"x\":\"{x}\",\"y\":\"{y}\",\"start\":\"{a}\",\"end\":\"{b}\",\"tolerance\":0.001}}"));
    };
    // the Bernstein weights of a cubic in t
    let (b1, b2, b3) = ("3*(1-t)^2*t", "3*(1-t)*t^2", "t^3");
    match name {
        "rectangular" => {
            s.cmd("sketch.rectangle.two_point", r#"{"p0":["0 mm","-width/2"],"p1":["length","width/2"]}"#.into());
        }
        "convex-tapered" | "concave-tapered" => {
            let lean = lean_of(name);
            let (k, _) = peak(lean);
            let (k1, k2) = (n(lean * k), n(k));
            let x = format!("length*({b1}*c1_x + {b2}*c2_x + {b3})");
            for sg in ["", "-"] {
                curve(s, &x, &format!("{sg}width/2*({b1}*{k1} + {b2}*{k2})"), "0", "1");
            }
        }
        "convex-narrow-tapered" => {
            // the Béziers from the blunt end's corners to the point, then the cap
            let x = format!("(1-t)^3*cap + {b1}*(cap + belly_x*(length - cap)) + {b2}*length + {b3}*length");
            for sg in ["", "-"] {
                curve(s, &x, &format!("{sg}((1-t)^3*end_w/2 + {b1}*(end_w/2 + belly))"), "0", "1");
            }
            curve(s, "cap - cap*sin(t)", "end_w/2*cos(t)", "0", "pi");
        }
        _ => {
            // the core here; the teeth are patterned on after
            s.cmd("sketch.rectangle.two_point", r#"{"p0":["0 mm","-core/2"],"p1":["length","core/2"]}"#.into());
        }
    }
}

fn design(name: &str, p: &Params) -> (String, f64) {
    let mut s = Script::new();
    p.declare(&mut s);
    s.len("thick", "laminations*ply", "read only: the blade's thickness");
    let b = blade(name, p);
    let body = format!("bullroarer {}", name.replace('-', " "));
    let sk = s.sketch("XY", None, &format!("{body} sketch"));
    outline(&mut s, name);
    let hx = if name == "convex-narrow-tapered" { "hole_inset" } else { "length - hole_inset" };
    if name != "sawtooth" {
        s.cmd("sketch.circle.center", format!(r#"{{"center":["{hx}","0 mm"],"diameter":"hole_d"}}"#));
    }
    s.finish();
    s.extrude(&sk, "thick", false, "new", &[], &body);
    if name == "sawtooth" {
        // a tooth on each edge, the bottom one the top turned half a turn,
        // patterned teeth times along; then the cord hole through the lot
        for (k, (y0, y1, x)) in [("core/2", "core/2 + depth", "lean"), ("-core/2", "-core/2 - depth", "(1 - lean)")].iter().enumerate() {
            let t = format!("{body} tooth {}", k + 1);
            let tk = s.sketch("XY", None, &format!("{t} sketch"));
            s.cmd("sketch.line", format!(r#"{{"points":[["0 mm","{y0}"],["{x}*length/teeth","{y1}"],["length/teeth","{y0}"]],"closed":true}}"#));
            s.finish();
            s.extrude(&tk, "thick", false, "join", &[&body], &t);
            s.cmd("solid.pattern.rectangular", format!(r#"{{"features":["{t}"],"dir1":[1,0,0],"count1":"teeth","spacing1":"length/teeth","name":"{t} pattern"}}"#));
        }
        s.cylinder(&format!("{body} cord hole"), ["length - hole_inset", "0 mm", "-1 mm"], None, "hole_d", "thick + 2 mm", "cut", &[&body]);
    }
    s.measure();
    let hole = std::f64::consts::PI * p.get("hole_d").powi(2) / 4.0;
    (s.text(), (b.area() - hole) * p.get("laminations") * p.get("ply"))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let pick = args.iter().find_map(|a| a.strip_prefix("--blade=").map(String::from));
    let rest: Vec<String> = args.into_iter().filter(|a| !a.starts_with("--blade=")).collect();
    let names: Vec<(&str, &str)> = match &pick {
        Some(b) => BLADES.iter().filter(|(n, _)| n == b).copied().collect(),
        None => BLADES.to_vec(),
    };
    if names.is_empty() {
        return Err(format!("error: no blade {}; they are {}.", pick.unwrap(), BLADES.map(|b| b.0).join(", ")));
    }
    let mut rc = 0;
    let mut first = true;
    for (name, stem) in names {
        let defaults = table(name);
        let mut p = table(name);
        let rest = p.parse(rest.clone())?;
        if pick.is_none() && p.changed(&defaults) != "" {
            return Err("error: a parameter flag needs --blade=NAME: each blade has its own.".into());
        }
        let o = outs(rest, &format!("--blade=NAME {}", p.flags()))?;
        if let Some(dir) = &o.solidcraft {
            let (text, v) = design(name, &p);
            write(&format!("{dir}/bullroarer-{name}.solidcraft.json"), &text)?;
            if std::env::var("SC_VOLUMES").is_ok() {
                eprintln!("bullroarer {}\t{v:.4}", name.replace('-', " "));
            }
            continue;
        }
        if !first {
            println!();
        }
        first = false;
        let b = blade(name, &p);
        let (lo, hi) = b.bbox();
        let (w, h) = (hi.0 - lo.0, hi.1 - lo.1);
        let hole = std::f64::consts::PI * p.get("hole_d").powi(2) / 4.0;
        let g = (b.area() - hole) * p.get("ply") * DENSITY / 1000.0;
        let edge = b.reach() - p.get("hole_d") / 2.0;
        println!("{name:22} {w:.1} × {h:.1}mm, {:.0}g a lamination, cord hole {}mm with {edge:.2}mm of ply round it", g, p.get("hole_d"));
        let mut c = Checks::new();
        c.note(b.hole.1 == 0.0, "the cord hole on the centreline", "the blade spins true".into());
        c.note(edge >= MIN_EDGE, "ply round the cord hole", format!("{edge:.2}mm (>= {MIN_EDGE})"));
        let shape_ok = match name {
            "convex-narrow-tapered" => p.get("cap") > 0.0 && p.get("cap") < p.get("length") && p.get("end_w") > 0.0,
            "sawtooth" => p.get("teeth") >= 1.0 && p.get("lean") > 0.0 && p.get("lean") < 1.0 && p.get("core") > p.get("hole_d"),
            _ => p.get("width") > p.get("hole_d"),
        };
        c.note(shape_ok, "the outline is a blade", format!("{w:.1} long, {h:.1} across"));
        c.note(w <= BED.0 && h <= BED.1, "fits the bed", format!("{} × {}mm", BED.0, BED.1));
        if c.done() != 0 {
            rc = 1;
        }
        if !o.no_write {
            let dir = o.out.clone().unwrap_or_else(|| HERE.to_string());
            let made = p.changed(&defaults);
            let with = if made.is_empty() { String::new() } else { format!(" Made with {made}.") };
            let mut sh = Sheet::new(w, h);
            sh.circle("#00ff00", b.hole.0 - lo.0, b.hole.1 - lo.1, p.get("hole_d"));
            sh.path("#000000", b.d(-lo.0, -lo.1));
            let desc = format!(
                "1 user unit = 1mm. A bullroarer blade, {}, drawn by bullroarer.rs --blade={name}.{with} Cut it in 3mm Baltic birch, face grain along the blade. Green #00ff00 cuts the cord hole first, then black #000000 frees the blade.",
                name.replace('-', " ")
            );
            write(&format!("{dir}/{stem}.svg"), &sh.svg(&format!("Bullroarer, {}", name.replace('-', " ")), &desc))?;
        }
    }
    Ok(rc)
}

fn main() {
    std::process::exit(match run() {
        Ok(rc) => rc,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    });
}
