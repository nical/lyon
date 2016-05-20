#[macro_use]
extern crate glium;
extern crate lyon;
extern crate vodk_id;
extern crate vodk_math;

use glium::Surface;
use glium::glutin;
use glium::index::PrimitiveType;
use glium::DisplayBuild;

use lyon::tesselation::path::{ ComplexPath, PathBuilder, PointType };
use lyon::tesselation::vertex_builder::{ VertexConstructor, VertexBuffers, vertex_builder };
use lyon::tesselation::convex::*;
use lyon::tesselation::tesselation::tesselate_complex_path_fill;
use lyon::tesselation::experimental::tesselate_fill;

use vodk_math::*;

#[derive(Copy, Clone, Debug)]
struct Vertex {
    a_position: [f32; 2],
    a_color: [f32; 3],
}

struct VertexCtor {
    color: [f32; 3]
}

impl VertexConstructor<Vec2, Vertex> for VertexCtor {
    fn new_vertex(&mut self, pos: Vec2) -> Vertex {
        Vertex {
            a_position: pos.array(),
            a_color: self.color,
        }
    }
}

implement_vertex!(Vertex, a_position, a_color);

#[derive(Copy, Clone, Debug)]
struct BgVertex {
    a_position: [f32; 2],
}

struct BgVertexCtor;
impl VertexConstructor<Vec2, BgVertex> for BgVertexCtor {
    fn new_vertex(&mut self, pos: Vec2) -> BgVertex {
        BgVertex { a_position: pos.array() }
    }
}

implement_vertex!(BgVertex, a_position);

