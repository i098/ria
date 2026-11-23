// Ria original, no Ardia source.

//! `Math.sin`, `Math.cos`, `Math.atan` and `Math.atan2` as V8 computes them (`src/base/ieee754.cc`,
//! from FreeBSD fdlibm). The system libm differs from V8 in the last bit for about 3% of inputs,
//! so `theme` calls these to match Ardia bit for bit.

#![allow(
    clippy::approx_constant,
    clippy::excessive_precision,
    clippy::explicit_counter_loop,
    clippy::needless_range_loop,
    clippy::unreadable_literal
)]

const fn hi(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

const fn lo(x: f64) -> u32 {
    x.to_bits() as u32
}

fn with_hi(x: f64, h: i32) -> f64 {
    f64::from_bits(((h as u32 as u64) << 32) | u64::from(lo(x)))
}

/// `x * 2^n`, exact while the result stays a normal number.
fn scalbn(mut x: f64, mut n: i32) -> f64 {
    while n > 1000 {
        x *= f64::from_bits((1023 + 1000_u64) << 52);
        n -= 1000;
    }
    while n < -1000 {
        x *= f64::from_bits((1023 - 1000_u64) << 52);
        n += 1000;
    }
    x * f64::from_bits(((1023 + i64::from(n)) as u64) << 52)
}

const S: [f64; 6] = [
    -1.66666666666666324348e-01,
    8.33333333332248946124e-03,
    -1.98412698298579493134e-04,
    2.75573137070700676789e-06,
    -2.50507602534068634195e-08,
    1.58969099521155010221e-10,
];

fn kernel_sin(x: f64, y: f64, iy: bool) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix < 0x3e400000 && x as i32 == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let r = S[1] + z * (S[2] + z * (S[3] + z * (S[4] + z * S[5])));
    if !iy {
        x + v * (S[0] + z * r)
    } else {
        x - ((z * (0.5 * y - v * r) - y) - v * S[0])
    }
}

const C: [f64; 6] = [
    4.16666666666666019037e-02,
    -1.38888888888741095749e-03,
    2.48015872894767294178e-05,
    -2.75573143513906633035e-07,
    2.08757232129817482790e-09,
    -1.13596475577881948265e-11,
];

fn kernel_cos(x: f64, y: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix < 0x3e400000 && x as i32 == 0 {
        return 1.0;
    }
    let z = x * x;
    let r = z * (C[0] + z * (C[1] + z * (C[2] + z * (C[3] + z * (C[4] + z * C[5])))));
    if ix < 0x3FD33333 {
        1.0 - (0.5 * z - (z * r - x * y))
    } else {
        let qx = if ix > 0x3fe90000 {
            0.28125
        } else {
            f64::from_bits(((ix - 0x00200000) as u64) << 32)
        };
        let hz = 0.5 * z - qx;
        let a = 1.0 - qx;
        a - (hz - (z * r - x * y))
    }
}

const TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

const PIO2: [f64; 8] = [
    1.57079625129699707031e+00,
    7.54978941586159635335e-08,
    5.39030252995776476554e-15,
    3.28200341580791294123e-22,
    1.27065575308067607349e-29,
    1.22933308981111328932e-36,
    2.73370053816464559624e-44,
    2.16741683877804819444e-51,
];

const TWO24: f64 = 16777216.0;
const TWON24: f64 = 5.96046447753906250000e-08;

