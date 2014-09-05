#[cfg(test)]
mod test {

use gfx::opengl;
use gfx::renderer;
use gfx::shaders;
use gfx::window;

#[test]
pub fn test() {
    let mut window = window::Window::create(800, 600, "vodk");
    let mut ctx = window.create_rendering_context();

    test_texture_upload_readback(ctx);
    ctx.reset_state();
    test_render_to_texture(ctx);
    ctx.reset_state();

    window.swap_buffers();
}

fn test_texture_upload_readback(ctx: &mut gpu::RenderingContext) {
    println!("test test_texture_upload_readback...");
    let checker_data : Vec<u8> = Vec::from_fn(64*64*4, |i|{ (((i / 4) % 2)*255) as u8 });

    let checker = ctx.create_texture(gpu::REPEAT|gpu::FILTER_NEAREST);

    ctx.upload_texture_data(checker, checker_data.as_slice(), 64, 64, gpu::R8G8B8A8);

    let mut checker_read_back : Vec<u8> = Vec::from_fn(64*64*4, |i|{ 1 as u8 });

    assert!(checker_data != checker_read_back);

    ctx.read_back_texture(checker, gpu::R8G8B8A8,
                          checker_read_back.as_mut_slice());

    assert_eq!(checker_data, checker_read_back);

    ctx.destroy_texture(checker);
}

fn test_render_to_texture(ctx: &mut gpu::RenderingContext) {
    println!("test test_render_to_texture...");
    let w = 256;
    let h = 256;

    ctx.set_clear_color(0.0, 1.0, 0.0, 1.0);

    let target_texture = ctx.create_texture(gpu::CLAMP|gpu::FILTER_NEAREST);
    ctx.allocate_texture(target_texture, w, h, gpu::R8G8B8A8);
    let rt = match ctx.create_render_target([target_texture], None, None) {
        Ok(target) => target,
        Err(_) => fail!()
    };

    ctx.set_render_target(rt);

    ctx.clear(gpu::COLOR);

    let mut read_back : Vec<u8> = Vec::from_fn((w*h*4) as uint, |i|{ 1 as u8 });
    ctx.read_back_texture(target_texture, gpu::R8G8B8A8,
                          read_back.as_mut_slice());

    for j in range(0, h) {
        for i in range(0, w) {
            assert_eq!(*read_back.get(((i+j*h)*4    ) as uint), 0);
            assert_eq!(*read_back.get(((i+j*h)*4 + 1) as uint), 255);
            assert_eq!(*read_back.get(((i+j*h)*4 + 2) as uint), 0);
            assert_eq!(*read_back.get(((i+j*h)*4 + 3) as uint), 255);
        }
    }

    ctx.destroy_render_target(rt);
    ctx.destroy_texture(target_texture);
}

} // mod