fn main() {

    let mut path = ComplexPath::new();

    PathBuilder::begin(&mut path, vec2(122.631, 69.716)).flattened()
        .relative_line_to(vec2(-4.394, -2.72))
        .relative_cubic_bezier_to(vec2(-0.037, -0.428), vec2(-0.079, -0.855), vec2(-0.125, -1.28))
        .relative_line_to(vec2(3.776, -3.522))
        .relative_cubic_bezier_to(vec2(0.384, -0.358), vec2(0.556, -0.888), vec2(0.452, -1.401))
        .relative_cubic_bezier_to(vec2(-0.101, -0.515), vec2(-0.462, -0.939), vec2(-0.953, -1.122))
        .relative_line_to(vec2(-4.827, -1.805))
        .relative_cubic_bezier_to(vec2(-0.121, -0.418), vec2(-0.248, -0.833), vec2(-0.378, -1.246))
        .relative_line_to(vec2(3.011, -4.182))
        .relative_cubic_bezier_to(vec2(0.307, -0.425), vec2(0.37, -0.978), vec2(0.17, -1.463))
        .relative_cubic_bezier_to(vec2(-0.2, -0.483), vec2(-0.637, -0.829), vec2(-1.154, -0.914))
        .relative_line_to(vec2(-5.09, -0.828))
        .relative_cubic_bezier_to(vec2(-0.198, -0.386), vec2(-0.404, -0.766), vec2(-0.612, -1.143))
        .relative_line_to(vec2(2.139, -4.695))
        .relative_cubic_bezier_to(vec2(0.219, -0.478), vec2(0.174, -1.034), vec2(-0.118, -1.468))
        .relative_cubic_bezier_to(vec2(-0.291, -0.436), vec2(-0.784, -0.691), vec2(-1.31, -0.671))
        .relative_line_to(vec2(-5.166, 0.18))
        .relative_cubic_bezier_to(vec2(-0.267, -0.334), vec2(-0.539, -0.665), vec2(-0.816, -0.99))
        .relative_line_to(vec2(1.187, -5.032))
        .relative_cubic_bezier_to(vec2(0.12, -0.511), vec2(-0.031, -1.046), vec2(-0.403, -1.417))
        .relative_cubic_bezier_to(vec2(-0.369, -0.37), vec2(-0.905, -0.523), vec2(-1.416, -0.403))
        .relative_line_to(vec2(-5.031, 1.186))
        .relative_cubic_bezier_to(vec2(-0.326, -0.276), vec2(-0.657, -0.549), vec2(-0.992, -0.816))
        .relative_line_to(vec2(0.181, -5.166))
        .relative_cubic_bezier_to(vec2(0.02, -0.523), vec2(-0.235, -1.02), vec2(-0.671, -1.31))
        .relative_cubic_bezier_to(vec2(-0.437, -0.292), vec2(-0.99, -0.336), vec2(-1.467, -0.119))
        .relative_line_to(vec2(-4.694, 2.14))
        .relative_cubic_bezier_to(vec2(-0.379, -0.208), vec2(-0.759, -0.414), vec2(-1.143, -0.613))
        .relative_line_to(vec2(-0.83, -5.091))
        .relative_cubic_bezier_to(vec2(-0.084, -0.516), vec2(-0.43, -0.954), vec2(-0.914, -1.154))
        .relative_cubic_bezier_to(vec2(-0.483, -0.201), vec2(-1.037, -0.136), vec2(-1.462, 0.17))
        .relative_line_to(vec2(-4.185, 3.011))
        .relative_cubic_bezier_to(vec2(-0.412, -0.131), vec2(-0.826, -0.257), vec2(-1.244, -0.377))
        .relative_line_to(vec2(-1.805, -4.828))
        .relative_cubic_bezier_to(vec2(-0.183, -0.492), vec2(-0.607, -0.853), vec2(-1.122, -0.955))
        .relative_cubic_bezier_to(vec2(-0.514, -0.101), vec2(-1.043, 0.07), vec2(-1.4, 0.452))
        .relative_line_to(vec2(-3.522, 3.779))
        .relative_cubic_bezier_to(vec2(-0.425, -0.047), vec2(-0.853, -0.09), vec2(-1.28, -0.125))
        .relative_line_to(vec2(-2.72, -4.395))
        .relative_cubic_bezier_to(vec2(-0.275, -0.445), vec2(-0.762, -0.716), vec2(-1.286, -0.716))
        .relative_cubic_bezier_to_s(vec2(-1.011, 0.271), vec2(-1.285, 0.716))
        .relative_line_to(vec2(-2.72, 4.395))
        .relative_cubic_bezier_to(vec2(-0.428, 0.035), vec2(-0.856, 0.078), vec2(-1.281, 0.125))
        .relative_line_to(vec2(-3.523, -3.779))
        .relative_cubic_bezier_to(vec2(-0.357, -0.382), vec2(-0.887, -0.553), vec2(-1.4, -0.452))
        .relative_cubic_bezier_to(vec2(-0.515, 0.103), vec2(-0.939, 0.463), vec2(-1.122, 0.955))
        .relative_line_to(vec2(-1.805, 4.828))
        .relative_cubic_bezier_to(vec2(-0.418, 0.12), vec2(-0.832, 0.247), vec2(-1.245, 0.377))
        .relative_line_to(vec2(-4.184, -3.011))
        .relative_cubic_bezier_to(vec2(-0.425, -0.307), vec2(-0.979, -0.372), vec2(-1.463, -0.17))
        .relative_cubic_bezier_to(vec2(-0.483, 0.2), vec2(-0.83, 0.638), vec2(-0.914, 1.154))
        .relative_line_to(vec2(-0.83, 5.091))
        .relative_cubic_bezier_to(vec2(-0.384, 0.199), vec2(-0.764, 0.404), vec2(-1.143, 0.613))
        .relative_line_to(vec2(-4.694, -2.14))
        .relative_cubic_bezier_to(vec2(-0.477, -0.218), vec2(-1.033, -0.173), vec2(-1.467, 0.119))
        .relative_cubic_bezier_to(vec2(-0.436, 0.29), vec2(-0.691, 0.787), vec2(-0.671, 1.31))
        .relative_line_to(vec2(0.18, 5.166))
        .relative_cubic_bezier_to(vec2(-0.334, 0.267), vec2(-0.665, 0.54), vec2(-0.992, 0.816))
        .relative_line_to(vec2(-5.031, -1.186))
        .relative_cubic_bezier_to(vec2(-0.511, -0.119), vec2(-1.047, 0.033), vec2(-1.417, 0.403))
        .relative_cubic_bezier_to(vec2(-0.372, 0.371), vec2(-0.523, 0.906), vec2(-0.403, 1.417))
        .relative_line_to(vec2(1.185, 5.032))
        .relative_cubic_bezier_to(vec2(-0.275, 0.326), vec2(-0.547, 0.656), vec2(-0.814, 0.99))
        .relative_line_to(vec2(-5.166, -0.18))
        .relative_cubic_bezier_to(vec2(-0.521, -0.015), vec2(-1.019, 0.235), vec2(-1.31, 0.671))
        .relative_cubic_bezier_to(vec2(-0.292, 0.434), vec2(-0.336, 0.99), vec2(-0.119, 1.468))
        .relative_line_to(vec2(2.14, 4.695))
        .relative_cubic_bezier_to(vec2(-0.208, 0.377), vec2(-0.414, 0.757), vec2(-0.613, 1.143))
        .relative_line_to(vec2(-5.09, 0.828))
        .relative_cubic_bezier_to(vec2(-0.517, 0.084), vec2(-0.953, 0.43), vec2(-1.154, 0.914))
        .relative_cubic_bezier_to(vec2(-0.2, 0.485), vec2(-0.135, 1.038), vec2(0.17, 1.463))
        .relative_line_to(vec2(3.011, 4.182))
        .relative_cubic_bezier_to(vec2(-0.131, 0.413), vec2(-0.258, 0.828), vec2(-0.378, 1.246))
        .relative_line_to(vec2(-4.828, 1.805))
        .relative_cubic_bezier_to(vec2(-0.49, 0.183), vec2(-0.851, 0.607), vec2(-0.953, 1.122))
        .relative_cubic_bezier_to(vec2(-0.102, 0.514), vec2(0.069, 1.043), vec2(0.452, 1.401))
        .relative_line_to(vec2(3.777, 3.522))
        .relative_cubic_bezier_to(vec2(-0.047, 0.425), vec2(-0.089, 0.853), vec2(-0.125, 1.28))
        .relative_line_to(vec2(-4.394, 2.72))
        .relative_cubic_bezier_to(vec2(-0.445, 0.275), vec2(-0.716, 0.761), vec2(-0.716, 1.286))
        .relative_cubic_bezier_to_s(vec2(0.271, 1.011), vec2(0.716, 1.285))
        .relative_line_to(vec2(4.394, 2.72))
        .relative_cubic_bezier_to(vec2(0.036, 0.428), vec2(0.078, 0.855), vec2(0.125, 1.28))
        .relative_line_to(vec2(-3.777, 3.523))
        .relative_cubic_bezier_to(vec2(-0.383, 0.357), vec2(-0.554, 0.887), vec2(-0.452, 1.4))
        .relative_cubic_bezier_to(vec2(0.102, 0.515), vec2(0.463, 0.938), vec2(0.953, 1.122))
        .relative_line_to(vec2(4.828, 1.805))
        .relative_cubic_bezier_to(vec2(0.12, 0.418), vec2(0.247, 0.833), vec2(0.378, 1.246))
        .relative_line_to(vec2(-3.011, 4.183))
        .relative_cubic_bezier_to(vec2(-0.306, 0.426), vec2(-0.371, 0.979), vec2(-0.17, 1.462))
        .relative_cubic_bezier_to(vec2(0.201, 0.485), vec2(0.638, 0.831), vec2(1.155, 0.914))
        .relative_line_to(vec2(5.089, 0.828))
        .relative_cubic_bezier_to(vec2(0.199, 0.386), vec2(0.403, 0.766), vec2(0.613, 1.145))
        .relative_line_to(vec2(-2.14, 4.693))
        .relative_cubic_bezier_to(vec2(-0.218, 0.477), vec2(-0.173, 1.032), vec2(0.119, 1.468))
        .relative_cubic_bezier_to(vec2(0.292, 0.437), vec2(0.789, 0.692), vec2(1.31, 0.671))
        .relative_line_to(vec2(5.164, -0.181))
        .relative_cubic_bezier_to(vec2(0.269, 0.336), vec2(0.54, 0.665), vec2(0.816, 0.992))
        .relative_line_to(vec2(-1.185, 5.033))
        .relative_cubic_bezier_to(vec2(-0.12, 0.51), vec2(0.031, 1.043), vec2(0.403, 1.414))
        .relative_cubic_bezier_to(vec2(0.369, 0.373), vec2(0.906, 0.522), vec2(1.417, 0.402))
        .relative_line_to(vec2(5.031, -1.185))
        .relative_cubic_bezier_to(vec2(0.327, 0.278), vec2(0.658, 0.548), vec2(0.992, 0.814))
        .relative_line_to(vec2(-0.18, 5.167))
        .relative_cubic_bezier_to(vec2(-0.02, 0.523), vec2(0.235, 1.019), vec2(0.671, 1.311))
        .relative_cubic_bezier_to(vec2(0.434, 0.291), vec2(0.99, 0.335), vec2(1.467, 0.117))
        .relative_line_to(vec2(4.694, -2.139))
        .relative_cubic_bezier_to(vec2(0.378, 0.21), vec2(0.758, 0.414), vec2(1.143, 0.613))
        .relative_line_to(vec2(0.83, 5.088))
        .relative_cubic_bezier_to(vec2(0.084, 0.518), vec2(0.43, 0.956), vec2(0.914, 1.155))
        .relative_cubic_bezier_to(vec2(0.483, 0.201), vec2(1.038, 0.136), vec2(1.463, -0.169))
        .relative_line_to(vec2(4.182, -3.013))
        .relative_cubic_bezier_to(vec2(0.413, 0.131), vec2(0.828, 0.259), vec2(1.246, 0.379))
        .relative_line_to(vec2(1.805, 4.826))
        .relative_cubic_bezier_to(vec2(0.183, 0.49), vec2(0.607, 0.853), vec2(1.122, 0.953))
        .relative_cubic_bezier_to(vec2(0.514, 0.104), vec2(1.043, -0.068), vec2(1.4, -0.452))
        .relative_line_to(vec2(3.523, -3.777))
        .relative_cubic_bezier_to(vec2(0.425, 0.049), vec2(0.853, 0.09), vec2(1.281, 0.128))
        .relative_line_to(vec2(2.72, 4.394))
        .relative_cubic_bezier_to(vec2(0.274, 0.443), vec2(0.761, 0.716), vec2(1.285, 0.716))
        .relative_cubic_bezier_to_s(vec2(1.011, -0.272), vec2(1.286, -0.716))
        .relative_line_to(vec2(2.72, -4.394))
        .relative_cubic_bezier_to(vec2(0.428, -0.038), vec2(0.855, -0.079), vec2(1.28, -0.128))
        .relative_line_to(vec2(3.522, 3.777))
        .relative_cubic_bezier_to(vec2(0.357, 0.384), vec2(0.887, 0.556), vec2(1.4, 0.452))
        .relative_cubic_bezier_to(vec2(0.515, -0.101), vec2(0.939, -0.463), vec2(1.122, -0.953))
        .relative_line_to(vec2(1.805, -4.826))
        .relative_cubic_bezier_to(vec2(0.418, -0.12), vec2(0.833, -0.248), vec2(1.246, -0.379))
        .relative_line_to(vec2(4.183, 3.013))
        .relative_cubic_bezier_to(vec2(0.425, 0.305), vec2(0.979, 0.37), vec2(1.462, 0.169))
        .relative_cubic_bezier_to(vec2(0.484, -0.199), vec2(0.83, -0.638), vec2(0.914, -1.155))
        .relative_line_to(vec2(0.83, -5.088))
        .relative_cubic_bezier_to(vec2(0.384, -0.199), vec2(0.764, -0.406), vec2(1.143, -0.613))
        .relative_line_to(vec2(4.694, 2.139))
        .relative_cubic_bezier_to(vec2(0.477, 0.218), vec2(1.032, 0.174), vec2(1.467, -0.117))
        .relative_cubic_bezier_to(vec2(0.436, -0.292), vec2(0.69, -0.787), vec2(0.671, -1.311))
        .relative_line_to(vec2(-0.18, -5.167))
        .relative_cubic_bezier_to(vec2(0.334, -0.267), vec2(0.665, -0.536), vec2(0.991, -0.814))
        .relative_line_to(vec2(5.031, 1.185))
        .relative_cubic_bezier_to(vec2(0.511, 0.12), vec2(1.047, -0.029), vec2(1.416, -0.402))
        .relative_cubic_bezier_to(vec2(0.372, -0.371), vec2(0.523, -0.904), vec2(0.403, -1.414))
        .relative_line_to(vec2(-1.185, -5.033))
        .relative_cubic_bezier_to(vec2(0.276, -0.327), vec2(0.548, -0.656), vec2(0.814, -0.992))
        .relative_line_to(vec2(5.166, 0.181))
        .relative_cubic_bezier_to(vec2(0.521, 0.021), vec2(1.019, -0.234), vec2(1.31, -0.671))
        .relative_cubic_bezier_to(vec2(0.292, -0.436), vec2(0.337, -0.991), vec2(0.118, -1.468))
        .relative_line_to(vec2(-2.139, -4.693))
        .relative_cubic_bezier_to(vec2(0.209, -0.379), vec2(0.414, -0.759), vec2(0.612, -1.145))
        .relative_line_to(vec2(5.09, -0.828))
        .relative_cubic_bezier_to(vec2(0.518, -0.083), vec2(0.954, -0.429), vec2(1.154, -0.914))
        .relative_cubic_bezier_to(vec2(0.2, -0.483), vec2(0.137, -1.036), vec2(-0.17, -1.462))
        .relative_line_to(vec2(-3.011, -4.183))
        .relative_cubic_bezier_to(vec2(0.13, -0.413), vec2(0.257, -0.828), vec2(0.378, -1.246))
        .relative_line_to(vec2(4.827, -1.805))
        .relative_cubic_bezier_to(vec2(0.491, -0.184), vec2(0.853, -0.607), vec2(0.953, -1.122))
        .relative_cubic_bezier_to(vec2(0.104, -0.514), vec2(-0.068, -1.043), vec2(-0.452, -1.4))
        .relative_line_to(vec2(-3.776, -3.523))
        .relative_cubic_bezier_to(vec2(0.046, -0.425), vec2(0.088, -0.853), vec2(0.125, -1.28))
        .relative_line_to(vec2(4.394, -2.72))
        .relative_cubic_bezier_to(vec2(0.445, -0.274), vec2(0.716, -0.761), vec2(0.716, -1.285))
        .cubic_bezier_to_s(vec2(123.076, 69.991), vec2(122.631, 69.716))
        .close();
    PathBuilder::begin(&mut path, vec2(93.222, 106.167)).flattened()
        .relative_cubic_bezier_to(vec2(-1.678, -0.362), vec2(-2.745, -2.016), vec2(-2.385, -3.699))
        .relative_cubic_bezier_to(vec2(0.359, -1.681), vec2(2.012, -2.751), vec2(3.689, -2.389))
        .relative_cubic_bezier_to(vec2(1.678, 0.359), vec2(2.747, 2.016), vec2(2.387, 3.696))
        .cubic_bezier_to_s(vec2(94.899, 106.526), vec2(93.222, 106.167))
        .close();
    PathBuilder::begin(&mut path, vec2(91.729, 96.069)).flattened()
        .relative_cubic_bezier_to(vec2(-1.531, -0.328), vec2(-3.037, 0.646), vec2(-3.365, 2.18))
        .relative_line_to(vec2(-1.56, 7.28))
        .relative_cubic_bezier_to(vec2(-4.814, 2.185), vec2(-10.16, 3.399), vec2(-15.79, 3.399))
        .relative_cubic_bezier_to(vec2(-5.759, 0.0), vec2(-11.221, -1.274), vec2(-16.121, -3.552))
        .relative_line_to(vec2(-1.559, -7.28))
        .relative_cubic_bezier_to(vec2(-0.328, -1.532), vec2(-1.834, -2.508), vec2(-3.364, -2.179))
        .relative_line_to(vec2(-6.427, 1.38))
        .relative_cubic_bezier_to(vec2(-1.193, -1.228), vec2(-2.303, -2.536), vec2(-3.323, -3.917))
        .relative_horizontal_line_to(31.272)
        .relative_cubic_bezier_to(vec2(0.354, 0.0), vec2(0.59, -0.064), vec2(0.59, -0.386))
        .vertical_line_to(81.932)
        .relative_cubic_bezier_to(vec2(0.0, -0.322), vec2(-0.236, -0.386), vec2(-0.59, -0.386))
        .relative_horizontal_line_to(-9.146)
        .relative_vertical_line_to(-7.012)
        .relative_horizontal_line_to(9.892)
        .relative_cubic_bezier_to(vec2(0.903, 0.0), vec2(4.828, 0.258), vec2(6.083, 5.275))
        .relative_cubic_bezier_to(vec2(0.393, 1.543), vec2(1.256, 6.562), vec2(1.846, 8.169))
        .relative_cubic_bezier_to(vec2(0.588, 1.802), vec2(2.982, 5.402), vec2(5.533, 5.402))
        .relative_horizontal_line_to(15.583)
        .relative_cubic_bezier_to(vec2(0.177, 0.0), vec2(0.366, -0.02), vec2(0.565, -0.056))
        .relative_cubic_bezier_to(vec2(-1.081, 1.469), vec2(-2.267, 2.859), vec2(-3.544, 4.158))
        .line_to(vec2(91.729, 96.069))
        .close();
    PathBuilder::begin(&mut path, vec2(48.477, 106.015)).flattened()
        .relative_cubic_bezier_to(vec2(-1.678, 0.362), vec2(-3.33, -0.708), vec2(-3.691, -2.389))
        .relative_cubic_bezier_to(vec2(-0.359, -1.684), vec2(0.708, -3.337), vec2(2.386, -3.699))
        .relative_cubic_bezier_to(vec2(1.678, -0.359), vec2(3.331, 0.711), vec2(3.691, 2.392))
        .cubic_bezier_to(vec2(51.222, 103.999), vec2(50.154, 105.655), vec2(48.477, 106.015))
        .close();
    PathBuilder::begin(&mut path, vec2(36.614, 57.91)).flattened()
        .relative_cubic_bezier_to(vec2(0.696, 1.571), vec2(-0.012, 3.412), vec2(-1.581, 4.107))
        .relative_cubic_bezier_to(vec2(-1.569, 0.697), vec2(-3.405, -0.012), vec2(-4.101, -1.584))
        .relative_cubic_bezier_to(vec2(-0.696, -1.572), vec2(0.012, -3.41), vec2(1.581, -4.107))
        .cubic_bezier_to(vec2(34.083, 55.63), vec2(35.918, 56.338), vec2(36.614, 57.91))
        .close();
    PathBuilder::begin(&mut path, vec2(32.968, 66.553)).flattened()
        .relative_line_to(vec2(6.695, -2.975))
        .relative_cubic_bezier_to(vec2(1.43, -0.635), vec2(2.076, -2.311), vec2(1.441, -3.744))
        .relative_line_to(vec2(-1.379, -3.118))
        .relative_horizontal_line_to(5.423)
        .vertical_line_to(81.16)
        .horizontal_line_to(34.207)
        .relative_cubic_bezier_to(vec2(-0.949, -3.336), vec2(-1.458, -6.857), vec2(-1.458, -10.496))
        .cubic_bezier_to(vec2(32.749, 69.275), vec2(32.824, 67.902), vec2(32.968, 66.553))
        .close();
    PathBuilder::begin(&mut path, vec2(62.348, 64.179)).flattened()
        .relative_vertical_line_to(-7.205)
        .relative_horizontal_line_to(12.914)
        .relative_cubic_bezier_to(vec2(0.667, 0.0), vec2(4.71, 0.771), vec2(4.71, 3.794))
        .relative_cubic_bezier_to(vec2(0.0, 2.51), vec2(-3.101, 3.41), vec2(-5.651, 3.41))
        //.horizontal_line_to(62.348) //TODO
        .close();
    PathBuilder::begin(&mut path, vec2(109.28, 70.664)).flattened()
        .relative_cubic_bezier_to(vec2(0.0, 0.956), vec2(-0.035, 1.902), vec2(-0.105, 2.841))
        .relative_horizontal_line_to(-3.926)
        .relative_cubic_bezier_to(vec2(-0.393, 0.0), vec2(-0.551, 0.258), vec2(-0.551, 0.643))
        .relative_vertical_line_to(1.803)
        .relative_cubic_bezier_to(vec2(0.0, 4.244), vec2(-2.393, 5.167), vec2(-4.49, 5.402))
        .relative_cubic_bezier_to(vec2(-1.997, 0.225), vec2(-4.211, -0.836), vec2(-4.484, -2.058))
        .relative_cubic_bezier_to(vec2(-1.178, -6.626), vec2(-3.141, -8.041), vec2(-6.241, -10.486))
        .relative_cubic_bezier_to(vec2(3.847, -2.443), vec2(7.85, -6.047), vec2(7.85, -10.871))
        .relative_cubic_bezier_to(vec2(0.0, -5.209), vec2(-3.571, -8.49), vec2(-6.005, -10.099))
        .relative_cubic_bezier_to(vec2(-3.415, -2.251), vec2(-7.196, -2.702), vec2(-8.216, -2.702))
        .horizontal_line_to(42.509)
        .relative_cubic_bezier_to(vec2(5.506, -6.145), vec2(12.968, -10.498), vec2(21.408, -12.082))
        .relative_line_to(vec2(4.786, 5.021))
        .relative_cubic_bezier_to(vec2(1.082, 1.133), vec2(2.874, 1.175), vec2(4.006, 0.092))
        .relative_line_to(vec2(5.355, -5.122))
        .relative_cubic_bezier_to(vec2(11.221, 2.089), vec2(20.721, 9.074), vec2(26.196, 18.657))
        .relative_line_to(vec2(-3.666, 8.28))
        .relative_cubic_bezier_to(vec2(-0.633, 1.433), vec2(0.013, 3.109), vec2(1.442, 3.744))
        .relative_line_to(vec2(7.058, 3.135))
        .cubic_bezier_to(vec2(109.216, 68.115), vec2(109.28, 69.381), vec2(109.28, 70.664))
        .close();
    PathBuilder::begin(&mut path, vec2(68.705, 28.784)).flattened()
        .relative_cubic_bezier_to(vec2(1.24, -1.188), vec2(3.207, -1.141), vec2(4.394, 0.101))
        .relative_cubic_bezier_to(vec2(1.185, 1.245), vec2(1.14, 3.214), vec2(-0.103, 4.401))
        .relative_cubic_bezier_to(vec2(-1.24, 1.188), vec2(-3.207, 1.142), vec2(-4.394, -0.102))
        .cubic_bezier_to(vec2(67.418, 31.941), vec2(67.463, 29.972), vec2(68.705, 28.784))
        .close();
    PathBuilder::begin(&mut path, vec2(105.085, 58.061)).flattened()
        .relative_cubic_bezier_to(vec2(0.695, -1.571), vec2(2.531, -2.28), vec2(4.1, -1.583))
        .relative_cubic_bezier_to(vec2(1.569, 0.696), vec2(2.277, 2.536), vec2(1.581, 4.107))
        .relative_cubic_bezier_to(vec2(-0.695, 1.572), vec2(-2.531, 2.281), vec2(-4.101, 1.584))
        .cubic_bezier_to(vec2(105.098, 61.473), vec2(104.39, 59.634), vec2(105.085, 58.061))
        .close();

    let mut path = ComplexPath::new();
    PathBuilder::begin(&mut path, vec2(20.0, 20.0)).flattened()
        .line_to(vec2(60.0, 20.0))
        .line_to(vec2(60.0, 60.0))
        .line_to(vec2(20.0, 60.0))
        .close();
    PathBuilder::begin(&mut path, vec2(40.0, 10.0)).flattened()
        .line_to(vec2(70.0, 40.0))
        .line_to(vec2(40.0, 70.0))
        .line_to(vec2(10.0, 40.0))
        .close();


    let mut buffers: VertexBuffers<Vertex> = VertexBuffers::new();

    tesselate_fill(
        path.as_slice(),
        &mut vertex_builder(&mut buffers, VertexCtor{ color: [0.9, 0.9, 1.0] })
    ).unwrap();

/*
    for p in path.vertices().as_slice() {
        fill_ellipsis(p.position, vec2(10.0, 10.0), 16,
            &mut vertex_builder(&mut buffers,
                VertexCtor{ color: [0.0, 0.0, 0.0] }
            )
        );

        fill_ellipsis(p.position, vec2(5.0, 5.0), 16,
            &mut vertex_builder(&mut buffers,
                VertexCtor{
                    color: if p.point_type == PointType::Normal { [0.0, 1.0, 0.0] }
                           else { [0.0, 1.0, 1.0] }
                }
            )
        );
    }
*/
    let (indices, vertices) = (buffers.indices, buffers.vertices);

    println!(" -- {} vertices {} indices", vertices.len(), indices.len());

    let mut bg_buffers: VertexBuffers<BgVertex> = VertexBuffers::new();
    fill_rectangle(
        &Rect::new(-1.0, -1.0, 2.0, 2.0),
        &mut vertex_builder(&mut bg_buffers, BgVertexCtor)
    );

    // building the display, ie. the main object
    let display = glutin::WindowBuilder::new()
        .with_dimensions(700, 700)
        .with_title("tesselation".to_string())
        .build_glium().unwrap();

    let model_vbo = glium::VertexBuffer::new(&display, &vertices[..]).unwrap();
    let model_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &indices[..]
    ).unwrap();

    let bg_vbo = glium::VertexBuffer::new(&display, &bg_buffers.vertices[..]).unwrap();
    let bg_ibo = glium::IndexBuffer::new(
        &display, PrimitiveType::TrianglesList,
        &bg_buffers.indices[..]
    ).unwrap();

    // compiling shaders and linking them together
    let bg_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                in vec2 a_position;
                out vec2 v_position;
                void main() {
                    gl_Position = vec4(a_position, 0.0, 1.0);
                    v_position = a_position;
                }
            ",
            fragment: "
                #version 140
                uniform vec2 u_resolution;
                in vec2 v_position;
                out vec4 f_color;
                void main() {
                    vec2 px_position = (v_position * vec2(1.0, -1.0)    + vec2(1.0, 1.0))
                                     * 0.5 * u_resolution;
                    // #005fa4
                    float vignette = clamp(0.0, 1.0, (0.7*length(v_position)));

                    f_color = mix(
                        vec4(0.0, 0.47, 0.9, 1.0),
                        vec4(0.0, 0.1, 0.64, 1.0),
                        vignette
                    );

                    if (mod(px_position.x, 20.0) <= 1.0 ||
                        mod(px_position.y, 20.0) <= 1.0) {
                        f_color *= 1.2;
                    }

                    if (mod(px_position.x, 100.0) <= 1.0 ||
                        mod(px_position.y, 100.0) <= 1.0) {
                        f_color *= 1.2;
                    }
                }
            "
        },
    ).unwrap();

    // compiling shaders and linking them together
    let model_program = program!(&display,
        140 => {
            vertex: "
                #version 140
                uniform vec2 u_resolution;
                uniform mat4 u_matrix;
                in vec2 a_position;
                in vec3 a_color;
                out vec3 v_color;
                void main() {
                    gl_Position = vec4(a_position, 0.0, 1.0) * u_matrix;// / vec4(u_resolution, 1.0, 1.0);
                    v_color = a_color;
                }
            ",
            fragment: "
                #version 140
                in vec3 v_color;
                out vec4 f_color;
                void main() {
                    f_color = vec4(v_color, 1.0);
                }
            "
        },
    ).unwrap();

    loop {
        let mut target = display.draw();

        let (w, h) = target.get_dimensions();
        let resolution = vec2(w as f32, h as f32);

        let mut model_mat: Matrix4x4<units::Local, units::World> = Matrix4x4::identity();
        model_mat.scale_by(Vector3D::new(5.0, 5.0, 0.0));

        let mut view_mat: Matrix4x4<units::World, units::Screen> = Matrix4x4::identity();
        view_mat.scale_by(Vector3D::new(2.0/resolution.x, -2.0/resolution.y, 1.0));
        view_mat.translate(Vector3D::new(-1.0, 1.0, 0.0));
        //view_mat = view_mat * Matrix4x4::translation(Vector3D::new(-1.0, 1.0, 0.0));

        let uniforms = uniform! {
            u_resolution: resolution.array(),
            u_matrix: *(model_mat * view_mat).as_arrays()
        };

        target.clear_color(0.75, 0.75, 0.75, 1.0);
        target.draw(
            &bg_vbo, &bg_ibo,
            &bg_program, &uniforms,
            &Default::default()
        ).unwrap();
        target.draw(
            &model_vbo, &model_ibo,
            &model_program, &uniforms,
            &Default::default()
        ).unwrap();
        target.finish().unwrap();

        let mut should_close = false;
        for event in display.poll_events() {
            should_close |= match event {
                glutin::Event::Closed => true,
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) => true,
                _ => {
                    //println!("{:?}", evt);
                    false
                }
            };
        }
        if should_close {
            break;
        }
    }
}
