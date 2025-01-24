// Copyright 2023 by David Weikersdorfer. All rights reserved.

use crate::channels::SyncResult;
use core::ops;
use std::collections::{vec_deque, VecDeque};

/// The front stage of StageQueue
pub struct FrontStage<T> {
    items: VecDeque<T>,
    capacity: usize,
}

/// The back stage of StageQueue
pub struct BackStage<T> {
    items: VecDeque<T>,
    overflow_policy: OverflowPolicy,
    retention_policy: RetentionPolicy,
}

/// Push policy in case the back stage is at capacity when an item is pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    /// An error code is returned and the item is not added to the queue.
    Reject(usize),

    /// The oldest item is removed to make room for the new item.
    Forget(usize),

    /// Queue capacity is increased indefinitely to fit the new item. This is a dangerous policy
    /// as it can lead to unbound memory consumption. Consider to use the 'Forget' or 'Reject'
    /// policies instead.
    Resize,
}

/// Describes how leftover items in the front queue are handled when a new frame begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionPolicy {
    /// Keeps leftover items. This policy can only be used if the overflow policy is Forget or
    /// Resize.
    Keep,

    /// Removes leftover items from the queue.
    Drop,

    /// The dev must drain all items out of the queue before the frame ends.
    EnforceEmpty,
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
    pub fn new(overflow_policy: OverflowPolicy, retention_policy: RetentionPolicy) -> Self {
        assert!(
            retention_policy != RetentionPolicy::Keep
                || !matches!(overflow_policy, OverflowPolicy::Reject(_)),
            "Retention policy 'Keep' not allowed with overflow policy 'Reject'"
        );

        let items = match overflow_policy {
            OverflowPolicy::Reject(n) | OverflowPolicy::Forget(n) => VecDeque::with_capacity(n),
            OverflowPolicy::Resize => VecDeque::new(),
        };

        Self {
            items,
            overflow_policy,
            retention_policy,
        }
    }

    pub fn overflow_policy(&self) -> &OverflowPolicy {
        &self.overflow_policy
    }

    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, value: T) -> Result<(), PushError> {
        match self.overflow_policy {
            OverflowPolicy::Reject(n) => {
                if self.items.len() == n {
                    return Err(PushError::Rejected);
                }
            }
            OverflowPolicy::Forget(n) => {
                if self.items.len() == n {
                    self.items.pop_front();
                }
            }
            OverflowPolicy::Resize => {}
        }

        self.items.push_back(value);

        Ok(())
    }

    /// Clears the front stage and moves all items from the backstage to the front stage
    pub fn sync(&mut self, target: &mut FrontStage<T>) -> SyncResult {
        match self.retention_policy {
            RetentionPolicy::Keep => {
                match self.overflow_policy {
                    OverflowPolicy::Forget(n) => {
                        let incoming_count = self.items.len();
                        assert!(incoming_count <= n);
                        let current_count = target.items.len();
                        assert!(current_count <= n);

                        let available_count = n - target.len();
                        let forgotten = if available_count < incoming_count {
                            let delta = incoming_count - available_count;
                            target.drain(0..delta);
                            delta
                        } else {
                            0
                        };

                        target.items.append(&mut self.items);

                        assert_eq!(target.items.len(), (current_count + incoming_count).min(n));
                        assert_eq!(target.items.capacity(), n);
                        assert_eq!(self.items.len(), 0);
                        assert_eq!(self.items.capacity(), n);

                        SyncResult {
                            received: incoming_count,
                            forgotten,
                            ..Default::default()
                        }
                    }
                    OverflowPolicy::Reject(_) => {
                        // SAFETY: This is checked in the constructor.
                        unreachable!();
                    }
                    OverflowPolicy::Resize => {
                        let result = SyncResult {
                            received: self.items.len(),
                            ..Default::default()
                        };

                        target.items.append(&mut self.items);

                        result
                    }
                }
            }
            RetentionPolicy::Drop | RetentionPolicy::EnforceEmpty => {
                let result = SyncResult {
                    received: self.items.len(),
                    dropped: target.items.len(),
                    enforce_empty_violation: self.retention_policy == RetentionPolicy::EnforceEmpty
                        && !target.items.is_empty(),
                    ..Default::default()
                };

                target.items.clear();

                std::mem::swap(&mut self.items, &mut target.items);

                result
            }
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError {
    Rejected,
}

#[cfg(test)]
mod tests {
    use crate::{
        channels::{BackStage, FrontStage, PushError, SyncResult},
        prelude::*,
    };

    pub struct StageQueue<T> {
        back: BackStage<T>,
        front: FrontStage<T>,
    }

    impl<T> StageQueue<T> {
        pub fn new(capacity: usize, policy: OverflowPolicy) -> StageQueue<T> {
            StageQueue {
                back: BackStage::new(policy, RetentionPolicy::Drop),
                front: FrontStage::new(capacity),
            }
        }

        pub fn capacity(&self) -> usize {
            self.back.items.capacity()
        }

        pub fn push(&mut self, value: T) -> Result<(), PushError> {
            self.back.push(value)
        }

        pub fn sync(&mut self) -> SyncResult {
            self.back.sync(&mut self.front)
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
        let mut sq = StageQueue::new(1, OverflowPolicy::Resize);
        assert_eq!(sq.capacity(), 0);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Ok(()));

        assert_eq!(sq.pop(), None);

        assert_eq!(
            sq.sync(),
            SyncResult {
                received: 2,
                ..Default::default()
            }
        );

        assert_eq!(sq.pop(), Some(31));
        assert_eq!(sq.pop(), Some(42));

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.push(53), Ok(()));

        assert_eq!(
            sq.sync(),
            SyncResult {
                received: 3,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_push_reject() {
        let mut sq = StageQueue::new(1, OverflowPolicy::Reject(1));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Err(PushError::Rejected));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.pop(), None);
        assert_eq!(
            sq.sync(),
            SyncResult {
                received: 1,
                ..Default::default()
            }
        );
        assert_eq!(sq.pop(), Some(31));
        assert_eq!(sq.pop(), None);

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 1);
    }

    #[test]
    fn test_push_forget() {
        let mut sq = StageQueue::new(1, OverflowPolicy::Forget(1));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.push(31), Ok(()));
        assert_eq!(sq.push(42), Ok(()));
        assert_eq!(sq.capacity(), 1);

        assert_eq!(sq.pop(), None);
        assert_eq!(
            sq.sync(),
            SyncResult {
                received: 1,
                ..Default::default()
            }
        );
        assert_eq!(sq.pop(), Some(42));
        assert_eq!(sq.pop(), None);

        assert_eq!(sq.push(53), Ok(()));
        assert_eq!(sq.capacity(), 1);
    }
}
