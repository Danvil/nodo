// Copyright 2023 by David Weikersdorfer. All rights reserved.

use core::ops;
use std::collections::vec_deque;
use std::collections::VecDeque;

/// The front stage of StageQueue
pub struct FrontStage<T> {
    items: VecDeque<T>,
    capacity: usize,
}

/// The back stage of StageQueue
pub struct BackStage<T> {
    items: VecDeque<T>,
    capacity: usize,
    overflow_policy: OverflowPolicy,
}

/// Push policy in case the back stage is at capacity when an item is pushed.
pub enum OverflowPolicy {
    /// An error code is returned and the item is not added to the queue.
    Reject,

    /// The oldest item is removed to make room for the new item.
    Forget,

    /// Queue capacity is increased to fit the new item.
    Resize(StrictlyIncreasingLinear),
}

/// A strictly increasing linear function of the form `f(x) = a*x + b`
pub struct StrictlyIncreasingLinear {
    addend: usize,
    factor: usize,
}

impl<T> FrontStage<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.items.clear()
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    pub fn drain<R>(&mut self, range: R) -> vec_deque::Drain<'_, T>
    where
        R: ops::RangeBounds<usize>,
    {
        self.items.drain(range)
    }
}

impl<T> ops::Index<usize> for FrontStage<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.items[idx]
    }
}

impl<T> ops::IndexMut<usize> for FrontStage<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.items[idx]
    }
}

impl<T> BackStage<T> {
    pub fn new(capacity: usize, overflow_policy: OverflowPolicy) -> Self {
        Self {
            items: VecDeque::with_capacity(capacity),
            capacity,
            overflow_policy,
        }
    }

    pub fn overflow_policy(&self) -> &OverflowPolicy {
        &self.overflow_policy
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.items.len() == self.capacity {
            match &self.overflow_policy {
                OverflowPolicy::Reject => return Err(value),
                OverflowPolicy::Forget => {
                    self.items.pop_front();
                }
                OverflowPolicy::Resize(sil) => {
                    self.capacity = sil.eval(self.capacity);
                    self.items.reserve_exact(self.capacity);
                }
            }
        }
        self.items.push_back(value);
        Ok(())
    }

    /// Clears the front stage and moves all items from the backstage to the front stage
    pub fn sync(&mut self, other: &mut FrontStage<T>) {
        other.items.clear();

        if matches!(self.overflow_policy, OverflowPolicy::Resize { .. }) {
            other.items.reserve_exact(self.capacity());
        } else {
            assert_eq!(other.capacity(), self.capacity());
        };

        std::mem::swap(&mut self.items, &mut other.items);
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.items.iter()
    }

    pub fn drain_all(&mut self) -> std::collections::vec_deque::Drain<'_, T> {
        self.items.drain(..)
    }

    pub fn clear(&mut self) {
        self.items.clear()
    }
}

impl StrictlyIncreasingLinear {
    pub fn from_addend(addend: usize) -> Self {
        Self::new(addend, 1)
    }

    pub fn from_factor(factor: usize) -> Self {
        Self::new(0, factor)
    }

    pub fn new(addend: usize, factor: usize) -> Self {
        match (addend, factor) {
            (_, 0) => panic!("`factor` must not be 0."),
            (0, 1) => panic!("If `addend` is 0, `factor` must be at least 2"),
            (addend, factor) => Self { addend, factor },
        }
    }

    /// Compute the next value. Will panic if the next value cannot be represented as a usize.
    pub fn eval(&self, current: usize) -> usize {
        let next = if current == 0 && self.addend == 0 {
            1
        } else {
            current
                .checked_mul(self.factor)
                .unwrap()
                .checked_add(self.addend)
                .unwrap()
        };

        if next <= current {
            unreachable!(
                "next value not larger than current.
                 current: {current}, addend: {}, factor: {}, next: {next}",
                self.addend, self.factor
            );
        }

        next
    }
}

#[cfg(test)]
mod tests {
    use crate::channels::stage_queue::OverflowPolicy;
    use crate::channels::stage_queue::StrictlyIncreasingLinear;
    use crate::channels::BackStage;
    use crate::channels::FrontStage;

    pub struct StageQueue<T> {
        back: BackStage<T>,
        front: FrontStage<T>,
    }

    impl<T> StageQueue<T> {
        pub fn new(capacity: usize, policy: OverflowPolicy) -> StageQueue<T> {
            StageQueue {
                back: BackStage::new(capacity, policy),
                front: FrontStage::new(capacity),
            }
        }

        pub fn capacity(&self) -> usize {
            self.back.capacity()
        }

        pub fn push(&mut self, value: T) -> Result<(), T> {
            self.back.push(value)
        }

        pub fn sync(&mut self) {
            self.back.sync(&mut self.front);
        }

        pub fn len(&mut self) -> usize {
            self.front.len()
        }

        pub fn is_empty(&self) -> bool {
            self.front.is_empty()
        }

        pub fn pop(&mut self) -> Option<T> {
            self.front.pop()
        }
    }

    #[test]
    fn test_push_resize() {
        let mut sq = StageQueue::new(
            1,
            OverflowPolicy::Resize(StrictlyIncreasingLinear::from_factor(2)),
        );
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Ok(()));
        assert_eq!(sq.capacity(), 2);

        assert_eq!(sq.pop(), None);
        sq.sync();
        assert_eq!(sq.pop(), Some(31));
        assert_eq!(sq.pop(), Some(42));

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 2);
        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 2);
        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 4);
    }

    #[test]
    fn test_push_reject() {
        let mut sq = StageQueue::new(1, OverflowPolicy::Reject);
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Err(42));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.pop(), None);
        sq.sync();
        assert_eq!(sq.pop(), Some(31));
        assert_eq!(sq.pop(), None);

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 1);
    }

    #[test]
    fn test_push_forget() {
        let mut sq = StageQueue::new(1, OverflowPolicy::Forget);
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Ok(()));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.pop(), None);
        sq.sync();
        assert_eq!(sq.pop(), Some(42));
        assert_eq!(sq.pop(), None);

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 1);
    }

    #[test]
    fn test_strictly_increasing_linear() {
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::from_addend(1));
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::from_factor(2));
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::new(2, 1));
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::new(1, 2));
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::new(2, 3));
    }

    fn test_strictly_increasing_linear_impl(f: StrictlyIncreasingLinear) {
        let mut x = 0;
        for _ in 0..10 {
            let xn = f.eval(x);
            assert!(xn > x);
            x = xn;
        }
    }

    #[test]
    #[should_panic]
    fn test_strictly_increasing_linear_panic_2() {
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::from_addend(0));
    }

    #[test]
    #[should_panic]
    fn test_strictly_increasing_linear_panic_1() {
        test_strictly_increasing_linear_impl(StrictlyIncreasingLinear::from_factor(1));
    }

    #[test]
    fn test_strictly_increasing_linear_addend_1() {
        let sil = StrictlyIncreasingLinear::from_addend(1);
        for i in 0..100 {
            assert_eq!(sil.eval(i), i + 1);
        }
    }

    #[test]
    fn test_strictly_increasing_linear_factor_2() {
        let sil = StrictlyIncreasingLinear::from_factor(2);
        assert_eq!(sil.eval(0), 1);
        for i in 1..100 {
            assert_eq!(sil.eval(i), 2 * i);
        }
    }
}
