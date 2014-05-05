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

use os::mutex::{Mutex, MUTEX_INIT, Guard};
use os::cond_var::{CondVar, COND_VAR_INIT};
use hal::cortex_m3::sched::NoInterrupts;
use lib::shared::{Shared};
use core::option::{Option, None, Some};
use core::vec::Vec;
use core::intrinsics::abort;
use core::container::Container;
use core::ptr::RawPtr;

use core::kinds::marker;
use core::ty::Unsafe;

#[path="../../lib/ioreg.rs"] mod ioreg;

pub enum Error {
  Nack
}

pub enum State {
  Idle,
  Failed(Error),
  RxStart(*mut u8, uint),
  Rx(*mut u8, uint),
  Tx(*const u8, uint)
}

pub struct I2C {
  lock: Mutex,
  state: Shared<State>,
  irq: CondVar,
  reg: &'static reg::I2CRegs,
}

pub static mut i2c0: I2C = I2C {
    lock: MUTEX_INIT,
    state: Shared {value: Unsafe{value: Idle, marker1: marker::InvariantType}},
    irq: COND_VAR_INIT,
    reg: &reg::I2C0
};

pub struct Context<'a>(&'static I2C, Guard<'a>);

pub struct Address(u8);

impl I2C {
  pub fn start<'a>(&'static self) -> Context<'a>{
    Context(self, self.lock.lock())
  }
}

impl<'a> Context<'a> {
  fn finish(&self) -> Option<Error> {
    let &Context(ref i2c,_) = self;
    unsafe {
      let crit = NoInterrupts::new();
      let state = i2c.state.borrow(&crit);
      match *state {
        Idle => None,
        Failed(err) => Some(err),
        _ => abort()
      }
    }
  }

  pub fn write(&self, Address(addr): Address, buffer: &Vec<u8>) -> Option<Error> {
    let &Context(ref i2c,_) = self;

    // ensure STOP symbol has been sent
    while i2c.reg.S() & I2C_S_BUSY != 0 {}
    i2c.reg.set_C1(i2c.reg.C1() | I2C_C1_IICEN | I2C_C1_IICIE | I2C_C1_MST | I2C_C1_TX);
    i2c.reg.set_D(addr << 1);
    {
      let crit = NoInterrupts::new();
      let mut state = i2c.state.borrow(&crit);
      *state = Tx(buffer.as_ptr(), buffer.len());
    }
    i2c.irq.wait();
    self.finish()
  }

  pub fn read(&self, Address(addr): Address, buffer: &mut Vec<u8>) -> Option<Error> {
    let &Context(ref i2c,_) = self;

    // ensure STOP symbol has been sent
    while i2c.reg.S() & I2C_S_BUSY != 0 {}
    i2c.reg.set_C1(i2c.reg.C1() | I2C_C1_IICEN | I2C_C1_IICIE | I2C_C1_MST | I2C_C1_TX);
    i2c.reg.set_D(addr << 1);
    {
      let crit = NoInterrupts::new();
      let mut state = i2c.state.borrow(&crit);
      *state = RxStart(buffer.as_mut_ptr(), buffer.len());
    }
    i2c.irq.wait();
    self.finish()
  }
}

static I2C_C1_TXAK  : u8 = 1<<3;
static I2C_C1_TX    : u8 = 1<<4;
static I2C_C1_MST   : u8 = 1<<5;
static I2C_C1_IICIE : u8 = 1<<6;
static I2C_C1_IICEN : u8 = 1<<7;

static I2C_S_RXAK   : u8 = 1<<0;
static I2C_S_BUSY   : u8 = 1<<5;
static I2C_S_TCF    : u8 = 1<<7;

fn irq_handler(i2c: &I2C) {
  let crit = NoInterrupts::new(); // FIXME
  let mut state = i2c.state.borrow(&crit);
  let signal = || {i2c.irq.signal()};
  let status = i2c.reg.S();
  match *state {
    Idle | Failed(_) => unsafe { abort() }, // spurious interrupt
    RxStart(_, _) if status & I2C_S_RXAK != 0 => {
      // premature nack
      *state = Failed(Nack);
      signal();
    },
    RxStart(d, rem) => {
      i2c.reg.set_C1(i2c.reg.C1() & !I2C_C1_TX);
      i2c.reg.D(); // throw away byte
      *state = Rx(d, rem);
    },
    Rx(d, rem) => {
      unsafe { *d = i2c.reg.D(); }
      match rem {
        1 => {
          // last byte has been recieved
          *state = Idle;
          signal();
        },
        _ => {
          if rem == 2 {
            // second-to-last byte has been recieved
            // send NACK with last byte
            i2c.reg.set_C1(i2c.reg.C1() | I2C_C1_TXAK);
          }
          unsafe {
            *d = i2c.reg.D();
            *state = Rx(d.offset(1), rem-1);
          }
        }
      }
    },
    Tx(d, rem) if status & I2C_S_RXAK != 0 => {
      *state = Failed(Nack);
      signal();
    },
    Tx(d, rem) => {
      unsafe {
        i2c.reg.set_D(*d);
        match rem {
          0 => {
            *state = Idle;
            signal();
          },
          _ => *state = Tx(d.offset(1), rem-1)
        }
      }
    },
  }
}

mod reg {
  use lib::volatile_cell::VolatileCell;

  ioreg!(I2CRegs: u8, A1, F, C1, S, D, C2, FLT, RA, SMB, A2, SLTH, SLTL)
  reg_rw!(I2CRegs, u8, A1,      set_A1,      A1)
  reg_rw!(I2CRegs, u8, C1,      set_C1,      C1)
  reg_rw!(I2CRegs, u8, S,       set_S,       S)
  reg_rw!(I2CRegs, u8, D,       set_D,       D)
  reg_rw!(I2CRegs, u8, C2,      set_C2,      C2)

  extern {
    #[link_name="iomem_I2C0"] pub static I2C0: I2CRegs;
  }
}

#[allow(dead_code)]
fn I2C0_Handler() {
  unsafe { // This will soon be unnecessary
    irq_handler(&i2c0);
  }
}
