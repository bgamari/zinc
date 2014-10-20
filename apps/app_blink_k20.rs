#![feature(phase)]
#![crate_type="staticlib"]
#![no_std]

extern crate core;
extern crate zinc;

use core::option::{None, Some};
use zinc::hal::k20::{pin, watchdog};
use zinc::hal::pin::Gpio;
use zinc::hal::cortex_m4::systick;

use zinc::drivers::chario::CharIO;
use zinc::hal::k20::sim::reg::SIM;
use zinc::hal::uart::{Disabled};
use zinc::hal::k20::uart::{UART, UART0};

use zinc::hal::k20::i2c::i2c0;
use zinc::hal::k20::i2c;
use zinc::util::debug;

/// Wait the given number of SysTick ticks
pub fn wait(ticks: u32) {
  let mut n = ticks;
  // Reset the tick flag
  systick::tick();
  loop {
    if systick::tick() {
      n -= 1;
      if n == 0 {
        break;
      }
    }
  }
}

#[no_mangle]
#[allow(unused_variables)]
#[allow(dead_code)]
pub unsafe fn main() {
  watchdog::init(watchdog::Disabled);
  zinc::hal::mem_init::init_stack();
  zinc::hal::mem_init::init_data();

  // Pins for MC HCK (http://www.mchck.org/)
  let led1 = pin::GpioPin::new(pin::PortB, 16, zinc::hal::pin::Out);

  systick::setup(systick::ten_ms().unwrap_or(480000));
  systick::enable();

  SIM.scgc4.set_uart0(true);
  pin::Pin::new(pin::PortA, 2, pin::AltFunction2, pin::PullNone, false);
  pin::Pin::new(pin::PortA, 1, pin::AltFunction2, pin::PullNone, false);
  let uart = UART::new(UART0, 264960, 8, Disabled, 1);

  let debug_token = debug::set_backend(&uart);

  SIM.scgc4.set_i2c0(true);

  let scl = pin::Pin::new(pin::PortB, 0, pin::AltFunction2, pin::PullNone, true);
  let sda = pin::Pin::new(pin::PortB, 1, pin::AltFunction2, pin::PullNone, true);
  let i2c = i2c0.begin();
  loop {
    led1.set_high();
    wait(10);
    //uart.puts("hi\n");
    led1.set_low();
    wait(10);

    let addr = i2c::Address::from_7bit(0x29);
    match i2c.write(addr, &[0x12]) {
      Some(e) => uart.puts("e"),
      None => {},
    }
    let mut ret = [0];
    match i2c.read(addr, &mut ret) {
      Some(e) => uart.puts("e"),
      None => uart.puti(ret[0] as u32),
    }
  }
}

#[no_stack_check]
#[no_mangle]
pub extern fn __morestack() {
  unsafe { core::intrinsics::abort() };
}
