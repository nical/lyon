
use math::units::pixels;
use math::units::texels;

pub fn tex_coords_from_ascii(c: u8) -> texels::Vec2 {
    return texels::vec2(
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

pub fn text_to_vertices(
    text: &str,
    pos: pixels::Vec2,          // position of the beginning of the text run;
    char_size: pixels::Vec2,
    char_margin: pixels::Vec2,  // separation between letters
    source_rect: texels::Rectangle,  // region of the font texture to sample from
    vertex_stride: uint,        // in bytes
    tex_coords_offset: uint,    // in bytes
    out: &mut [f32]             // output
) -> uint {                     // returns the number of vertices added
    let ds = 1.0/16.0 * source_rect.w;
    let dt = 1.0/16.0 * source_rect.h;

    let stride = vertex_stride / 4;
    let tc = tex_coords_offset / 4;

    let mut x = pos.x;
    let mut y = pos.y;

    let mut i: uint = 0;

    for c in text.chars() {
        if c != ' ' && c != '\n' {
            let tex_coords = tex_coords_from_ascii(c as u8);
            let tex_s = tex_coords.x * source_rect.w + source_rect.x;
            let tex_t = tex_coords.y * source_rect.h + source_rect.y;
            out[i] = x;
            out[i + 1 ] = y;
            out[i + tc]     = tex_s;
            out[i + tc + 1] = tex_t;

            i += stride;
            out[i] = x;
            out[i + 1] = y - char_size.y;
            out[i + tc] = tex_s;
            out[i + tc + 1] = tex_t - dt;

            i += stride;
            out[i] = x + char_size.x;
            out[i + 1] = y - char_size.y;
            out[i + tc] = tex_s + ds;
            out[i + tc + 1] = tex_t - dt;

            i += stride;
            out[i] = x;
            out[i + 1] = y;
            out[i + tc] = tex_s;
            out[i + tc + 1] = tex_t;

            i += stride;
            out[i] = x + char_size.x;
            out[i + 1] = y - char_size.y;
            out[i + tc] = tex_s + ds;
            out[i + tc + 1] = tex_t - dt;

            i += stride;
            out[i] = x + char_size.x;
            out[i + 1] = y;
            out[i + tc] = tex_s + ds;
            out[i + tc + 1] = tex_t;

            i += stride;
        }

        if c == '\n' {
            x = pos.x;
            y += char_size.y + char_margin.y;
        } else {
            x += char_size.x + char_margin.x;
        }
    }
    return i / stride;
}

