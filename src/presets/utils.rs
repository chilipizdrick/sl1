pub fn scale_u8(i: u8, scale: u8) -> u8 {
    ((i as u16 * scale as u16) >> 8) as u8
    // ((i as u16 * (scale as u16 + 1)) >> 8) as u8
}

pub fn lerp_i8(a: i8, b: i8, frac: u8) -> i8 {
    if b > a {
        let delta = (b as i16 - a as i16) as u8;
        let scaled = scale_u8(delta, frac);
        (a as i16 + scaled as i16) as i8
    } else {
        let delta = (a as i16 - b as i16) as u8;
        let scaled = scale_u8(delta, frac);
        (a as i16 - scaled as i16) as i8
    }
}

pub fn lerp_u8(a: u8, b: u8, frac: u8) -> u8 {
    if b > a {
        let delta = b - a;
        let scaled = scale_u8(delta, frac);
        a + scaled
    } else {
        let delta = a - b;
        let scaled = scale_u8(delta, frac);
        a - scaled
    }
}

pub fn grad_u8(hash: u8, x: i8, y: i8) -> i8 {
    let (mut u, mut v): (i8, i8);
    if hash & 4 > 0 {
        (u, v) = (y, x);
    } else {
        (u, v) = (x, y);
    }

    if hash & 1 > 0 {
        // u = -(if u == i8::MIN { u + 1 } else { u });
        u = u.wrapping_neg();
    }
    if hash & 2 > 0 {
        // v = -(if v == i8::MIN { v + 1 } else { v });
        v = v.wrapping_neg();
    }

    (u >> 1) + (v >> 1) + (u & 0x1)
}

pub fn fade_u8(x: u8) -> u8 {
    scale_u8(x, x)
    // scale_u8(x, x) >> 4
}

pub fn lerp_color(a: &[u8; 3], b: &[u8; 3], frac: u8) -> [u8; 3] {
    let mut result = [0; 3];
    for i in 0..=2 {
        result[i] = lerp_u8(a[i], b[i], frac);
    }
    result
}

pub fn color_wheel(wheel_pos: &u8) -> [u8; 3] {
    match wheel_pos {
        0..85 => [255 - wheel_pos * 3, wheel_pos * 3, 0],
        85..170 => [0, 255 - (wheel_pos - 85) * 3, (wheel_pos - 85) * 3],
        170..=255 => [(wheel_pos - 170) * 3, 0, 255 - (wheel_pos - 170) * 3],
    }
}

pub fn whiten(pixel: &[u8; 3], coeff: u8) -> [u8; 3] {
    lerp_color(pixel, &[255, 255, 255], coeff)
}

pub fn dim(pixel: &[u8; 3], coeff: u8) -> [u8; 3] {
    lerp_color(pixel, &[0, 0, 0], coeff)
}

pub fn lerp_gradient<const N: usize>(palette: &[[u8; 3]; N], frac: u8) -> [u8; 3] {
    let grad_count = N as u8 - 1;
    let variance = u8::MAX / grad_count;
    let grad_idx = (frac / variance).clamp(0, grad_count - 1) as usize;
    lerp_color(
        &palette[grad_idx],
        &palette[grad_idx + 1],
        (frac % variance) * grad_count,
    )
}
