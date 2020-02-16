use std::slice::from_raw_parts_mut;

/// A view on a writable image in memory.
pub struct MutableImageSlice<'l, Pixel: Copy + 'static> {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub pixels: &'l mut [Pixel],
}

impl<'l, Pixel: Copy + 'static> MutableImageSlice<'l, Pixel> {
    pub fn new(
        width: usize,
        height: usize,
        pixels: &'l mut [Pixel],
    ) -> MutableImageSlice<'l, Pixel> {
        return MutableImageSlice::with_stride(width, height, width, pixels);
    }

    pub fn with_stride(
        width: usize,
        height: usize,
        stride: usize,
        pixels: &'l mut [Pixel],
    ) -> MutableImageSlice<'l, Pixel> {
        assert!(width <= stride);
        assert!(pixels.len() >= height * stride);

        return MutableImageSlice {
            width: width,
            height: height,
            stride: stride,
            pixels: pixels,
        };
    }

    pub fn pixel_offset(&self, x: usize, y: usize) -> usize {
        x + y * self.stride
    }

    pub fn contains_pixel(&self, x: usize, y: usize) -> bool {
        x <= self.width && y <= self.height
    }

    pub fn split_vertically(
        &'l mut self,
        at: usize,
    ) -> (MutableImageSlice<'l, Pixel>, MutableImageSlice<'l, Pixel>) {
        unsafe {
            let split = if at < self.width { at } else { self.width };

            let p: *mut Pixel = &mut self.pixels[0];
            let q: *mut Pixel = p.offset(split as isize);
            let remainder = self.pixels.len() - split;

            let pixels_left: &'l mut [Pixel] = from_raw_parts_mut(p, self.pixels.len());
            let pixels_right: &'l mut [Pixel] = from_raw_parts_mut(q, remainder);

            return (
                MutableImageSlice {
                    width: split,
                    height: self.height,
                    stride: self.stride,
                    pixels: pixels_left,
                },
                MutableImageSlice {
                    width: self.width - split,
                    height: self.height,
                    stride: self.stride,
                    pixels: pixels_right,
                },
            );
        }
    }

    fn split_horizontally(
        &'l mut self,
        at: usize,
    ) -> (MutableImageSlice<'l, Pixel>, MutableImageSlice<'l, Pixel>) {
        unsafe {
            let split = if at < self.width { at } else { self.width };

            let p: *mut Pixel = &mut self.pixels[0];
            let q: *mut Pixel = p.offset((self.stride * split) as isize);
            let remainder = self.pixels.len() - self.stride * split;

            let pixels_left: &'l mut [Pixel] = from_raw_parts_mut(p, self.pixels.len());
            let pixels_right: &'l mut [Pixel] = from_raw_parts_mut(q, remainder);

            return (
                MutableImageSlice {
                    width: self.width,
                    height: split,
                    stride: self.stride,
                    pixels: pixels_left,
                },
                MutableImageSlice {
                    width: self.width,
                    height: self.height - split,
                    stride: self.stride,
                    pixels: pixels_right,
                },
            );
        }
    }
}
