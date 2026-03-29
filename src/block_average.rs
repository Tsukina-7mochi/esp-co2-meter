use num::traits::AsPrimitive;

#[derive(Debug, Clone)]
pub struct BlockAverage<T> {
    block_size: usize,
    sum: T,
    sum_count: usize,
}

impl<T> BlockAverage<T> {
    pub fn new(block_size: usize) -> Self
    where
        T: Default,
    {
        Self {
            block_size,
            sum: T::default(),
            sum_count: 0,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T>
    where
        T: 'static + Copy + core::ops::Add<Output=T> + core::ops::Div<Output=T> + Default,
        usize: AsPrimitive<T>,
    {
        self.sum = self.sum.clone() + item;
        self.sum_count += 1;

        if self.sum_count >= self.block_size {
            let average = self.sum.clone() / self.block_size.as_();
            self.sum = T::default();
            self.sum_count = 0;

            Some(average)
        } else {
            None
        }
    }
}
