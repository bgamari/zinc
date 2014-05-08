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

use core::option::{Some, None};
use core::cmp::{PartialEq, Eq, PartialOrd, Ord};
use core::ops::{Drop, Add};

use core::ptr::{RawPtr, mut_null};
use core::ty::Unsafe;
use core::kinds::marker::InvariantType;

use lib::shared::Shared;
use lib::queue::{Node, Queue};
use os::cond_var::CondVar;
use hal::cortex_m4::sched::NoInterrupts;
use super::sim::reg::SIM;

#[path="../../lib/ioreg.rs"] mod ioreg;

/// Internal encoding of time
#[deriving(PartialEq, Eq, PartialOrd, Ord)]
pub struct Time { raw: u32 }

/// A time difference in milliseconds
pub struct TimeDelta(u32);

impl Add<TimeDelta, Time> for Time {
  fn add(&self, &TimeDelta(dt): &TimeDelta) -> Time {
    Time {raw: self.raw + dt}
  }
}

struct Timeout {
  cond: CondVar,
  time: Time,
}

impl PartialEq for Timeout {
  fn eq(&self, other: &Timeout) -> bool {self.time == other.time}
}

impl Eq for Timeout { }

impl PartialOrd for Timeout {
  fn lt(&self, other: &Timeout) -> bool {self.time < other.time}
}

impl Ord for Timeout { }

struct TimeoutState {
  queue: Queue<Timeout>,
  lazy_cur_time: Time,
  ref_count: uint
}

static mut timeout: Shared<TimeoutState> =
    Shared {
        value: Unsafe {
            value: TimeoutState {
                queue: Queue {
                    head: Unsafe { value: mut_null(),
                                   marker1: InvariantType
                    },
                    tail: Unsafe { value: mut_null(),
                                   marker1: InvariantType
                    },
                },
                lazy_cur_time: Time {raw: 0},
                ref_count: 0,
            },
            marker1: InvariantType,
        }
    };

fn update_cur_time(crit: &NoInterrupts) -> Time {
  let state = timeout.borrow(crit);
  let t = state.lazy_cur_time.raw;
  let count = t & 0xffff;
  let mut epoch = (t >> 16) & 0xffff;
  reg::LPTMR0.set_CNR(0);
  let now = reg::LPTMR0.CNR();
  if count > now {
    epoch += 1;
  }
  state.lazy_cur_time = Time{ raw: (epoch << 16) | now};
  state.lazy_cur_time
}

/// Block for the given duration
pub fn delay(delay: TimeDelta) {
  let mut t = Node::new(Timeout {cond: CondVar::new(), time: Time {raw: 0}});
  {
    let crit = NoInterrupts::new();
    let cur_time = update_cur_time(&crit);
    t.data.time = cur_time + delay;
    let state = timeout.borrow(&crit);
    state.queue.insert(&mut t, &crit);
    match state.queue.peek() {
      Some(front) if **front == *t => reschedule(&crit),
      _ => {}
    }
  }
  t.cond.wait();
}

fn reschedule(crit: &NoInterrupts) {
  let state = timeout.borrow(crit);
  match state.queue.peek() {
    None if state.ref_count == 0  => {
      // We can stop the timebase
    },
    None => {
      // Schedule a wrap
      reg::LPTMR0.set_CMR(state.lazy_cur_time.raw & 0xffff);
    },
    Some(head) => {
      /* will we have to wrap the epoch before the next timeout? */
      if (**head).time > (state.lazy_cur_time + TimeDelta(0x10000)) {
        // schedule a wrap
        reg::LPTMR0.set_CMR(state.lazy_cur_time.raw & 0xffff);
      } else {
        reg::LPTMR0.set_CMR((**head).time.raw & 0xffff);
      }
    },
  }
}

fn LPT_Handler() {
  let crit = NoInterrupts::new();
  let state = timeout.borrow(&crit);
  loop {
    let cur_time = update_cur_time(&crit);
    match state.queue.peek() {
      None  => break,
      Some(t) if (**t).time > cur_time => break,
      Some(t) => {
        state.queue.pop(&crit);
        (**t).cond.signal()
      },
    }
  }
  reschedule(&crit);
}

/// A reference to keep the LPTMR timebase running
pub struct TimebaseRef {
  dummy: ()
}

impl TimebaseRef {
  pub fn new() -> TimebaseRef {
    let crit = NoInterrupts::new();
    let state = timeout.borrow(&crit);
    state.ref_count += 1;
    TimebaseRef{dummy: ()}
  }
}

impl Drop for TimebaseRef {
  fn drop(&mut self) {
    let crit = NoInterrupts::new();
    let state = timeout.borrow(&crit);
    state.ref_count -= 1;
  }
}

/// Get the current time
pub fn current_time(_: &TimebaseRef) -> Time {
  let crit = NoInterrupts::new();
  update_cur_time(&crit)
}

pub fn setup() {
  SIM.set_SCGC5();
  reg::LPTMR0.set_PSR();
}

mod reg {
  use lib::volatile_cell::VolatileCell;

  ioreg!(LPTMR: u32, CSR, PSR, CMR, CNR)
  reg_rw!(LPTMR, u32, CSR, set_CSR, CSR)
  reg_rw!(LPTMR, u32, PSR, set_PSR, PSR)
  reg_rw!(LPTMR, u32, CMR, set_CMR, CMR)
  reg_rw!(LPTMR, u32, CNR, set_CNR, CNR)

  extern {
    #[link_name="iomem_LPTMR"] pub static LPTMR0: LPTMR;
  }
}
