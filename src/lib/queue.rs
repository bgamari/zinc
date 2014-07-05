// Zinc, the bare metal stack for rust.
// Copyright 2014 Ben Gamari <bgamari@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/*!
An intrusive queue.

This module provides a queue data structure supporting O(1)
removal from the head (popping) and insertion to the tail
(pushing). Moreover, an O(n) priority insertion is provided for types
with an ordering.

The rough structure was taken from libsync's mpcs_intrusive.
*/

//
// head                       tail
// | |--->| |--->| |--->| |--->| |
//

use core::ty::Unsafe;
use core::cmp::Ord;
use core::ops::Deref;
use core::ptr::RawPtr;
use core::option::{Option,Some,None};

use hal::arch::sched::NoInterrupts;

/// A queue entry
pub struct Node<T> {
  pub next: Unsafe<*mut Node<T>>,
  pub data: T
}

/// A queue
pub struct Queue<T> {
  pub head: Unsafe<*mut Node<T>>,
  pub tail: Unsafe<*mut Node<T>>
}

fn null_mut<T>() -> *mut T { 0 as *mut T }

impl<T> Queue<T> {
  pub fn new() -> Queue<T> {
    Queue {
      head: Unsafe::new(null_mut()),
      tail: Unsafe::new(null_mut())
    }
  }

  /// Push to tail
  pub unsafe fn push(&self, node: *mut Node<T>, _: &NoInterrupts) {
    if (*self.head.get()).is_null() {
      *self.head.get() = node;
    }
    let tail: *mut Node<T> = *self.tail.get();
    *(*node).next.get() = null_mut();
    if !tail.is_null() {
      *(*tail).next.get() = node;
    }
    *self.tail.get() = node;
  }

  /// Peek at head
  pub unsafe fn peek(&self) -> Option<*mut Node<T>> {
    let head = self.head.get();
    if (*head).is_null() {
      None
    } else {
      Some(*head)
    }
  }

  /// Pop off of head
  pub unsafe fn pop(&self, _: &NoInterrupts) -> Option<*mut Node<T>> {
    let head = self.head.get();
    if (*head).is_null() {
      None
    } else {
      *head = *(**head).next.get();
      Some(*head)
    }
  }
}

impl<T: Ord> Queue<T> {
  /// Priority insertion (higher ends up closer to head)
  pub unsafe fn insert(&self, node: *mut Node<T>, _: &NoInterrupts) {
    let mut next: &Unsafe<*mut Node<T>> = &self.head;
    loop {
      let i: *mut Node<T> = *next.get();
      if i.is_null() {
        break;
      }
      if (*i).data > (*node).data {
        break;
      }
      next = &(*i).next;
    }
    *(*node).next.get() = *next.get();
    *next.get() = node;
    if (*(*node).next.get()).is_null() {
      *self.tail.get() = node;
    }
  }
}

impl<T> Node<T> {
  pub fn new(data: T) -> Node<T> {
    Node { next: Unsafe::new(null_mut()), data: data }
  }
}

impl<T> Deref<T> for Node<T> {
  fn deref<'a>(&'a self) -> &'a T {&self.data}
}
