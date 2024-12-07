#![cfg_attr(test, no_std)]
extern crate alloc;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

pub struct ListItem<'a, T> {
    value: T,
    next: Option<NonNull<ListItem<'a, T>>>,
    marker: PhantomData<&'a T>,
}

pub struct LinkedList<'a, T> {
    head: Option<NonNull<ListItem<'a, T>>>,
    last: Option<NonNull<ListItem<'a, T>>>,
    marker: PhantomData<&'a T>,
}

impl<T> ListItem<'_, T> {
    pub const fn new(value: T) -> Self {
        ListItem {
            value,
            next: None,
            marker: PhantomData,
        }
    }
}

impl<T> Deref for ListItem<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for ListItem<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Default for LinkedList<'_, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> LinkedList<'a, T> {
    pub const fn new() -> Self {
        LinkedList {
            head: None,
            last: None,
            marker: PhantomData,
        }
    }

    pub fn push_back(&mut self, item: &'a mut ListItem<'a, T>) {
        let ptr = unsafe { NonNull::new_unchecked(item as *mut ListItem<T>) };
        let prev_last = self.last.replace(ptr);
        if prev_last.is_none() {
            self.head = Some(ptr);
        } else if let Some(mut i) = prev_last {
            unsafe { i.as_mut().next = Some(ptr) };
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.head
            .map(|ptr| unsafe { &mut *ptr.as_ptr() }.deref_mut())
    }

    pub fn pop_front(&mut self) -> Option<&'a mut ListItem<'a, T>> {
        let result = self.head.take();
        let next = result.and_then(|mut ptr| unsafe { ptr.as_mut().next });
        if next.is_none() {
            self.last = None;
        }
        self.head = next;
        result.map(|ptr| unsafe { &mut *ptr.as_ptr() })
    }
}

#[cfg(test)]
mod test {
    use super::LinkedList;
    use super::ListItem;

    #[test]
    fn test_list() {
        let mut item1 = ListItem::new(1);
        let mut item2 = ListItem::new(2);
        let mut item3 = ListItem::new(3);
        let mut list = LinkedList::new();

        list.push_back(&mut item1);
        assert_eq!(Some(&mut 1), list.front_mut());

        list.push_back(&mut item2);
        list.push_back(&mut item3);

        assert_eq!(Some(&mut 1), list.front_mut());
        let result1: &u32 = list.pop_front().unwrap();
        assert_eq!(Some(&mut 2), list.front_mut());
        let result2: &u32 = list.pop_front().unwrap();
        assert_eq!(Some(&mut 3), list.front_mut());
        let result3: &u32 = list.pop_front().unwrap();
        assert_eq!(1, *result1);
        assert_eq!(2, *result2);
        assert_eq!(3, *result3);

        assert!(list.is_empty());

        let mut item4 = ListItem::new(4);
        let mut item5 = ListItem::new(5);
        list.push_back(&mut item4);
        list.push_back(&mut item5);

        let result4: &u32 = list.pop_front().unwrap();
        let result5: &u32 = list.pop_front().unwrap();
        assert_eq!(4, *result4);
        assert_eq!(5, *result5);

        assert!(list.is_empty());
    }

    #[test]
    fn test_list_boxed() {
        use alloc::boxed::Box;

        let mut boxed_item1 = Box::new(ListItem::new(1));
        let mut boxed_item2 = Box::new(ListItem::new(2));
        let mut boxed_item3 = Box::new(ListItem::new(3));
        let mut list = LinkedList::new();

        list.push_back(&mut boxed_item1);
        assert_eq!(Some(&mut 1), list.front_mut());

        list.push_back(&mut boxed_item2);
        list.push_back(&mut boxed_item3);

        assert_eq!(Some(&mut 1), list.front_mut());
        let result1: &u32 = list.pop_front().unwrap();
        assert_eq!(Some(&mut 2), list.front_mut());
        let result2: &u32 = list.pop_front().unwrap();
        assert_eq!(Some(&mut 3), list.front_mut());
        let result3: &u32 = list.pop_front().unwrap();
        assert_eq!(1, *result1);
        assert_eq!(2, *result2);
        assert_eq!(3, *result3);

        assert!(list.is_empty());

        let mut boxed_item4 = Box::new(ListItem::new(4));
        let mut boxed_item5 = Box::new(ListItem::new(5));
        list.push_back(&mut boxed_item4);
        list.push_back(&mut boxed_item5);

        let result4: &u32 = list.pop_front().unwrap();
        let result5: &u32 = list.pop_front().unwrap();
        assert_eq!(4, *result4);
        assert_eq!(5, *result5);

        assert!(list.is_empty());
    }

    #[test]
    fn test_list_boxed_multibyte() {
        use alloc::boxed::Box;

        #[derive(PartialEq, Debug)]
        pub struct Point {
            x: u32,
            y: u32,
        }

        impl Point {
            fn new(x: u32, y: u32) -> Self {
                Point { x, y }
            }
        }

        let mut boxed_item1 = Box::new(ListItem::new(Point::new(1, 2)));
        let mut boxed_item2 = Box::new(ListItem::new(Point::new(2, 4)));
        let mut boxed_item3 = Box::new(ListItem::new(Point::new(3, 6)));
        let mut list = LinkedList::new();

        list.push_back(&mut boxed_item1);
        assert_eq!(Some(&mut Point::new(1, 2)), list.front_mut());

        list.push_back(&mut boxed_item2);
        list.push_back(&mut boxed_item3);

        assert_eq!(Some(&mut Point::new(1, 2)), list.front_mut());
        let result1: &Point = list.pop_front().unwrap();
        assert_eq!(Some(&mut Point::new(2, 4)), list.front_mut());
        let result2: &Point = list.pop_front().unwrap();
        assert_eq!(Some(&mut Point::new(3, 6)), list.front_mut());
        let result3: &Point = list.pop_front().unwrap();
        assert_eq!(Point::new(1, 2), *result1);
        assert_eq!(Point::new(2, 4), *result2);
        assert_eq!(Point::new(3, 6), *result3);

        assert!(list.is_empty());

        let mut boxed_item4 = Box::new(ListItem::new(Point::new(4, 8)));
        let mut boxed_item5 = Box::new(ListItem::new(Point::new(5, 10)));
        list.push_back(&mut boxed_item4);
        list.push_back(&mut boxed_item5);

        let result4: &Point = list.pop_front().unwrap();
        let result5: &Point = list.pop_front().unwrap();
        assert_eq!(Point::new(4, 8), *result4);
        assert_eq!(Point::new(5, 10), *result5);

        assert!(list.is_empty());
    }
}
