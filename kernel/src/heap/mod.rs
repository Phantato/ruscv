mod free_list;

use core::{
    alloc::Layout,
    cmp::{max, min},
    mem::size_of,
    ptr::NonNull,
};

use self::free_list::FreeList;

pub struct Heap<const ORDER: usize> {
    free_list: [FreeList; ORDER],
}

impl<const ORDER: usize> Heap<ORDER> {
    pub fn new() -> Self {
        Self {
            free_list: [FreeList::new(); ORDER],
        }
    }
    pub unsafe fn add(&mut self, mut start: usize, mut end: usize) {
        let unit = size_of::<usize>();
        let mask = !unit + 1;
        start = (start + unit - 1) & mask;
        end = end & mask;
        assert!(start <= end);

        while start + unit <= end {
            let lowbit = start & (!start + 1);
            let size = min(lowbit, prev_power_of_two(end - start));
            self.free_list[size.trailing_zeros() as usize].push(start as *mut usize);
            start += size
        }
    }
    pub fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, ()> {
        let size = max(
            max(layout.size().next_power_of_two(), layout.align()),
            size_of::<usize>(),
        );
        let class = size.trailing_zeros() as usize;
        let exists = self
            .free_list
            .iter()
            .position(|l| !l.is_empty())
            .ok_or(())?;

        for i in (class..exists).rev() {
            let block = self.free_list[i + 1].pop().ok_or(())?;
            unsafe {
                self.free_list[i].push((block as usize + (1 << i)) as *mut usize);
                self.free_list[i].push(block);
            }
        }
        NonNull::new(
            self.free_list[class]
                .pop()
                .expect("current block should have free space now") as *mut u8,
        )
        .ok_or(())
    }
    pub fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let size = max(
            max(layout.size().next_power_of_two(), layout.align()),
            size_of::<usize>(),
        );
        let mut class = size.trailing_zeros() as usize;
        unsafe {
            self.free_list[class].push(ptr.as_ptr() as *mut usize);
            let mut p = ptr.as_ptr() as usize;
            while class < ORDER {
                let buddy = p ^ (1 << class);
                match self.free_list[class].iter().find(|n| *n == buddy) {
                    Some(mut n) => {
                        n.pop();
                        self.free_list[class].pop();
                        p = min(p, buddy);
                        class += 1;
                        self.free_list[class].push(p as *mut usize);
                    }
                    None => break,
                }
            }
        }
    }
}

fn prev_power_of_two(num: usize) -> usize {
    1 << (8 * (size_of::<usize>()) - num.leading_zeros() as usize - 1)
}
