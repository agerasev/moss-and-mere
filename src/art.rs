//! Original, deterministic pixel art for Moss & Mere. All pixels are drawn at
//! their native resolution so nearest-neighbour presentation stays crisp.

type Color = [u8; 4];

pub struct PixelArt {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<Color>,
}

const CLEAR: Color = [0, 0, 0, 0];
const INK: Color = [37, 48, 43, 255];
const SHADOW: Color = [27, 42, 33, 85];
const BARK: Color = [87, 68, 49, 255];
const BARK_LIGHT: Color = [125, 95, 64, 255];

impl PixelArt {
    fn new(w: u32, h: u32, color: Color) -> Self {
        Self {
            width: w,
            height: h,
            pixels: vec![color; (w * h) as usize],
        }
    }

    fn dot(&mut self, x: i32, y: i32, color: Color) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            self.pixels[(y as u32 * self.width + x as u32) as usize] = color;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.dot(xx, yy, color);
            }
        }
    }

    fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            self.dot(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e = 2 * err;
            if e >= dy {
                err += dy;
                x0 += sx;
            }
            if e <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, color: Color) {
        for y in -ry..=ry {
            for x in -rx..=rx {
                if x * x * ry * ry + y * y * rx * rx <= rx * rx * ry * ry {
                    self.dot(cx + x, cy + y, color);
                }
            }
        }
    }

    fn poly(&mut self, points: &[(i32, i32)], color: Color) {
        let min_y = points.iter().map(|p| p.1).min().unwrap_or(0);
        let max_y = points.iter().map(|p| p.1).max().unwrap_or(0);
        for y in min_y..=max_y {
            let mut crossings = Vec::new();
            for i in 0..points.len() {
                let (x1, y1) = points[i];
                let (x2, y2) = points[(i + 1) % points.len()];
                if (y1 <= y && y2 > y) || (y2 <= y && y1 > y) {
                    crossings.push(x1 + (y - y1) * (x2 - x1) / (y2 - y1));
                }
            }
            crossings.sort_unstable();
            for pair in crossings.as_chunks::<2>().0 {
                self.rect(pair[0], y, pair[1] - pair[0] + 1, 1, color);
            }
        }
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    [r, g, b, 255]
}
fn random(seed: u32) -> u32 {
    let mut n = seed.wrapping_mul(747796405).wrapping_add(2891336453);
    n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
    (n >> 22) ^ n
}

/// Wilderness creatures use the same bottom-centre ground anchor as people.
/// Kinds are moss slime (0) and briar wolf (1); facing and steps match `person`.
pub fn monster(kind: u8, facing: u8, step: u8) -> PixelArt {
    if kind == 2 {
        guardian(facing % 4, step % 3)
    } else if kind == 0 {
        moss_slime(facing % 4, step % 3)
    } else {
        briar_wolf(facing % 4, step % 3)
    }
}

fn moss_slime(facing: u8, step: u8) -> PixelArt {
    let mut a = PixelArt::new(28, 24, CLEAR);
    let outline = rgb(48, 65, 42);
    let dark = rgb(77, 99, 48);
    let middle = rgb(129, 149, 62);
    let light = rgb(175, 185, 83);
    let shine = rgb(211, 219, 129);
    let (rise, width) = match step {
        1 => (-2, -1),
        2 => (1, 1),
        _ => (0, 0),
    };
    a.ellipse(14, 22, 10, 1, SHADOW);
    a.ellipse(14, 16 + rise, 11 + width, 6 - rise / 2, outline);
    a.ellipse(14, 12 + rise, 8, 7 - rise / 2, outline);
    a.ellipse(14, 16 + rise, 10 + width, 5 - rise / 2, dark);
    a.ellipse(14, 12 + rise, 7, 6 - rise / 2, middle);
    a.ellipse(13, 15 + rise, 9 + width, 4, middle);
    a.ellipse(11, 11 + rise, 5, 4, light);
    a.rect(8, 10 + rise, 2, 3, shine);
    a.rect(10, 8 + rise, 3, 1, shine);
    a.rect(5 - width, 18 + rise, 4, 2, dark);
    a.rect(19 + width, 17 + rise, 3, 2, dark);
    a.rect(10, 20 + rise, 9, 1, rgb(111, 126, 54));
    a.dot(19, 10 + rise, light);
    a.dot(21, 16 + rise, shine);
    // A little sprig identifies these as forest creatures even when seen behind.
    a.line(14, 7 + rise, 15, 3 + rise, outline);
    a.poly(
        &[(14, 5 + rise), (9, 2 + rise), (10, 5 + rise)],
        rgb(66, 101, 54),
    );
    a.poly(
        &[(15, 5 + rise), (19, 1 + rise), (20, 4 + rise)],
        rgb(99, 132, 62),
    );
    a.dot(18, 3 + rise, rgb(164, 183, 98));
    if facing != 3 {
        let eye_shift = match facing {
            1 => -3,
            2 => 3,
            _ => 0,
        };
        for x in [10 + eye_shift, 17 + eye_shift] {
            a.rect(x - 1, 12 + rise, 4, 4, outline);
            a.rect(x, 13 + rise, 2, 2, rgb(243, 193, 93));
            a.dot(x, 13 + rise, rgb(255, 229, 155));
        }
        a.rect(13 + eye_shift, 17 + rise, 3, 1, outline);
        a.dot(13 + eye_shift, 18 + rise, light);
    } else {
        a.ellipse(17, 12 + rise, 2, 2, dark);
        a.dot(17, 11 + rise, light);
        a.dot(10, 17 + rise, light);
    }
    a
}

