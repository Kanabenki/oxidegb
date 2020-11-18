use super::palette;

pub struct PixelFifo {
    _fifo: [palette::Color; 16],
    start: usize,
    end: usize,
}

impl PixelFifo {
    const SIZE: usize = 16;
    const _HALF_SIZE: usize = Self::SIZE / 2;

    pub fn new() -> Self {
        Self {
            _fifo: [palette::Color::White; Self::SIZE],
            start: 0,
            end: 0,
        }
    }

    fn _size(&self) -> usize {
        (self.end as isize - self.start as isize
            + (if self.start > self.end {
                Self::SIZE as isize
            } else {
                0
            })) as usize
    }

    fn _push_line(&mut self, colors: &[palette::Color; 8]) -> bool {
        if self._size() <= Self::_HALF_SIZE {
            for (i, color) in colors.iter().cloned().enumerate() {
                self._fifo[(self.end + i) % Self::SIZE] = color;
            }
            self.end = (self.end + 1) % 16;
            true
        } else {
            false
        }
    }

    fn _pop(&mut self) -> Option<palette::Color> {
        if self._size() > Self::_HALF_SIZE {
            let color = Some(self._fifo[self.start]);
            self.start = (self.start + 1) % Self::SIZE;
            color
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.start = 0;
        self.end = 0;
    }
}