/// `__kernel_rem_pio2` for double precision (`jk` = 4).
fn kernel_rem_pio2(x: &[f64; 3], e0: i32, nx: usize) -> (i32, [f64; 2]) {
    const JK: usize = 4;
    let jx = nx - 1;
    let jv = ((e0 - 3) / 24).max(0);
    let mut q0 = e0 - 24 * (jv + 1);
    let mut f = [0.0_f64; 20];
    let mut q = [0.0_f64; 20];
    let mut iq = [0_i32; 20];
    let mut fq = [0.0_f64; 20];
    let mut j = jv - jx as i32;
    for fi in f.iter_mut().take(jx + JK + 1) {
        *fi = if j < 0 {
            0.0
        } else {
            f64::from(TWO_OVER_PI[j as usize])
        };
        j += 1;
    }
    for i in 0..=JK {
        q[i] = (0..=jx)
            .map(|j| x[j] * f[jx + i - j])
            .fold(0.0, |a, b| a + b);
    }
    let mut jz = JK;
    let (mut z, mut n, mut ih);
    loop {
        z = q[jz];
        let mut j = jz;
        for iqi in iq.iter_mut().take(jz) {
            let fw = f64::from((TWON24 * z) as i32);
            *iqi = (z - TWO24 * fw) as i32;
            z = q[j - 1] + fw;
            j -= 1;
        }
        z = scalbn(z, q0);
        z -= 8.0 * (z * 0.125).floor();
        n = z as i32;
        z -= f64::from(n);
        ih = 0;
        if q0 > 0 {
            let i = iq[jz - 1] >> (24 - q0);
            n += i;
            iq[jz - 1] -= i << (24 - q0);
            ih = iq[jz - 1] >> (23 - q0);
        } else if q0 == 0 {
            ih = iq[jz - 1] >> 23;
        } else if z >= 0.5 {
            ih = 2;
        }
        if ih > 0 {
            n += 1;
            let mut carry = false;
            for iqi in iq.iter_mut().take(jz) {
                let j = *iqi;
                if !carry {
                    if j != 0 {
                        carry = true;
                        *iqi = 0x1000000 - j;
                    }
                } else {
                    *iqi = 0xffffff - j;
                }
            }
            match q0 {
                1 => iq[jz - 1] &= 0x7fffff,
                2 => iq[jz - 1] &= 0x3fffff,
                _ => {}
            }
            if ih == 2 {
                z = 1.0 - z;
                if carry {
                    z -= scalbn(1.0, q0);
                }
            }
        }
        if z == 0.0 {
            let mut j = 0;
            for i in (JK..jz).rev() {
                j |= iq[i];
            }
            if j == 0 {
                let mut k = 1;
                while iq[JK - k] == 0 {
                    k += 1;
                }
                for i in jz + 1..=jz + k {
                    f[jx + i] = f64::from(TWO_OVER_PI[jv as usize + i]);
                    q[i] = (0..=jx)
                        .map(|j| x[j] * f[jx + i - j])
                        .fold(0.0, |a, b| a + b);
                }
                jz += k;
                continue;
            }
        }
        break;
    }
    if z == 0.0 {
        jz -= 1;
        q0 -= 24;
        while iq[jz] == 0 {
            jz -= 1;
            q0 -= 24;
        }
    } else {
        z = scalbn(z, -q0);
        if z >= TWO24 {
            let fw = f64::from((TWON24 * z) as i32);
            iq[jz] = (z - TWO24 * fw) as i32;
            jz += 1;
            q0 += 24;
            iq[jz] = fw as i32;
        } else {
            iq[jz] = z as i32;
        }
    }
    let mut fw = scalbn(1.0, q0);
    for i in (0..=jz).rev() {
        q[i] = fw * f64::from(iq[i]);
        fw *= TWON24;
    }
    for i in (0..=jz).rev() {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= JK && k <= jz - i {
            fw += PIO2[k] * q[i + k];
            k += 1;
        }
        fq[jz - i] = fw;
    }
    let mut fw = 0.0;
    for i in (0..=jz).rev() {
        fw += fq[i];
    }
    let y0 = if ih == 0 { fw } else { -fw };
    let mut fw = fq[0] - fw;
    for fqi in fq.iter().take(jz + 1).skip(1) {
        fw += fqi;
    }
    let y1 = if ih == 0 { fw } else { -fw };
    (n & 7, [y0, y1])
}

const NPIO2_HW: [i32; 32] = [
    0x3ff921fb, 0x400921fb, 0x4012d97c, 0x401921fb, 0x401f6a7a, 0x4022d97c, 0x4025fdbb, 0x402921fb,
    0x402c463a, 0x402f6a7a, 0x4031475c, 0x4032d97c, 0x40346b9c, 0x4035fdbb, 0x40378fdb, 0x403921fb,
    0x403ab41b, 0x403c463a, 0x403dd85a, 0x403f6a7a, 0x40407e4c, 0x4041475c, 0x4042106c, 0x4042d97c,
    0x4043a28c, 0x40446b9c, 0x404534ac, 0x4045fdbb, 0x4046c6cb, 0x40478fdb, 0x404858eb, 0x404921fb,
];

