
use io::inputs;

pub trait Application {
    fn handle_events(&mut self, &[inputs::Event]);
    fn update(&mut self, elapsed_time: f32, frame_count: u64);
    fn get_help(&self) -> String;
    fn shut_down(&mut self);
}