fn briar_wolf(facing: u8, step: u8) -> PixelArt {
    let mut a = PixelArt::new(36, 30, CLEAR);
    let outline = rgb(48, 40, 49);
    let dark = rgb(77, 61, 72);
    let fur = rgb(115, 85, 102);
    let light = rgb(154, 111, 124);
    let mane = rgb(95, 78, 78);
    let horn = rgb(193, 170, 124);
    let eye = rgb(248, 181, 86);
    let bob = if step == 1 { -1 } else { 0 };
    let stride = match step {
        1 => 2,
        2 => -2,
        _ => 0,
    };
    a.ellipse(18, 28, 12, 1, SHADOW);
    if facing == 1 || facing == 2 {
        // Side view is drawn facing right, then mirrored as a complete sprite.
        a.poly(
            &[
                (9, 17 + bob),
                (4, 17 + bob),
                (1, 10 + bob),
                (5, 12 + bob),
                (10, 12 + bob),
            ],
            outline,
        );
        a.poly(
            &[(9, 16 + bob), (5, 15 + bob), (3, 12 + bob), (9, 14 + bob)],
            fur,
        );
        for (x, offset) in [(12, -stride), (24, stride)] {
            a.poly(
                &[
                    (x, 18 + bob),
                    (x + 3, 18 + bob),
                    (x + 2 + offset, 26),
                    (x - 1 + offset, 26),
                ],
                outline,
            );
            a.rect(x + offset, 23, 2, 3, dark);
            a.rect(x - 1 + offset, 26, 5, 2, outline);
        }
        a.ellipse(17, 16 + bob, 10, 6, outline);
        a.ellipse(17, 15 + bob, 9, 5, fur);
        a.rect(11, 12 + bob, 10, 2, light);
        a.poly(
            &[
                (9, 18 + bob),
                (21, 20 + bob),
                (25, 15 + bob),
                (23, 21 + bob),
                (13, 21 + bob),
            ],
            dark,
        );
        for (x, offset) in [(10, stride), (23, -stride)] {
            a.poly(
                &[
                    (x, 18 + bob),
                    (x + 4, 18 + bob),
                    (x + 3 + offset, 26),
                    (x + offset, 27),
                ],
                outline,
            );
            a.poly(
                &[
                    (x + 1, 19 + bob),
                    (x + 3, 19 + bob),
                    (x + 2 + offset, 25),
                    (x + 1 + offset, 25),
                ],
                fur,
            );
            a.rect(x + offset, 26, 5, 2, outline);
            a.rect(x + 2 + offset, 26, 3, 1, light);
        }
        a.poly(
            &[
                (20, 17 + bob),
                (19, 7 + bob),
                (22, 9 + bob),
                (25, 5 + bob),
                (29, 9 + bob),
                (30, 15 + bob),
                (25, 21 + bob),
            ],
            outline,
        );
        a.poly(
            &[
                (21, 15 + bob),
                (22, 8 + bob),
                (26, 7 + bob),
                (29, 11 + bob),
                (28, 17 + bob),
                (24, 19 + bob),
            ],
            mane,
        );
        a.ellipse(27, 11 + bob, 5, 4, outline);
        a.ellipse(27, 11 + bob, 4, 3, fur);
        a.poly(
            &[
                (23, 9 + bob),
                (22, 3 + bob),
                (26, 6 + bob),
                (28, 3 + bob),
                (29, 9 + bob),
            ],
            outline,
        );
        a.line(23, 4 + bob, 25, 7 + bob, horn);
        a.line(28, 4 + bob, 28, 7 + bob, light);
        a.rect(29, 12 + bob, 5, 3, outline);
        a.rect(29, 12 + bob, 4, 2, rgb(179, 143, 128));
        a.rect(33, 11 + bob, 2, 3, outline);
        a.rect(28, 9 + bob, 3, 2, outline);
        a.rect(29, 10 + bob, 2, 1, eye);
        a.dot(31, 15 + bob, horn);
        for (x, y) in [(15, 10), (19, 9), (21, 17)] {
            a.poly(
                &[(x - 1, y + 2 + bob), (x, y - 3 + bob), (x + 2, y + 2 + bob)],
                outline,
            );
            a.line(x, y - 2 + bob, x, y + bob, horn);
        }
        if facing == 1 {
            for y in 0..a.height as usize {
                a.pixels[y * a.width as usize..(y + 1) * a.width as usize].reverse();
            }
        }
    } else {
        let back = facing == 3;
        a.poly(
            &[
                (17, 17 + bob),
                (13, 9 + bob),
                (14, 4 + bob),
                (17, 8 + bob),
                (20, 16 + bob),
            ],
            outline,
        );
        a.line(15, 8 + bob, 18, 16 + bob, light);
        for (x, offset) in [(10, stride), (22, -stride)] {
            a.rect(x, 19 + bob, 4, 7 + offset / 2, outline);
            a.rect(x + 1, 20 + bob, 2, 5 + offset / 2, dark);
            a.rect(x - 1, 25 + offset / 2, 5, 2, outline);
        }
        a.ellipse(18, 17 + bob, 8, 8, outline);
        a.ellipse(18, 16 + bob, 7, 7, fur);
        a.ellipse(16, 14 + bob, 4, 5, light);
        for (x, offset) in [(12, -stride), (21, stride)] {
            a.rect(x, 21 + bob, 3, 5 + offset / 2, outline);
            a.rect(x + 1, 21 + bob, 1, 4 + offset / 2, fur);
            a.rect(x - 1, 26 + offset / 2, 5, 2, outline);
            a.dot(x + 1, 26 + offset / 2, light);
        }
        a.poly(
            &[
                (9, 13 + bob),
                (12, 6 + bob),
                (18, 4 + bob),
                (24, 6 + bob),
                (27, 13 + bob),
                (24, 18 + bob),
                (12, 18 + bob),
            ],
            outline,
        );
        a.poly(
            &[
                (11, 12 + bob),
                (13, 7 + bob),
                (18, 5 + bob),
                (23, 7 + bob),
                (25, 12 + bob),
                (23, 16 + bob),
                (13, 16 + bob),
            ],
            mane,
        );
        a.poly(
            &[
                (11, 9 + bob),
                (10, 2 + bob),
                (15, 5 + bob),
                (21, 5 + bob),
                (26, 2 + bob),
                (25, 9 + bob),
            ],
            outline,
        );
        a.line(11, 3 + bob, 14, 7 + bob, horn);
        a.line(25, 3 + bob, 22, 7 + bob, horn);
        a.ellipse(18, 10 + bob, 6, 5, fur);
        a.rect(15, 6 + bob, 5, 2, light);
        if back {
            a.line(18, 8 + bob, 18, 20 + bob, dark);
            for y in [9, 14, 19] {
                a.poly(
                    &[(16, y + 2 + bob), (18, y - 2 + bob), (20, y + 2 + bob)],
                    outline,
                );
                a.dot(18, y - 1 + bob, horn);
            }
        } else {
            a.rect(13, 10 + bob, 4, 2, outline);
            a.rect(20, 10 + bob, 4, 2, outline);
            a.rect(14, 11 + bob, 2, 1, eye);
            a.rect(21, 11 + bob, 2, 1, eye);
            a.ellipse(18, 14 + bob, 4, 3, rgb(174, 138, 125));
            a.rect(17, 12 + bob, 3, 2, outline);
            a.line(18, 14 + bob, 18, 16 + bob, outline);
            a.dot(15, 16 + bob, horn);
            a.dot(21, 16 + bob, horn);
        }
    }
    a
}

