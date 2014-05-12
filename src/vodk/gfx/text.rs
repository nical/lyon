
pub fn tex_coords_from_ascii(c: u8) -> (f32, f32) {
    return (
        (c%16) as f32 / 16.0,
        (c/16+1) as f32 / 16.0
    );
}

pub fn count_non_space(text: &str) -> uint {
    return text.chars().fold(0, |accum, c| {
        match c {
            ' '|'\n' => accum,
            _ => accum+1,
        }
    })
}

pub fn text_buffer(text: &str,
               x_offset: f32, y_offset: f32,
               char_w: f32, char_h: f32,
               out: &mut [f32]) {
    let ds = 1.0/16.0;
    let dt = 1.0/16.0;

    let mut x = x_offset;
    let mut y = y_offset;

    let mut i: uint = 0;

    for c in text.chars() {
        if c != ' ' && c != '\n' {
            let (tex_s, tex_t) = tex_coords_from_ascii(c as u8);
            out[i  ] = x;
            out[i+1] = y;
            out[i+2] = tex_s;
            out[i+3] = tex_t;

            out[i+4] = x;
            out[i+5] = y + char_h;
            out[i+6] = tex_s;
            out[i+7] = tex_t - dt;

            out[i+8] = x + char_w;
            out[i+9] = y + char_h;
            out[i+10] = tex_s + ds;
            out[i+11] = tex_t - dt;

            out[i+12] = x;
            out[i+13] = y;
            out[i+14] = tex_s;
            out[i+15] = tex_t;

            out[i+16] = x + char_w;
            out[i+17] = y + char_h;
            out[i+18] = tex_s + ds;
            out[i+19] = tex_t - dt;

            out[i+20] = x + char_w;
            out[i+21] = y;
            out[i+22] = tex_s + ds;
            out[i+23] = tex_t;
        }

        if c == '\n' {
            x = x_offset;
            y -= char_h;
        } else {
            x += char_w;
        }

        i += 24;
    }
}

