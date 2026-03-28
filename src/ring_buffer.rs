use core::ops::Index;

pub struct RingBuffer<T, const N: usize> {
    data: [T; N],
    index: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self
    where
        T: Copy + Default,
    {
        Self {
            data: [T::default(); N],
            index: 0,
        }
    }

    pub fn push(&mut self, element: T) {
        self.data[self.index] = element;
        self.index = (self.index + 1) % N;
    }
}

impl<T, const N: usize> Index<usize> for RingBuffer<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[(index + self.index) % N]
    }
}
