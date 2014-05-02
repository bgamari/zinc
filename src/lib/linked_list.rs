/*!
 * Intrusive singly linked lists
 */

#![feature(macro_rules)]
#[macro_escape]

use core::option::{Option, Some, None};
use core::cell::{Cell};
use core::cmp::{Eq};
use core::iter::{Iterator};

pub struct ListCell<'a, T>(Cell<Option<&'a T>>);

impl<'a, T: LinkedList<'a>> ListCell<'a, T> {
  pub fn set_next(&self, new: Option<&'a T>) {
    let &ListCell(ref cell) = self;
    cell.set(new);
  }

  pub fn get_next(&self) -> Option<&'a T> {
    let &ListCell(ref cell) = self;
    cell.get()
  }

  pub fn new() -> ListCell<'a, T> {
    ListCell(Cell::new(None))
  }

  #[inline]
  /// Add an element to the end of the list
  pub fn append(&'a self, new: &'a T) {
    new.set_next(None);
    match self.last() {
      None         => self.set_next(Some(new)),
      Some(oldEnd) => oldEnd.set_next(Some(new))
    }
  }

  #[inline]
  /// Add an element to the beginning of the list
  pub fn prepend(&'a self, new: &'a T) {
    match self.get_next() {
      None      => self.set_next(Some(new)),
      Some(old) => {
        new.set_next(Some(old));
        self.set_next(Some(new));
      }
    }
  }

  /// Remove the first element from the list
  pub fn pop(&'a self) -> Option<&'a T> {
    match self.get_next() {
      None       => None,
      Some(next) => {
        self.set_next(next.get_next());
        next.set_next(None);
        Some(next)
      }
    }
  }

  #[inline]
  /// Return the first value in the list
  pub fn head(&'a self) -> Option<&'a T> { self.get_next() }

  #[inline]
  /// Return the last value in the list
  pub fn last(&'a self) -> Option<&'a T> {
    let mut i = self.iter();
    let mut end: Option<&'a T> = None;
    loop {
      match i.next() {
        None       => break,
        Some(next) => end = Some(next),
      }
    }
    end
  }

  #[inline]
  /// Return an iterator over the elements of a list.
  pub fn iter(&'a self) -> LinkedListIter<'a, T> {
    match self.get_next() {
      None    => LinkedListIter(None),
      Some(h) => LinkedListIter(Some(h))
    }
  }
}

/// A type with an intrusive linked-list cell. Simply embed a ListCell
/// into your type, implement get_cell, and get linked-list operations for
/// free.
pub trait LinkedList<'a> {
  fn get_cell(&'a self) -> &'a ListCell<'a, Self>;
  fn set_next(&'a self, next: Option<&'a Self>) { self.get_cell().set_next(next); }
  fn get_next(&'a self) -> Option<&'a Self> { return self.get_cell().get_next(); }
}

#[macro_export]
macro_rules! deriveLinkedList(
    ($typ:ident, $next:ident) => (
        impl<'a> LinkedList<'a> for $typ<'a> {
            fn get_cell(&'a self) -> &'a ListCell<'a, $typ<'a>> { return &self.$next; }
        }
    )
)

/// Remove the element equal to the given value from the list.
pub fn remove<'a, T: Eq+LinkedList<'a>>(head: &'a ListCell<'a, T>, old: &'a T) -> Option<&'a T> {
  let mut cur = head.iter();
  let mut last: &ListCell<T> = head;
  loop {
    match cur.next() {
      None       => return None,
      Some(this) if this == old => {
        last.set_next(this.get_next());
        this.set_next(None);
        return Some(this);
      }
      Some(this) => last = this.get_cell()
    }
  }      
}

/// Insert the given value after the current value
pub fn insert_after<'a, T: LinkedList<'a>>(after: &'a T, value: &'a T) {
  after.get_cell().append(value);
}

pub struct LinkedListIter<'a, T>(Option<&'a T>);

impl<'a, T: LinkedList<'a>> Iterator<&'a T> for LinkedListIter<'a, T> {
  fn next(&mut self) -> Option<&'a T> {
    match *self {
      LinkedListIter(None) => None,
      LinkedListIter(Some(cur)) => {
        match cur.get_next() {
          None => None,
          Some(next) => {
              (*self) = LinkedListIter(Some(next));
              Some(cur)
          }
        }
      }
    }
  }
}

