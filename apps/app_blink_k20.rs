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
use zinc::hal::k20::sim;
use zinc::hal::uart::{Disabled};
use zinc::hal::k20::uart::{UART, UART0};

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
  zinc::hal::mem_init::init_stack();
  zinc::hal::mem_init::init_data();
  watchdog::init(watchdog::Disabled);

  // Pins for MC HCK (http://www.mchck.org/)
  let led1 = pin::Pin::new(pin::PortB, 16, pin::Gpio, Some(zinc::hal::pin::Out));

  systick::setup(systick::ten_ms().unwrap_or(480000));
  systick::enable();
  
  SIM.scgc4.set_uart0(true);
  pin::Pin::new(pin::PortA, 2, pin::AltFunction2, None);
  pin::Pin::new(pin::PortA, 1, pin::AltFunction2, None);
  let uart = UART::new(UART0, 264960, 8, Disabled, 1);
  loop {
    led1.set_high();
    wait(10);
    uart.puts("hi\n");
    led1.set_low();
    wait(10);
  }
}