/// Ground tiles: grass, forest floor, path, river, bank, bridge, and farm.
pub fn terrain(kind: u8, variant: u8) -> PixelArt {
    let colors = match kind {
        1 => [
            rgb(70, 101, 69),
            rgb(76, 108, 70),
            rgb(61, 91, 62),
            rgb(101, 116, 69),
        ],
        2 => [
            rgb(177, 157, 107),
            rgb(189, 167, 116),
            rgb(165, 143, 97),
            rgb(199, 179, 132),
        ],
        3 => [
            rgb(55, 119, 130),
            rgb(64, 133, 141),
            rgb(49, 108, 122),
            rgb(106, 163, 163),
        ],
        4 => [
            rgb(177, 176, 128),
            rgb(190, 187, 138),
            rgb(160, 163, 112),
            rgb(202, 196, 149),
        ],
        5 => [
            rgb(140, 105, 67),
            rgb(160, 122, 77),
            rgb(109, 85, 58),
            rgb(179, 142, 94),
        ],
        7 => [
            rgb(99, 110, 108),
            rgb(119, 128, 117),
            rgb(80, 94, 92),
            rgb(143, 146, 127),
        ],
        6 => [
            rgb(112, 91, 58),
            rgb(129, 104, 65),
            rgb(93, 79, 52),
            rgb(151, 117, 68),
        ],
        _ => [
            rgb(108, 139, 79),
            rgb(115, 145, 84),
            rgb(99, 131, 72),
            rgb(139, 159, 94),
        ],
    };
    let mut a = PixelArt::new(16, 16, colors[0]);
    for i in 0..19u32 {
        let n = random(variant as u32 * 391 + kind as u32 * 1783 + i * 97);
        let x = (n % 16) as i32;
        let y = ((n >> 8) % 16) as i32;
        a.rect(
            x,
            y,
            2 + (n % 3) as i32,
            1 + ((n >> 5) % 2) as i32,
            colors[1 + (i as usize % 2)],
        );
    }
    match kind {
        7 => {
            a.rect(0, 0, 16, 1, colors[2]);
            a.rect(0, 8, 16, 1, colors[2]);
            a.rect(variant as i32 % 2 * 8, 0, 1, 8, colors[2]);
            a.rect(8 - variant as i32 % 2 * 8, 8, 1, 8, colors[2]);
            a.line(1, 1, 14, 1, colors[1]);
            if variant == 2 {
                a.line(4, 8, 7, 12, colors[2]);
                a.line(7, 12, 6, 15, colors[2]);
            }
        }
        0 | 1 => {
            for i in 0..3u32 {
                let n = random(variant as u32 * 41 + i * 911);
                let x = 1 + (n % 13) as i32;
                let y = 2 + ((n >> 8) % 12) as i32;
                a.dot(x, y, colors[3]);
                a.dot(x + 1, y - 1, colors[1]);
                a.dot(x - 1, y - 1, colors[2]);
            }
            if kind == 1 {
                a.line(
                    3 + (variant % 6) as i32,
                    11,
                    7 + (variant % 6) as i32,
                    12,
                    rgb(106, 103, 65),
                );
            }
        }
        2 | 4 => {
            a.rect(3 + (variant % 7) as i32, 11, 2, 1, colors[3]);
            a.dot(12, 4 + (variant % 5) as i32, colors[2]);
        }
        3 => {
            let shift = (variant % 4) as i32;
            a.line(2 + shift, 4, 6 + shift, 4, colors[1]);
            a.line(8 - shift, 11, 12 - shift, 11, colors[3]);
            a.dot(13 - shift, 10, colors[1]);
            a.line(0, 15, 3, 15, colors[2]);
        }
        5 => {
            for y in [0, 5, 10, 15] {
                a.rect(0, y, 16, 1, colors[2]);
                a.rect(0, y + 1, 16, 1, colors[3]);
                a.dot(2, y + 2, rgb(74, 72, 55));
                a.dot(13, y + 2, rgb(74, 72, 55));
            }
        }
        6 => {
            for x in [2, 8, 14] {
                a.rect(x - 1, 0, 2, 16, colors[2]);
                for y in [3, 10] {
                    a.line(x, y + 2, x, y - 1, rgb(87, 120, 60));
                    a.line(x - 2, y, x, y + 1, rgb(134, 153, 75));
                    a.line(x, y + 1, x + 2, y - 1, rgb(154, 167, 87));
                }
            }
        }
        _ => {}
    }
    a
}

