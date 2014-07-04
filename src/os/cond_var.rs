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

use core::intrinsics::abort;
use core::option::{None, Some};
use core::ptr::{null, RawPtr};
use core::ty::Unsafe;
use core::kinds::marker;
use core::kinds::Share;

use hal::cortex_m3::sched::NoInterrupts;
use lib::queue::{Queue, Node};
use os::task::{TaskDescriptor, Tasks};

struct WaitEntry {
  task: *mut TaskDescriptor,
  signalled: *const Unsafe<*const CondVar>,
}

pub struct WaitMany<'a, T> {
  var: &'a CondVar,
  entry: WaitEntry,
  value: T,
}

impl<'a, T> WaitMany<'a, T> {
  pub fn new(var: &'a CondVar, value: T) -> WaitMany<'a, T> {
    WaitMany {
      var: var,
      entry: WaitEntry {
        task: Tasks.current_task() as *mut TaskDescriptor,
        signalled: null(),
      },
      value: value
    }
  }
}

pub struct CondVar {
  waiting: Queue<*mut WaitEntry>
}

pub static COND_VAR_INIT: CondVar = CondVar {
  waiting: Queue {
    head: Unsafe { value: 0 as *mut Node<*mut WaitEntry>, marker1: marker::InvariantType },
    tail: Unsafe { value: 0 as *mut Node<*mut WaitEntry>, marker1: marker::InvariantType },
  }
};

impl CondVar {
  pub fn new() -> CondVar {
    CondVar { waiting: Queue::new() }
  }

  /// Wait on a condition variable
  pub fn wait<'a>(&'a self) {
    /*
     * The signalling thread is responsible for removing the waiting
     * thread which ensures that a signal wakes up exactly one thread
     * whenever there is one waiting.
    */
    unsafe {
      let crit = NoInterrupts::new();
      let mut signalled: Unsafe<*const CondVar> = Unsafe::new(null());
      let mut waiting = Node::new(&mut WaitEntry {
        task: Tasks.current_task() as *mut TaskDescriptor,
        signalled: &signalled
      } as *mut WaitEntry);
      self.waiting.push(&mut waiting, &crit);
      Tasks.current_task().block(crit);
    }
  }

  pub fn wait_many<'a, T>(waits: &mut [WaitMany<'a, T>]) -> T {
    unsafe {
      let crit = NoInterrupts::new();
      let mut signalled: Unsafe<*const CondVar> = Unsafe::new(null());
      for wait in waits.iter().enumerate() {
        wait.entry.signalled = &signalled;
        wait.var.waiting.push(&mut wait.entry, &crit);
      }
      Tasks.current_task().block(crit);

      // Figure out who woke us up
      for wait in waits.iter() {
        if &wait.var == signalled {
          return wait.value;
        }
      }
      abort();
    }
  }

  /// Wake up a thread waiting on a condition variable
  pub fn signal<'a>(&'a self) {
    unsafe {
      let crit = NoInterrupts::new();
      loop {
        match self.waiting.pop(&crit) {
          None => return,
          Some(e) => {
            let entry: &WaitEntry = &*(*e).data;
            let signalled: *mut *const CondVar = (*entry.signalled).get();
            if (*signalled).is_null() {
              *signalled = &*self as *const CondVar;
              (*entry.task).unblock(&crit);
              return;
            }
          }
        }
      }
    }
  }

  /// Wake up all threads waiting on a condition variable
  pub fn broadcast<'a>(&'a self) {
    unsafe {
      let crit = NoInterrupts::new();
      loop {
        match self.waiting.pop(&crit) {
          None => return,
          Some(e) => {
            let entry: &WaitEntry = &*(*e).data;
            let signalled: *mut *const CondVar = (*entry.signalled).get();
            if (*signalled).is_null() {
              *signalled = &*self as *const CondVar;
              (*entry.task).unblock(&crit);
            }
          }
        }
      }
    }
  }
}

impl Share for CondVar {}
