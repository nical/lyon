use api::*;
use core::math::Point;
use buffer::{CpuBuffer, IdRange};
use gpu_data::{GpuBlock4, GpuBlock8, GpuBlock16, GpuBlock32};
use gpu_data::{GpuRect};

pub struct DataStore {
    points: Vec<Point>,
    colors: Vec<Color>,
    numbers: Vec<f32>,

    data_4: CpuBuffer<GpuBlock4>,
    data_8: CpuBuffer<GpuBlock8>,
    data_16: CpuBuffer<GpuBlock16>,
    data_32: CpuBuffer<GpuBlock32>,
}

impl DataStore {
    pub fn new() -> Self {
        DataStore {
            points: Vec::new(),
            colors: Vec::new(),
            numbers: Vec::new(),

            data_4: CpuBuffer::new(256),
            data_8: CpuBuffer::new(256),
            data_16: CpuBuffer::new(256),
            data_32: CpuBuffer::new(256),
        }
    }

    pub fn add_colors(&mut self, colors: &[Color], _usage: Usage) -> ColorIdRange {
        let first = self.colors.len();
        self.colors.extend_from_slice(colors);
        let last = self.colors.len();
        ColorIdRange::from_indices(first..last)
    }

    pub fn add_transforms(&mut self, transforms: &[Transform], _usage: Usage) -> TransformIdRange {
        self.data_8.push_range(transforms)
    }

    pub fn add_numbers(&mut self, values: &[f32], _usage: Usage) -> NumberIdRange {
        let first = self.numbers.len();
        self.numbers.extend_from_slice(values);
        let last = self.numbers.len();
        NumberIdRange::from_indices(first..last)
    }

    pub fn add_points(&mut self, values: &[Point], _usage: Usage) -> PointIdRange {
        let first = self.points.len();
        self.points.extend_from_slice(values);
        let last = self.points.len();
        PointIdRange::from_indices(first..last)
    }

    pub fn add_rects(&mut self, values: &[GpuRect], _usage: Usage) -> RectIdRange {
        self.data_4.push_range(values)
    }

    pub fn set_colors(&mut self, range: ColorIdRange, values: &[Color]) {
        for i in range.usize_range() {
            self.colors[i] = values[i]
        }
    }

    pub fn set_transforms(&mut self, range: TransformIdRange, values: &[Transform]) {
        self.data_8.set_range(range, values);
    }

    pub fn set_numbers(&mut self, range: NumberIdRange, values: &[f32]) {
        for i in range.usize_range() {
            self.numbers[i] = values[i]
        }
    }

    pub fn set_points(&mut self, range: PointIdRange, values: &[Point]) {
        for i in range.usize_range() {
            self.points[i] = values[i]
        }
    }

    pub fn set_rects(&mut self, range: RectIdRange, values: &[GpuRect]) {
        self.data_4.set_range(range, values);
    }

    pub fn add_gpu_blocks_4(&mut self, blocks: &[GpuBlock4]) -> IdRange<GpuBlock4> {
        self.data_4.push_range(blocks)
    }

    pub fn add_gpu_blocks_8(&mut self, blocks: &[GpuBlock8]) -> IdRange<GpuBlock8> {
        self.data_8.push_range(blocks)
    }

    pub fn add_gpu_blocks_16(&mut self, blocks: &[GpuBlock16]) -> IdRange<GpuBlock16> {
        self.data_16.push_range(blocks)
    }

    pub fn add_gpu_blocks_32(&mut self, blocks: &[GpuBlock32]) -> IdRange<GpuBlock32> {
        self.data_32.push_range(blocks)
    }
}