fn broadleaf(variant: u8) -> PixelArt {
    let mut a = PixelArt::new(40, 52, CLEAR);
    a.ellipse(20, 47, 16, 4, SHADOW);
    a.poly(
        &[(15, 48), (18, 35), (16, 26), (23, 25), (22, 40), (26, 48)],
        BARK,
    );
    a.rect(19, 30, 2, 17, BARK_LIGHT);
    a.line(19, 39, 11, 31, BARK);
    a.line(22, 37, 29, 29, BARK);
    a.line(16, 48, 21, 44, rgb(67, 66, 45));
    let colors = if variant % 4 == 3 {
        [
            rgb(49, 79, 58),
            rgb(62, 103, 65),
            rgb(86, 125, 73),
            rgb(111, 143, 79),
            rgb(145, 163, 96),
        ]
    } else {
        [
            rgb(39, 75, 55),
            rgb(49, 95, 59),
            rgb(68, 115, 63),
            rgb(94, 135, 75),
            rgb(124, 153, 87),
        ]
    };
    // Overlapping leaf masses give the crown a stepped silhouette, with light
    // from the upper left and deep pockets beneath the lowest boughs.
    for (x, y, rx, ry) in [
        (12, 27, 10, 10),
        (27, 27, 11, 11),
        (19, 18, 15, 14),
        (18, 10, 10, 8),
    ] {
        a.ellipse(x, y, rx, ry, colors[0]);
        a.ellipse(x - 1, y - 2, rx - 1, ry - 2, colors[1]);
    }
    for (x, y, rx, ry) in [
        (10, 23, 7, 6),
        (25, 24, 10, 8),
        (14, 14, 9, 8),
        (23, 12, 8, 7),
        (18, 6, 6, 4),
    ] {
        a.ellipse(x, y, rx, ry, colors[2]);
        a.ellipse(x - 2, y - 2, rx - 2, (ry - 2).max(2), colors[3]);
    }
    for i in 0..31u32 {
        let n = random(variant as u32 * 89 + i * 213);
        let x = 5 + (n % 29) as i32;
        let y = 4 + ((n >> 8) % 28) as i32;
        let p = a.pixels[(y * 40 + x) as usize];
        if p == colors[2] || p == colors[3] {
            a.rect(
                x,
                y,
                2 + (n % 2) as i32,
                1,
                if i % 3 == 0 { colors[4] } else { colors[2] },
            );
        }
    }
    a
}

fn pine(variant: u8) -> PixelArt {
    let mut a = PixelArt::new(32, 52, CLEAR);
    a.ellipse(16, 47, 12, 3, SHADOW);
    a.rect(14, 34, 5, 15, BARK);
    a.rect(15, 36, 1, 12, BARK_LIGHT);
    for (top, bottom, half) in [(19, 42, 14), (11, 32, 12), (4, 23, 9), (1, 14, 5)] {
        a.poly(
            &[
                (16, top),
                (16 - half, bottom - 3),
                (8, bottom),
                (16, bottom - 1),
                (24, bottom),
                (16 + half, bottom - 3),
            ],
            rgb(36, 76, 58),
        );
        a.poly(
            &[(15, top), (17, bottom - 4), (16 - half + 2, bottom - 4)],
            rgb(55, 104, 68),
        );
        a.line(14, top + 6, 16 - half + 4, bottom - 5, rgb(83, 125, 79));
        a.line(19, bottom - 7, 16 + half - 3, bottom - 4, rgb(43, 90, 62));
        if variant % 2 == 1 {
            a.line(11, bottom - 5, 14, bottom - 4, rgb(97, 134, 85));
        }
    }
    a
}

