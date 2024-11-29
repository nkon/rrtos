use crate::linked_list::{LinkedList, ListItem};
use crate::process::Process;

pub struct Scheduler<'a> {
    list: LinkedList<'a, Process<'a>>,
}

impl<'a> Scheduler<'a> {
    pub fn new() -> Self {
        Scheduler {
            list: LinkedList::new(),
        }
    }

    pub fn push(&mut self, item: &'a mut ListItem<'a, Process<'a>>) {
        self.list.push(item);
    }

    fn schedule_next(&mut self) {
        let current = self.list.pop().unwrap();
        self.list.push(current);
    }

    pub fn exec(&mut self, delay: &mut cortex_m::delay::Delay) -> ! {
        loop {
            let current = self.list.head_mut();
            if current.is_none() {
                unimplemented!();
            }
            if let Some(p) = current {
                p.exec();
            }
            self.schedule_next();
            delay.delay_ms(1000);
        }
    }
}

impl Default for Scheduler<'_> {
    fn default() -> Self {
        Self::new()
    }
}