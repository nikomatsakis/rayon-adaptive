//! This module contains all traits enabling us to express some parallelism.
use scheduling::{schedule, schedule2, Policy};
use std;
use std::collections::LinkedList;

pub trait AdaptiveWork: Sized + Send {
    type Output: Mergeable;
    /// Work locally.
    fn work(&mut self, limit: usize);
    /// Get final output.
    fn output(self) -> Self::Output;
    /// Return how much work is left.
    fn remaining_length(&self) -> usize;
    /// We were stolen. Let's cut into two parts.
    fn split(self) -> (Self, Self);
    /// Launch computations with given scheduling policy.
    fn schedule(self, policy: Policy) -> Self::Output {
        schedule2(self, policy)
    }
}

pub trait Divisible: Sized + Send {
    /// Divide ourselves.
    fn split(self) -> (Self, Self);
    /// Return our length.
    fn len(&self) -> usize;
    /// apply work function with given policy.
    fn work<M, F>(self, work_function: F, policy: Policy) -> M
    where
        M: Mergeable,
        F: Fn(Self, usize) -> (Option<Self>, M) + Sync,
    {
        //Note: we used to have a `Block` trait REQUIRING a work_function.
        //it's better now because we can compose Divisibles as we like without
        //caring about the function applied to them.
        schedule(self, &work_function, policy)
    }
}

/// All outputs must implement this trait.
pub trait Mergeable: Sized + Send {
    /// Merge two outputs into one.
    fn fuse(self, other: Self) -> Self;
}

impl Mergeable for () {
    fn fuse(self, _other: Self) -> Self {
        ()
    }
}

impl<T: Send> Mergeable for LinkedList<T> {
    fn fuse(self, other: Self) -> Self {
        let mut left = self;
        let mut right = other; // TODO: change type of self and other ?
        left.append(&mut right);
        left
    }
}

impl<'a, T: Sync> Divisible for &'a [T] {
    fn len(&self) -> usize {
        (*self as &[T]).len()
    }
    fn split(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.split_at(mid)
    }
}

//TODO: I don't get why the compiler requires send here
impl<'a, T: 'a + Sync + Send> Divisible for &'a mut [T] {
    fn len(&self) -> usize {
        (*self as &[T]).len()
    }
    fn split(self) -> (Self, Self) {
        let mid = self.len() / 2;
        self.split_at_mut(mid)
    }
}

//TODO: macroize all that stuff ; even better : derive ?
impl<A: Divisible, B: Divisible> Divisible for (A, B) {
    fn len(&self) -> usize {
        std::cmp::min(self.0.len(), self.1.len())
    }
    fn split(self) -> (Self, Self) {
        let (left_a, right_a) = self.0.split();
        let (left_b, right_b) = self.1.split();
        ((left_a, left_b), (right_a, right_b))
    }
}

impl<A: Divisible, B: Divisible, C: Divisible> Divisible for (A, B, C) {
    fn len(&self) -> usize {
        std::cmp::min(self.0.len(), std::cmp::min(self.1.len(), self.2.len()))
    }
    fn split(self) -> (Self, Self) {
        let (left_a, right_a) = self.0.split();
        let (left_b, right_b) = self.1.split();
        let (left_c, right_c) = self.2.split();
        ((left_a, left_b, left_c), (right_a, right_b, right_c))
    }
}
