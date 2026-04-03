use winit::event_loop::EventLoop;
use matrix_rain::gpu;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = gpu::app::App::new();
    event_loop.run_app(&mut app).expect("Event loop error");

    // Print a random Matrix quote on exit
    use rand::Rng;
    let quotes = [
        "The Matrix has you.",
        "Follow the white rabbit.",
        "There is no spoon.",
        "Wake up, Neo...",
        "Welcome to the real world.",
        "Free your mind.",
    ];
    let mut rng = rand::thread_rng();
    println!("{}", quotes[rng.gen_range(0..quotes.len())]);
}
