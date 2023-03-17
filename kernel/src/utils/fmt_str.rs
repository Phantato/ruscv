use core::fmt;

struct FmtStr<'a> {
    buf: &'a mut [u8],
    tail: usize,
}

impl fmt::Write for FmtStr<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let new_tail = self.tail + s.len();
        if new_tail <= self.buf.len() {
            self.buf[self.tail..new_tail].copy_from_slice(s.as_bytes());
            self.tail = new_tail;
            Ok(())
        } else {
            Err(fmt::Error)
        }
    }
}

pub fn _fmt_str<'a>(buf: &'a mut [u8], args: fmt::Arguments) -> Result<&'a str, fmt::Error> {
    let mut f = FmtStr { buf, tail: 0 };
    fmt::write(&mut f, args)?;
    Ok(unsafe { core::str::from_utf8_unchecked(&f.buf[..f.tail]) })
}

#[macro_export]
macro_rules! fmt_str {
    ($buf: expr, $($arg: tt)*) => ($crate::utils::fmt_str::_fmt_str($buf, format_args!($($arg)*)));
}