fn house(variant: u8) -> PixelArt {
    let mut a = PixelArt::new(72, 72, CLEAR);
    let timber = rgb(85, 66, 49);
    let plaster = if variant.is_multiple_of(2) {
        rgb(219, 198, 151)
    } else {
        rgb(207, 188, 151)
    };
    let roof = if variant % 3 == 1 {
        [
            rgb(117, 83, 52),
            rgb(147, 111, 64),
            rgb(175, 139, 78),
            rgb(192, 157, 91),
        ]
    } else {
        [
            rgb(111, 62, 48),
            rgb(151, 80, 57),
            rgb(178, 105, 70),
            rgb(195, 128, 84),
        ]
    };
    a.ellipse(36, 66, 33, 5, SHADOW);
    a.rect(9, 32, 54, 30, timber);
    a.rect(12, 34, 48, 25, plaster);
    a.rect(51, 34, 10, 25, rgb(183, 161, 122));
    a.rect(10, 59, 52, 6, rgb(113, 116, 101));
    a.rect(10, 59, 52, 2, rgb(154, 151, 125));
    for x in (12..60).step_by(9) {
        a.rect(x, 62, 7, 1, rgb(145, 141, 115));
        a.rect(x + 3, 59, 1, 3, rgb(91, 97, 84));
    }
    for x in [11, 31, 50, 59] {
        a.rect(x, 34, 3, 26, timber);
    }
    a.rect(11, 47, 49, 2, timber);
    a.line(14, 36, 29, 46, timber);
    a.line(34, 46, 48, 36, timber);
    // The chimney rises behind the roof, with an ochre rim and brick joints.
    a.rect(49, 7, 8, 20, rgb(120, 105, 89));
    a.rect(49, 7, 3, 19, rgb(163, 142, 111));
    a.rect(47, 6, 12, 3, rgb(98, 87, 77));
    a.rect(49, 5, 8, 2, rgb(182, 154, 113));
    for y in [11, 16, 21] {
        a.line(50, y, 56, y, rgb(94, 85, 74));
    }
    a.poly(
        &[(5, 35), (18, 12), (54, 12), (68, 35), (66, 39), (6, 39)],
        roof[0],
    );
    a.poly(&[(7, 34), (20, 13), (52, 13), (65, 34)], roof[1]);
    a.poly(&[(8, 33), (20, 14), (36, 14), (36, 34)], roof[2]);
    for y in [17, 22, 27, 32] {
        let edge = 19 - ((y - 14) * 3 / 5);
        a.line(edge, y, 72 - edge, y, roof[0]);
        a.line(edge + 1, y - 1, 70 - edge, y - 1, roof[3]);
        for x in (edge + 3..69 - edge).step_by(7) {
            a.line(x + (y % 2) * 3, y + 1, x + (y % 2) * 3 - 2, y + 4, roof[0]);
        }
    }
    a.rect(18, 11, 37, 2, roof[3]);
    a.rect(5, 35, 63, 3, timber);
    a.rect(8, 38, 56, 2, rgb(65, 58, 47));
    // Shutters, lit glass, and the dark doorway remain legible at game scale.
    for x in [17, 43] {
        a.rect(x - 2, 42, 14, 13, timber);
        a.rect(x, 43, 9, 9, rgb(62, 91, 87));
        a.rect(x + 1, 44, 6, 6, rgb(222, 177, 103));
        a.rect(x + 4, 43, 1, 9, timber);
        a.rect(x, 47, 9, 1, timber);
        a.rect(x - 3, 53, 15, 2, BARK_LIGHT);
        a.rect(x - 3, 55, 14, 3, rgb(112, 81, 52));
        a.rect(x - 2, 54, 12, 2, rgb(76, 117, 65));
        for offset in [0, 5, 9] {
            a.dot(x + offset, 53, rgb(207, 142, 111));
        }
    }
    a.rect(30, 46, 12, 17, timber);
    a.rect(32, 48, 8, 15, rgb(114, 85, 58));
    a.rect(34, 48, 1, 14, rgb(137, 101, 65));
    a.rect(38, 49, 1, 13, rgb(77, 65, 48));
    a.dot(38, 56, rgb(225, 185, 98));
    a.rect(29, 63, 15, 3, rgb(165, 157, 126));
    a.rect(27, 66, 19, 3, rgb(140, 139, 116));
    a.line(28, 66, 44, 66, rgb(184, 173, 139));
    a
}

