#![feature(globs, macro_rules)]
 
use hal::cortex_m3::sched::{disable_irqs, enable_irqs};
use os::task::{TaskDescriptor, Tasks, Status, Blocked, Runnable, task_scheduler};
use lib::linked_list::*;
use core::option::{Option, None, Some};
use core::cell::{Cell};
use core::ops::{Drop};

struct WaitingTask<'a> {
  task: &'a TaskDescriptor,
  next: ListCell<'a, WaitingTask<'a>>
}

//deriveLinkedList!(WaitingTask, next)
impl<'a> LinkedList<'a> for WaitingTask<'a> {
    fn get_cell(&'a self) -> &'a ListCell<'a, WaitingTask<'a>> { return &self.next; }
}

pub struct Mutex<'a> {
  owner: Cell<Option<&'a mut TaskDescriptor>>,
  waiting: ListCell<'a, WaitingTask<'a>>
}

impl<'a> Mutex<'a> {
  fn lock(&'a self) {
    disable_irqs();
    match self.owner.get() {
      None    => { },
      Some(owner) => unsafe {
        let task = Tasks.current_task();
        let mut waiting = WaitingTask{task: task, next: ListCell::new()};
        task.status = Blocked;
        self.waiting.append(&waiting);
        enable_irqs();
        task_scheduler();
      }
    }

    self.owner.set(Some(Tasks.current_task()));
    enable_irqs();
  }

  fn unlock(&'a self) {
    disable_irqs();
    self.owner.set(None);
    match self.waiting.pop() {
      None => { },
      Some(nextTask) => unsafe {
        nextTask.task.status = Runnable;
      }
    }
    enable_irqs();
  }
}

pub struct MutexLock<'a>(&'a Mutex<'a>);

impl<'a> MutexLock<'a> {
  fn new(mutex: &'a Mutex<'a>) -> MutexLock<'a> {
    mutex.lock();
    return MutexLock(mutex);
  }
}

impl<'a> Drop for MutexLock<'a> {
  fn drop(&mut self) {
    let &MutexLock(ref mutex) = self;
    mutex.unlock();
  }
}
