use std::slice::from_raw_parts_mut;

/// A view on a writable image in memory.
pub struct MutableImageSlice<'l, Pixel:'l> {
    width: usize,
    height: usize,
    stride: usize,
    data: &'l mut [Pixel],
}

impl<'l, Pixel:'l> MutableImageSlice<'l, Pixel> {
    pub fn new(width: usize, height: usize, data: &'l mut [Pixel]) -> MutableImageSlice<'l, Pixel> {
        return MutableImageSlice::with_stride(width, height, width, data);
    }

    pub fn with_stride(width: usize, height: usize, stride: usize, data: &'l mut [Pixel]) -> MutableImageSlice<'l, Pixel> {
        assert!(width <= stride);
        assert!(data.len() >= height * stride);

        return MutableImageSlice {
            width: width,
            height: height,
            stride: stride,
            data: data,
        };
    }

    pub fn get_size(&self) -> (usize, usize) { (self.width, self.height) }

    pub fn get_stride(&self) -> usize { self.stride }

    pub fn get_data(&'l self) -> &'l [Pixel] { self.data }

    pub fn get_mut_data(&'l mut self) -> &'l mut [Pixel] { self.data }

    pub fn pixel_offset(&self, x: usize, y: usize) -> usize { x + y * self.stride }

    pub fn contains_pixel(&self, x: usize, y: usize) -> bool { x <= self.width && y <= self.height }

    pub fn split_vertically(&'l mut self, at: usize) -> (MutableImageSlice<'l, Pixel>, MutableImageSlice<'l, Pixel>) {
        unsafe {
            let split = if at < self.width { at } else { self.width };

            let p: *mut Pixel = &mut self.data[0];
            let q: *mut Pixel = p.offset(split as isize);
            let remainder = self.data.len() - split;

            let data_left: &'l mut [Pixel] = from_raw_parts_mut(p, self.data.len());
            let data_right: &'l mut [Pixel] = from_raw_parts_mut(q, remainder);

            return (
                MutableImageSlice {
                    width: split,
                    height: self.height,
                    stride: self.stride,
                    data: data_left,
                },
                MutableImageSlice {
                    width: self.width - split,
                    height: self.height,
                    stride: self.stride,
                    data: data_right,
                }
            );
        }
    }

    fn split_horizontally(&'l mut self, at: usize) -> (MutableImageSlice<'l, Pixel>, MutableImageSlice<'l, Pixel>) {
        unsafe {
            let split = if at < self.width { at } else { self.width };

            let p: *mut Pixel = &mut self.data[0];
            let q: *mut Pixel = p.offset((self.stride * split) as isize);
            let remainder = self.data.len() - self.stride * split;

            let data_left: &'l mut [Pixel] = from_raw_parts_mut(p, self.data.len());
            let data_right: &'l mut [Pixel] = from_raw_parts_mut(q, remainder);

            return (
                MutableImageSlice {
                    width: self.width,
                    height: split,
                    stride: self.stride,
                    data: data_left,
                },
                MutableImageSlice {
                    width: self.width,
                    height: self.height - split,
                    stride: self.stride,
                    data: data_right,
                }
            );
        }
    }
}