/// World props. Every prop uses a bottom-centre placement anchor.
pub fn prop(kind: u8, variant: u8) -> PixelArt {
    if kind >= 8 {
        return ruin_prop(kind, variant);
    }
    match kind {
        0 => broadleaf(variant),
        1 => pine(variant),
        2 => house(variant),
        3 => {
            let mut a = PixelArt::new(28, 30, CLEAR);
            a.ellipse(14, 27, 13, 3, SHADOW);
            a.ellipse(14, 23, 10, 5, rgb(101, 113, 102));
            a.rect(4, 20, 21, 4, rgb(120, 130, 113));
            a.ellipse(14, 20, 10, 4, rgb(169, 173, 142));
            a.ellipse(14, 20, 7, 2, rgb(44, 65, 60));
            a.rect(6, 9, 3, 16, BARK);
            a.rect(20, 9, 3, 16, BARK);
            a.rect(7, 10, 1, 14, BARK_LIGHT);
            a.rect(7, 11, 15, 2, BARK_LIGHT);
            a.rect(14, 11, 1, 9, rgb(184, 159, 108));
            a.rect(13, 17, 5, 4, rgb(146, 111, 69));
            a.poly(&[(1, 10), (8, 2), (21, 2), (27, 10)], rgb(130, 77, 52));
            a.line(5, 7, 24, 7, rgb(177, 109, 69));
            a.rect(1, 10, 26, 2, rgb(82, 61, 44));
            a.line(6, 25, 11, 26, rgb(168, 171, 142));
            a.dot(18, 25, rgb(74, 88, 81));
            a
        }
        4 => {
            let mut a = PixelArt::new(20, 24, CLEAR);
            a.ellipse(10, 22, 7, 2, SHADOW);
            a.rect(9, 5, 3, 18, BARK);
            a.rect(9, 12, 1, 10, BARK_LIGHT);
            a.poly(
                &[(2, 3), (15, 3), (19, 7), (15, 11), (2, 11)],
                rgb(91, 70, 48),
            );
            a.poly(
                &[(3, 4), (14, 4), (17, 7), (14, 9), (3, 9)],
                rgb(177, 144, 90),
            );
            a.rect(5, 6, 8, 1, rgb(109, 84, 51));
            a.rect(5, 8, 5, 1, rgb(126, 98, 59));
            a
        }
        5 => {
            let mut a = PixelArt::new(20, 16, CLEAR);
            a.ellipse(10, 13, 9, 2, SHADOW);
            a.poly(
                &[
                    (1, 11),
                    (4, 5),
                    (10, 2),
                    (16, 5),
                    (19, 11),
                    (14, 14),
                    (5, 14),
                ],
                rgb(95, 110, 99),
            );
            a.poly(
                &[(2, 10), (5, 5), (10, 3), (15, 5), (12, 10)],
                rgb(147, 151, 125),
            );
            a.poly(&[(5, 5), (10, 3), (13, 5), (9, 7)], rgb(168, 168, 137));
            a.line(11, 9, 14, 12, rgb(117, 130, 109));
            a.rect(3, 12, 5, 2, rgb(95, 128, 72));
            a
        }
        6 => {
            let mut a = PixelArt::new(16, 16, CLEAR);
            let petals = if variant.is_multiple_of(2) {
                rgb(218, 183, 111)
            } else {
                rgb(190, 155, 183)
            };
            for (x, y) in [(3, 8), (10, 4), (12, 11), (6, 12)] {
                a.line(x, y, x, y + 3, rgb(71, 111, 60));
                a.dot(x - 1, y + 2, rgb(123, 154, 80));
                a.rect(x - 1, y, 3, 1, petals);
                a.rect(x, y - 1, 1, 3, petals);
                a.dot(x, y, rgb(240, 211, 144));
            }
            a
        }
        _ => {
            let mut a = PixelArt::new(32, 20, CLEAR);
            a.ellipse(16, 18, 15, 2, SHADOW);
            a.rect(0, 7, 32, 3, rgb(133, 105, 64));
            a.rect(0, 14, 32, 3, rgb(111, 91, 57));
            a.rect(0, 7, 32, 1, rgb(167, 135, 83));
            for x in [3, 25] {
                a.rect(x, 4, 4, 15, BARK);
                a.rect(x, 4, 2, 14, BARK_LIGHT);
                a.rect(x + 1, 2, 2, 2, rgb(156, 122, 75));
                a.dot(x + 2, 8, rgb(72, 72, 54));
            }
            a
        }
    }
}

