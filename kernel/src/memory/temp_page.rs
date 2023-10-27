use super::{
    address::{PhysPageNum, VirtAddr, VirtPageNum},
    page_table::{PTEFlags, PageTable},
};

struct TempPage {
    vpn: VirtPageNum,
}

#[allow(unused)]
impl TempPage {
    pub fn map(&mut self, ppn: PhysPageNum, page_table: &mut PageTable) -> VirtAddr {
        // TODO: use a custome allocator to avoid frame allocate every time
        page_table.map(self.vpn, ppn, PTEFlags::V | PTEFlags::W);
        self.vpn.into()
    }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        page_table.unmap(self.vpn)
    }
}