const INVPIO2: f64 = 6.36619772367581382433e-01;
const PIO2_1: f64 = 1.57079632673412561417e+00;
const PIO2_1T: f64 = 6.07710050650619224932e-11;
const PIO2_2: f64 = 6.07710050630396597660e-11;
const PIO2_2T: f64 = 2.02226624879595063154e-21;
const PIO2_3: f64 = 2.02226624871116645580e-21;
const PIO2_3T: f64 = 8.47842766036889956997e-32;

/// `__ieee754_rem_pio2`: `x` = `n` * pi/2 + `y[0]` + `y[1]`; returns `(n, y)`. `x` is finite.
fn rem_pio2(x: f64) -> (i32, [f64; 2]) {
    let hx = hi(x);
    let ix = hx & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return (0, [x, 0.0]);
    }
    if ix < 0x4002d97c {
        let (z, sign) = if hx > 0 {
            (x - PIO2_1, 1.0)
        } else {
            (x + PIO2_1, -1.0)
        };
        let (y0, y1) = if ix != 0x3ff921fb {
            let y0 = z - sign * PIO2_1T;
            (y0, (z - y0) - sign * PIO2_1T)
        } else {
            let z = z - sign * PIO2_2;
            let y0 = z - sign * PIO2_2T;
            (y0, (z - y0) - sign * PIO2_2T)
        };
        return (sign as i32, [y0, y1]);
    }
    if ix <= 0x413921fb {
        let t = x.abs();
        let n = (t * INVPIO2 + 0.5) as i32;
        let fn_ = f64::from(n);
        let mut r = t - fn_ * PIO2_1;
        let mut w = fn_ * PIO2_1T;
        let mut y0;
        if n < 32 && ix != NPIO2_HW[n as usize - 1] {
            y0 = r - w;
        } else {
            let j = ix >> 20;
            y0 = r - w;
            let mut i = j - ((hi(y0) >> 20) & 0x7ff);
            if i > 16 {
                let t = r;
                w = fn_ * PIO2_2;
                r = t - w;
                w = fn_ * PIO2_2T - ((t - r) - w);
                y0 = r - w;
                i = j - ((hi(y0) >> 20) & 0x7ff);
                if i > 49 {
                    let t = r;
                    w = fn_ * PIO2_3;
                    r = t - w;
                    w = fn_ * PIO2_3T - ((t - r) - w);
                    y0 = r - w;
                }
            }
        }
        let y1 = (r - y0) - w;
        return if hx < 0 {
            (-n, [-y0, -y1])
        } else {
            (n, [y0, y1])
        };
    }
    let e0 = (ix >> 20) - 1046;
    let mut z = with_hi(x, ix - (e0 << 20));
    let mut tx = [0.0; 3];
    for t in tx.iter_mut().take(2) {
        *t = f64::from(z as i32);
        z = (z - *t) * TWO24;
    }
    tx[2] = z;
    let mut nx = 3;
    while tx[nx - 1] == 0.0 {
        nx -= 1;
    }
    let (n, y) = kernel_rem_pio2(&tx, e0, nx);
    if hx < 0 { (-n, [-y[0], -y[1]]) } else { (n, y) }
}

/// V8's `Math.sin`.
pub fn sin(x: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return kernel_sin(x, 0.0, false);
    }
    if ix >= 0x7ff00000 {
        return f64::NAN;
    }
    let (n, y) = rem_pio2(x);
    match n & 3 {
        0 => kernel_sin(y[0], y[1], true),
        1 => kernel_cos(y[0], y[1]),
        2 => -kernel_sin(y[0], y[1], true),
        _ => -kernel_cos(y[0], y[1]),
    }
}

/// V8's `Math.cos`.
pub fn cos(x: f64) -> f64 {
    let ix = hi(x) & 0x7fffffff;
    if ix <= 0x3fe921fb {
        return kernel_cos(x, 0.0);
    }
    if ix >= 0x7ff00000 {
        return f64::NAN;
    }
    let (n, y) = rem_pio2(x);
    match n & 3 {
        0 => kernel_cos(y[0], y[1]),
        1 => -kernel_sin(y[0], y[1], true),
        2 => -kernel_cos(y[0], y[1]),
        _ => kernel_sin(y[0], y[1], true),
    }
}