/// Human sprites: down, left, right, up. Steps 1/2 alternate feet and arms.
pub fn person(variant: u8, facing: u8, step: u8) -> PixelArt {
    let mut a = PixelArt::new(20, 28, CLEAR);
    let palettes = [
        (
            rgb(62, 111, 112),
            rgb(89, 145, 139),
            rgb(196, 151, 111),
            rgb(77, 56, 42),
        ),
        (
            rgb(155, 96, 68),
            rgb(190, 130, 87),
            rgb(216, 173, 127),
            rgb(89, 63, 43),
        ),
        (
            rgb(101, 103, 137),
            rgb(136, 137, 164),
            rgb(145, 99, 73),
            rgb(49, 44, 39),
        ),
        (
            rgb(126, 135, 76),
            rgb(158, 161, 97),
            rgb(205, 157, 111),
            rgb(152, 116, 64),
        ),
        (
            rgb(167, 134, 93),
            rgb(199, 164, 113),
            rgb(118, 80, 63),
            rgb(46, 43, 39),
        ),
        (
            rgb(114, 88, 122),
            rgb(154, 117, 153),
            rgb(226, 183, 141),
            rgb(190, 175, 144),
        ),
        (
            rgb(70, 118, 90),
            rgb(105, 149, 107),
            rgb(177, 126, 88),
            rgb(59, 48, 37),
        ),
        (
            rgb(170, 151, 109),
            rgb(210, 190, 140),
            rgb(226, 183, 141),
            rgb(123, 76, 46),
        ),
        (
            rgb(127, 81, 65),
            rgb(174, 119, 81),
            rgb(162, 114, 81),
            rgb(183, 177, 151),
        ),
    ];
    let (coat, light, skin, hair) = palettes[(variant as usize) % palettes.len()];
    let skin_light = [
        skin[0].saturating_add(14),
        skin[1].saturating_add(12),
        skin[2].saturating_add(9),
        255,
    ];
    let walking = step % 3;
    let bob = if walking == 0 { 0 } else { -1 };
    let left_foot = if walking == 1 { -1 } else { 0 };
    let right_foot = if walking == 2 { -1 } else { 0 };
    a.ellipse(10, 26, 6, 1, SHADOW);
    a.rect(6, 19 + bob, 3, 6 + left_foot, rgb(67, 67, 54));
    a.rect(11, 19 + bob, 3, 6 + right_foot, rgb(57, 61, 52));
    a.rect(5, 24 + left_foot, 4, 2, rgb(71, 53, 41));
    a.rect(11, 24 + right_foot, 4, 2, rgb(62, 48, 39));
    let side = facing == 1 || facing == 2;
    a.poly(
        &[
            (7, 10 + bob),
            (12, 10 + bob),
            (15, 14 + bob),
            (15, 21 + bob),
            (5, 21 + bob),
            (5, 14 + bob),
        ],
        INK,
    );
    a.rect(6, 12 + bob, 8, 9, coat);
    a.rect(6, 13 + bob, 3, 7, light);
    a.rect(6, 19 + bob, 8, 2, rgb(96, 70, 46));
    a.dot(10, 19 + bob, rgb(201, 173, 102));
    if facing != 3 {
        a.rect(5, 3 + bob, 10, 9, INK);
        a.rect(6, 4 + bob, 8, 7, skin);
        a.rect(7, 6 + bob, 4, 3, skin_light);
        a.rect(7, 10 + bob, 5, 2, skin);
        a.rect(6, 2 + bob, 8, 4, hair);
        a.rect(5, 4 + bob, 2, 4, hair);
        a.rect(13, 4 + bob, 2, 3, hair);
        a.rect(
            7,
            2 + bob,
            5,
            1,
            [
                hair[0].saturating_add(19),
                hair[1].saturating_add(17),
                hair[2].saturating_add(11),
                255,
            ],
        );
        if side {
            let eye_x = if facing == 1 { 6 } else { 13 };
            a.dot(eye_x, 7 + bob, INK);
            a.dot(if facing == 1 { 5 } else { 14 }, 8 + bob, skin_light);
            a.rect(if facing == 1 { 11 } else { 6 }, 5 + bob, 3, 5, hair);
        } else {
            a.dot(8, 7 + bob, INK);
            a.dot(12, 7 + bob, INK);
            a.rect(9, 10 + bob, 2, 1, rgb(153, 103, 80));
        }
    } else {
        a.rect(5, 3 + bob, 10, 8, INK);
        a.rect(6, 2 + bob, 8, 8, hair);
        a.rect(
            6,
            4 + bob,
            3,
            5,
            [
                hair[0].saturating_add(13),
                hair[1].saturating_add(11),
                hair[2].saturating_add(8),
                255,
            ],
        );
        a.rect(8, 10 + bob, 4, 2, skin);
    }
    let arm_swing = if walking == 1 {
        1
    } else if walking == 2 {
        -1
    } else {
        0
    };
    if side {
        let arm_x = if facing == 1 { 11 } else { 6 };
        a.rect(arm_x, 13 + bob, 3, 5 + arm_swing, light);
        a.rect(arm_x, 18 + bob + arm_swing, 3, 2, skin);
    } else {
        a.rect(4, 13 + bob + arm_swing, 2, 5, light);
        a.rect(14, 13 + bob - arm_swing, 2, 5, coat);
        a.rect(4, 18 + bob + arm_swing, 2, 2, skin_light);
        a.rect(14, 18 + bob - arm_swing, 2, 2, skin);
    }
    if variant == 0 {
        if facing == 3 || side {
            let x = if facing == 1 {
                12
            } else if facing == 2 {
                3
            } else {
                7
            };
            a.rect(x, 12 + bob, 6, 8, rgb(76, 60, 44));
            a.rect(x + 1, 13 + bob, 4, 6, rgb(139, 101, 61));
            a.rect(x + 1, 13 + bob, 4, 2, rgb(163, 124, 77));
            a.dot(x + 3, 15 + bob, rgb(216, 178, 107));
        } else {
            a.line(7, 12 + bob, 12, 18 + bob, rgb(163, 122, 76));
        }
        a.rect(6, 11 + bob, 8, 2, rgb(165, 72, 49));
        a.rect(7, 11 + bob, 6, 1, rgb(209, 106, 69));
        a.rect(
            if facing == 3 { 12 } else { 6 },
            13 + bob,
            2,
            4,
            rgb(165, 72, 49),
        );
    } else if variant % 3 == 1 {
        a.rect(5, 3 + bob, 10, 2, rgb(173, 149, 93));
        a.rect(7, 1 + bob, 6, 3, rgb(199, 178, 117));
    } else if variant % 3 == 2 && !side && facing == 0 {
        a.rect(8, 14 + bob, 5, 6, rgb(207, 192, 150));
    }
    a
}

