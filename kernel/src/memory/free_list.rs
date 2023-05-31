use core::{marker::PhantomData, ptr};

pub type FreeList = LinkedList;

#[derive(Clone, Copy)]
pub struct LinkedList {
    next: *mut usize,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    pub const fn new() -> Self {
        Self {
            next: ptr::null_mut(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.next.is_null()
    }
    pub unsafe fn push(&mut self, item: *mut usize) {
        *item = self.next as usize;
        self.next = item;
    }
    pub fn pop(&mut self) -> Option<*mut usize> {
        if !self.is_empty() {
            let item = self.next;
            self.next = unsafe { *item as *mut usize };
            Some(item)
        } else {
            None
        }
    }
    pub fn iter(&mut self) -> Iter {
        Iter {
            prev: ptr::from_mut(&mut self.next) as *mut usize,
            curr: self.next,
            list: PhantomData,
        }
    }
}

pub struct PopableNode<'a> {
    prev: *mut usize,
    curr: *mut usize,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> PopableNode<'a> {
    pub fn pop(&mut self) -> *mut usize {
        let item = self.curr;
        unsafe {
            *self.prev = *item;
            self.curr = *item as *mut usize;
        }
        item
    }
}
impl PartialEq<usize> for PopableNode<'_> {
    fn eq(&self, other: &usize) -> bool {
        self.curr as usize == *other
    }
}

pub struct Iter<'a> {
    prev: *mut usize,
    curr: *mut usize,
    list: PhantomData<&'a LinkedList>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = PopableNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.curr.is_null() {
            true => None,
            false => {
                let ret = Self::Item {
                    prev: self.prev,
                    curr: self.curr,
                    list: self.list,
                };

                self.prev = self.curr;
                self.curr = unsafe { *self.curr as *mut usize };

                Some(ret)
            }
        }
    }
}
