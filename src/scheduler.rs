use crate::linked_list::{LinkedList, ListItem};
use crate::task::Task;

pub struct Scheduler<'a> {
    list: LinkedList<'a, Task<'a>>,
}

impl<'a> Scheduler<'a> {
    pub fn new() -> Self {
        Scheduler {
            list: LinkedList::new(),
        }
    }

    pub fn push_back(&mut self, item: &'a mut ListItem<'a, Task<'a>>) {
        self.list.push_back(item);
    }

    fn schedule_next(&mut self) {
        let current = self.list.pop_front().unwrap();
        self.list.push_back(current);
    }

    pub fn exec(&mut self) -> ! {
        loop {
            let current = self.list.front_mut();
            if current.is_none() {
                unimplemented!();
            }
            if let Some(p) = current {
                p.exec();
            }
            self.schedule_next();
        }
    }
}

impl Default for Scheduler<'_> {
    fn default() -> Self {
        Self::new()
    }
}