fn guardian(facing: u8, step: u8) -> PixelArt {
    let mut a = PixelArt::new(48, 52, CLEAR);
    let dark = rgb(49, 62, 61);
    let stone = rgb(102, 117, 106);
    let light = rgb(156, 159, 126);
    let moss = rgb(79, 120, 73);
    let ember = rgb(244, 153, 63);
    let bob = i32::from(step == 1);
    let stride = if step == 1 {
        2
    } else if step == 2 {
        -2
    } else {
        0
    };
    a.ellipse(24, 48, 21, 3, SHADOW);
    for (x, d) in [(12, stride), (28, -stride)] {
        a.rect(x + d, 34, 10, 15, dark);
        a.rect(x + d + 1, 35, 7, 12, stone);
        a.rect(x + d, 46, 12, 3, light);
        a.line(x + d + 4, 36, x + d + 3, 43, dark);
    }
    a.poly(&[(9, 18 + bob), (39, 18 + bob), (36, 38), (12, 38)], dark);
    a.poly(&[(11, 20 + bob), (36, 20 + bob), (33, 36), (14, 36)], stone);
    a.rect(13, 20 + bob, 4, 14, light);
    a.line(19, 21 + bob, 25, 26 + bob, dark);
    a.line(25, 26 + bob, 22, 35, dark);
    for (x, dy) in [(3, stride), (37, -stride)] {
        a.rect(x, 20 + dy, 9, 19, dark);
        a.rect(x + 1, 21 + dy, 6, 13, stone);
        a.rect(x + 2, 22 + dy, 3, 8, light);
        a.rect(x - 1, 34 + dy, 11, 9, dark);
        a.rect(x, 35 + dy, 8, 6, stone);
        a.rect(x, 19 + dy, 8, 3, moss);
    }
    a.rect(14, 5 + bob, 21, 17, dark);
    a.rect(16, 6 + bob, 17, 13, stone);
    a.rect(16, 6 + bob, 5, 9, light);
    a.rect(12, 4 + bob, 23, 4, moss);
    a.rect(19, 1 + bob, 3, 5, moss);
    a.rect(30, 2 + bob, 2, 4, moss);
    if facing != 3 {
        let dx = if facing == 1 {
            -2
        } else if facing == 2 {
            2
        } else {
            0
        };
        a.rect(18 + dx, 11 + bob, 5, 3, dark);
        a.rect(26 + dx, 11 + bob, 5, 3, dark);
        a.rect(19 + dx, 11 + bob, 3, 2, ember);
        a.rect(27 + dx, 11 + bob, 3, 2, ember);
        a.poly(
            &[
                (24, 23 + bob),
                (29, 28 + bob),
                (24, 33 + bob),
                (19, 28 + bob),
            ],
            dark,
        );
        a.poly(
            &[
                (24, 25 + bob),
                (27, 28 + bob),
                (24, 31 + bob),
                (21, 28 + bob),
            ],
            ember,
        );
        a.dot(24, 27 + bob, rgb(255, 229, 154));
    }
    a.rect(31, 28, 4, 4, moss);
    a.rect(28, 34, 6, 2, moss);
    a
}

fn ruin_prop(kind: u8, variant: u8) -> PixelArt {
    let stone = rgb(94, 107, 102);
    let light = rgb(143, 151, 130);
    let dark = rgb(55, 70, 69);
    let gold = rgb(209, 167, 79);
    match kind {
        8 | 11 => {
            let mut a = PixelArt::new(16, 32, CLEAR);
            a.rect(0, 1, 16, 31, dark);
            a.rect(0, 1, 16, 7, light);
            a.rect(0, 8, 16, 24, stone);
            a.line(0, 1, 15, 1, rgb(177, 174, 145));
            for y in [8, 16, 24, 31] {
                a.line(0, y, 15, y, dark);
            }
            for (x, y) in [(7, 9), (2, 17), (11, 25)] {
                a.rect(x, y, 1, 7, dark);
            }
            if kind == 11 {
                a.poly(&[(8, 9), (12, 15), (8, 22), (4, 16)], rgb(123, 164, 110));
                a.line(8, 12, 8, 25, gold);
            } else if variant.is_multiple_of(2) {
                a.rect(1, 2, 6, 2, rgb(85, 119, 73));
                a.rect(2, 4, 3, 7, rgb(66, 99, 66));
            }
            a
        }
        9 => {
            let mut a = PixelArt::new(16, 28, CLEAR);
            a.rect(7, 13, 3, 14, BARK);
            a.rect(6, 12, 5, 5, dark);
            a.poly(
                &[(8, 1), (13, 10), (10, 15), (5, 14), (3, 9), (6, 6)],
                rgb(226, 120, 54),
            );
            a.poly(&[(8, 5), (10, 11), (8, 14), (5, 11)], rgb(255, 200, 96));
            a.rect(7, 10, 2, 4, rgb(255, 235, 166));
            a
        }
        10 => {
            let mut a = PixelArt::new(24, 20, CLEAR);
            a.ellipse(12, 18, 11, 2, SHADOW);
            a.rect(4, 14, 16, 4, BARK);
            a.rect(8, 6, 8, 9, dark);
            a.poly(&[(1, 3), (23, 3), (19, 8), (5, 8)], stone);
            a.rect(4, 2, 16, 2, light);
            a.rect(6, 13, 13, 2, stone);
            a.rect(16, 0, 3, 3, gold);
            a
        }
        12 => {
            let mut a = PixelArt::new(24, 22, CLEAR);
            a.ellipse(12, 20, 11, 2, SHADOW);
            a.rect(2, 7, 20, 13, dark);
            a.rect(3, 8, 18, 10, BARK);
            a.rect(3, 8, 18, 2, BARK_LIGHT);
            a.rect(5, 8, 2, 10, gold);
            a.rect(17, 8, 2, 10, gold);
            if variant == 0 {
                a.poly(&[(2, 7), (5, 3), (19, 3), (22, 7)], BARK_LIGHT);
                a.rect(2, 9, 20, 2, dark);
                a.rect(10, 9, 4, 5, gold);
                a.dot(12, 11, dark);
            } else {
                a.rect(3, 2, 18, 5, BARK_LIGHT);
                a.rect(4, 7, 16, 5, INK);
            }
            a
        }
        _ => {
            let mut a = PixelArt::new(24, 36, CLEAR);
            a.ellipse(12, 33, 11, 2, SHADOW);
            a.rect(3, 29, 18, 5, dark);
            a.rect(5, 23, 14, 8, stone);
            a.ellipse(12, 23, 10, 3, light);
            a.ellipse(12, 22, 7, 2, dark);
            a.line(10, 27, 14, 27, gold);
            if variant == 0 {
                a.poly(&[(12, 2), (18, 11), (12, 20), (6, 11)], rgb(166, 73, 41));
                a.poly(&[(12, 3), (15, 10), (12, 17), (9, 10)], rgb(244, 164, 61));
                a.line(12, 6, 12, 12, rgb(255, 230, 150));
            }
            a
        }
    }
}
