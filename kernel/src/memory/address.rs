use core::{iter::Step, ops::Add};

use super::{PAGE_SIZE, PAGE_SIZE_BITS, PA_WIDTH, PPN_WIDTH, PTE_PER_PAGE, VA_WIDTH, VPN_WIDTH};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtAddr(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct PhysPageNum(pub usize);

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VirtPageNum(pub usize);

impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}

impl PhysAddr {
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 >> PAGE_SIZE_BITS)
    }
    pub fn ceil(&self) -> PhysPageNum {
        PhysPageNum((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS)
    }
    pub unsafe fn get_mut<T>(&self) -> Option<&'static mut T> {
        (self.0 as *mut T).as_mut()
    }
}
impl Add<usize> for PhysAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        PhysAddr(self.0 + rhs)
    }
}

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        Self(v & ((1 << PA_WIDTH) - 1))
    }
}
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        v.0
    }
}
impl VirtAddr {
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 >> PAGE_SIZE_BITS)
    }
    pub fn ceil(&self) -> VirtPageNum {
        VirtPageNum((self.0 + PAGE_SIZE - 1) >> PAGE_SIZE_BITS)
    }
}
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        Self(v & ((1 << VA_WIDTH) - 1))
    }
}

impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        Self(v & ((1 << PPN_WIDTH) - 1))
    }
}

impl VirtPageNum {
    pub fn indexes(&self) -> [usize; 4] {
        let mut vpn = self.0;
        let mut idx = [0usize; 4];
        for i in (0..4).rev() {
            idx[i] = vpn & (PTE_PER_PAGE - 1);
            vpn >>= 9;
        }
        idx
    }
}

impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        Self(v & ((1 << VPN_WIDTH) - 1))
    }
}

impl Step for VirtPageNum {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        usize::steps_between(&start.0, &end.0)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        usize::forward_checked(start.0, count).map(|s| s.into())
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        usize::backward_checked(start.0, count).map(|s| s.into())
    }
}

impl Step for PhysPageNum {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        usize::steps_between(&start.0, &end.0)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        usize::forward_checked(start.0, count).map(|s| s.into())
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        usize::backward_checked(start.0, count).map(|s| s.into())
    }
}
