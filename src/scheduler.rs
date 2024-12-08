use core::cell::UnsafeCell;

use crate::linked_list::{LinkedList, ListItem};
use crate::mutex::Mutex;
use crate::task::Task;

pub struct Scheduler<'a> {
    ready: Mutex<UnsafeCell<LinkedList<'a, Task<'a>>>>,
}

impl<'a> Scheduler<'a> {
    pub const fn new() -> Self {
        Scheduler {
            ready: Mutex::new(UnsafeCell::new(LinkedList::new())),
        }
    }

    pub fn push_back(&self, item: &'a mut ListItem<'a, Task<'a>>) {
        unsafe { self.ready.lock().get().as_mut().unwrap().push_back(item) };
    }

    fn schedule_next(&self) {
        // let current = self.ready.pop_front().unwrap();
        // self.ready.push_back(current);
        unsafe { self.ready.lock().get().as_mut().unwrap().rotate() };
    }

    pub fn exec(&self) -> ! {
        loop {
            let current = unsafe { self.ready.lock().get().as_mut().unwrap().front_mut() };
            match current {
                None => {
                    unimplemented!();
                }
                Some(p) => {
                    p.exec();
                }
            }
            self.schedule_next();
        }
    }

    pub fn current_task(&self) -> Option<&mut Task<'a>> {
        unsafe { self.ready.lock().get().as_mut().unwrap().front_mut() }
    }
}

impl Default for Scheduler<'_> {
    fn default() -> Self {
        Self::new()
    }
}