const ATAN_HI: [f64; 4] = [
    4.63647609000806093515e-01,
    7.85398163397448278999e-01,
    9.82793723247329054082e-01,
    1.57079632679489655800e+00,
];
const ATAN_LO: [f64; 4] = [
    2.26987774529616870924e-17,
    3.06161699786838301793e-17,
    1.39033110312309984516e-17,
    6.12323399573676603587e-17,
];
const AT: [f64; 11] = [
    3.33333333333329318027e-01,
    -1.99999999998764832476e-01,
    1.42857142725034663711e-01,
    -1.11111104054623557880e-01,
    9.09088713343650656196e-02,
    -7.69187620504482999495e-02,
    6.66107313738753120669e-02,
    -5.83357013379057348645e-02,
    4.97687799461593236017e-02,
    -3.65315727442169155270e-02,
    1.62858201153657823623e-02,
];

/// V8's `Math.atan`.
pub fn atan(x: f64) -> f64 {
    let hx = hi(x);
    let ix = hx & 0x7fffffff;
    let id;
    let mut x = x;
    if ix >= 0x44100000 {
        if x.is_nan() {
            return x + x;
        }
        return if hx > 0 {
            ATAN_HI[3] + ATAN_LO[3]
        } else {
            -ATAN_HI[3] - ATAN_LO[3]
        };
    }
    if ix < 0x3fdc0000 {
        if ix < 0x3e200000 {
            return x;
        }
        id = -1;
    } else {
        x = x.abs();
        if ix < 0x3ff30000 {
            if ix < 0x3fe60000 {
                id = 0;
                x = (2.0 * x - 1.0) / (2.0 + x);
            } else {
                id = 1;
                x = (x - 1.0) / (x + 1.0);
            }
        } else if ix < 0x40038000 {
            id = 2;
            x = (x - 1.5) / (1.0 + 1.5 * x);
        } else {
            id = 3;
            x = -1.0 / x;
        }
    }
    let z = x * x;
    let w = z * z;
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
    if id < 0 {
        return x - x * (s1 + s2);
    }
    let id = id as usize;
    let z = ATAN_HI[id] - ((x * (s1 + s2) - ATAN_LO[id]) - x);
    if hx < 0 { -z } else { z }
}

const PI: f64 = 3.1415926535897931160e+00;
const PI_LO: f64 = 1.2246467991473531772e-16;
const PI_O_2: f64 = 1.5707963267948965580e+00;
const PI_O_4: f64 = 7.8539816339744827900e-01;
const THREE_PI_O_4: f64 = 2.3561944901923448370e+00;

/// V8's `Math.atan2(y, x)`.
pub fn atan2(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let (hx, hy) = (hi(x), hi(y));
    let (ix, iy) = (hx & 0x7fffffff, hy & 0x7fffffff);
    let (xzero, yzero) = (x == 0.0, y == 0.0);
    if x == 1.0 {
        return atan(y);
    }
    let mut m = ((hy >> 31) & 1) | ((hx >> 30) & 2);
    if yzero {
        return match m {
            0 | 1 => y,
            2 => PI,
            _ => -PI,
        };
    }
    if xzero {
        return if hy < 0 { -PI_O_2 } else { PI_O_2 };
    }
    if ix == 0x7ff00000 {
        return if iy == 0x7ff00000 {
            [PI_O_4, -PI_O_4, THREE_PI_O_4, -THREE_PI_O_4][m as usize]
        } else {
            [0.0, -0.0, PI, -PI][m as usize]
        };
    }
    if iy == 0x7ff00000 {
        return if hy < 0 { -PI_O_2 } else { PI_O_2 };
    }
    let k = (iy - ix) >> 20;
    let z = if k > 60 {
        m &= 1;
        PI_O_2 + 0.5 * PI_LO
    } else if hx < 0 && k < -60 {
        0.0
    } else {
        atan((y / x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => PI - (z - PI_LO),
        _ => (z - PI_LO) - PI,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values printed by Node 26 (V8 14.6): `Math.sin(x)` etc. for the bit patterns below.
    #[test]
    fn matches_v8_on_samples() {
        assert_eq!(sin(1.0).to_bits(), 0x3FEAED548F090CEE);
        assert_eq!(cos(1.0).to_bits(), 0x3FE14A280FB5068C);
        assert_eq!(atan2(1.0, 2.0).to_bits(), 0x3FDDAC670561BB4F);
        assert_eq!(sin(1e22).to_bits(), 0xBFEB453AB76BF397);
    }
}
