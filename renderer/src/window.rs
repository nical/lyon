use wgpu::winit::{Event, WindowEvent};

pub struct Window {
    pub window: wgpu::winit::Window,
    pub events_loop: wgpu::winit::EventsLoop,
    pub surface: wgpu::Surface,
    pub swap_chain: wgpu::SwapChain,
    pub physical_width: f64,
    pub physical_height: f64,
}

impl Window {
    pub fn new(device: &wgpu::Device, instance: &wgpu::Instance) -> Window {
        let events_loop = wgpu::winit::EventsLoop::new();
        let window = wgpu::winit::Window::new(&events_loop).unwrap();

        let size = window.get_inner_size().unwrap().to_physical(
            window.get_hidpi_factor()
        );

        let physical_width = size.width.round();
        let physical_height = size.height.round();

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsageFlags::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: physical_width as u32,
            height: physical_height as u32,
        };

        let surface = instance.create_surface(&window);

        let swap_chain = device.create_swap_chain(
            &surface,
            &swap_chain_desc,
        );

        Window {
            window,
            events_loop,
            surface,
            swap_chain,
            physical_width,
            physical_height,
        }
    }

    pub fn poll_events<F>(&mut self, device: &wgpu::Device, mut callback: F) -> bool
    where
        F: FnMut(wgpu::winit::Event)
    {
        let mut status = true;

        let hidpi_factor = self.window.get_hidpi_factor();
        let surface = &self.surface;
        let swap_chain = &mut self.swap_chain;
        let physical_width = &mut self.physical_width;
        let physical_height = &mut self.physical_height;

        self.events_loop.poll_events(|event| {
            match event {
                Event::WindowEvent { event: WindowEvent::Destroyed, .. }
                | Event::WindowEvent { event: WindowEvent::CloseRequested, .. }
                => {
                    status = false;
                },
                Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                    let physical = size.to_physical(hidpi_factor);
                    let swap_chain_desc = wgpu::SwapChainDescriptor {
                        usage: wgpu::TextureUsageFlags::OUTPUT_ATTACHMENT,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        width: physical.width.round() as u32,
                        height: physical.height.round() as u32,
                    };

                    *physical_width = physical.width;
                    *physical_height = physical.height;

                    *swap_chain = device.create_swap_chain(surface, &swap_chain_desc);
                }
                _ => {}
            }

            callback(event);
        });

        status
    }
}

