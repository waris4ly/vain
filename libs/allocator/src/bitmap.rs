pub struct BitmapAllocator<'a> {
    bitmap: &'a mut [u8],
    total_frames: usize,
    free_frames: usize,
    last_alloc_index: usize,
}

impl<'a> BitmapAllocator<'a> {
    pub fn new(bitmap: &'a mut [u8], total_frames: usize) -> Self {
        for byte in bitmap.iter_mut() {
            *byte = 0xFF;
        }

        Self {
            bitmap,
            total_frames,
            free_frames: 0,
            last_alloc_index: 0,
        }
    }

    pub fn mark_free(&mut self, frame: usize) {
        if frame < self.total_frames {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            if self.bitmap[byte_idx] & (1 << bit_idx) != 0 {
                self.bitmap[byte_idx] &= !(1 << bit_idx);
                self.free_frames += 1;
            }
        }
    }

    pub fn mark_used(&mut self, frame: usize) {
        if frame < self.total_frames {
            let byte_idx = frame / 8;
            let bit_idx = frame % 8;
            if self.bitmap[byte_idx] & (1 << bit_idx) == 0 {
                self.bitmap[byte_idx] |= 1 << bit_idx;
                self.free_frames -= 1;
            }
        }
    }

    pub fn alloc(&mut self) -> Option<usize> {
        if self.free_frames == 0 {
            return None;
        }

        let start_idx = self.last_alloc_index;

        for i in 0..self.total_frames {
            let idx = (start_idx + i) % self.total_frames;
            let byte_idx = idx / 8;
            let bit_idx = idx % 8;

            if self.bitmap[byte_idx] & (1 << bit_idx) == 0 {
                self.mark_used(idx);
                self.last_alloc_index = idx;
                return Some(idx);
            }
        }

        None
    }

    pub fn free(&mut self, frame: usize) {
        self.mark_free(frame);
    }

    pub fn free_frames(&self) -> usize {
        self.free_frames
    }

    pub fn total_frames(&self) -> usize {
        self.total_frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_allocator() {
        let mut buffer = [0u8; 4];
        let mut allocator = BitmapAllocator::new(&mut buffer, 32);
        assert_eq!(allocator.free_frames(), 0);

        allocator.mark_free(0);
        allocator.mark_free(1);
        allocator.mark_free(5);
        assert_eq!(allocator.free_frames(), 3);

        assert_eq!(allocator.alloc(), Some(0));
        assert_eq!(allocator.alloc(), Some(1));
        assert_eq!(allocator.alloc(), Some(5));
        assert_eq!(allocator.alloc(), None);

        allocator.free(1);
        assert_eq!(allocator.alloc(), Some(1));
    }
}
