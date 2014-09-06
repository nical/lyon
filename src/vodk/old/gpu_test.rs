extern crate gpu;
extern crate glfw;
extern crate gl;

use gpu = gpu::context;
use gpu::opengl;

fn main() {
    let mut window = gfx::window::Window::create(800, 600, "vodk");
    let ctx = window.create_rendering_context();
    let mut input_events: Vec<inputs::Event> = Vec::new();

    let mut avg_frame_time: u64 = 0;
    let mut frame_count: u64 = 0;
    let mut previous_time = time::precise_time_ns();
    let mut i = 0;
    while !window.should_close() {
        input_events.clear();
        window.poll_events(&mut input_events);
        app.handle_events(input_events.as_slice());
        let frame_start_time = time::precise_time_ns();
        let elapsed_time = frame_start_time - previous_time;

        app.update(elapsed_time as f32 / 1000000.0 , i);

        i+=1;
        previous_time = frame_start_time;
        let frame_time = time::precise_time_ns() - frame_start_time;
        frame_count += 1;
        avg_frame_time += frame_time;

        if frame_count % 60 == 0 {
            println!("avg frame time: {}ms", avg_frame_time as f64/(60.0*1000000.0));
            avg_frame_time = 0;
        }

        let sleep_time: i64 = 16000000 - frame_time as i64;
        if sleep_time > 0 {
            sleep(sleep_time as u64/1000000 );
        }
    }

    app.shut_down();  
}