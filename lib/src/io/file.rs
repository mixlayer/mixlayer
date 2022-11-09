use anyhow::Result;
use std::{
    io::{Read, Write},
    path::Path,
};
use valence_runtime_ffi::ByteBuffer;

extern "C" {
    fn _valence_file_open(path_buf: *const ByteBuffer, mode: i32) -> i32;
    fn _valence_file_close(handle: i32) -> ();
    fn _valence_file_read(handle: i32, buf: *const ByteBuffer) -> i32;
    fn _valence_file_write(handle: i32, buf: *const ByteBuffer) -> i32;
}

pub struct VFile {
    handle: i32,
}

pub enum VFileMode {
    Unknown = 0,
    Read = 1,
    Write = 2,
}

impl VFile {
    pub fn open<P: AsRef<Path>>(path: P, mode: VFileMode) -> Result<Self> {
        let path = path.as_ref();

        if path.is_absolute() {
            return Err(anyhow::Error::msg("absolute paths are disallowed"));
        }

        //TODO may not handle non-unicode characters how we want
        let path = format!("{}", path.display());
        let path_buf: ByteBuffer = path.into();

        let handle = unsafe { _valence_file_open(&path_buf, mode as i32) };

        Ok(Self { handle })
    }

    pub fn close(self) -> Result<()> {
        unsafe { _valence_file_close(self.handle) };

        Ok(())
    }
}

impl Drop for VFile {
    fn drop(&mut self) {
        unsafe { _valence_file_close(self.handle) };
    }
}

impl Write for VFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let bb = ByteBuffer::from_slice(buf);
        let bytes_written = unsafe { _valence_file_write(self.handle, &bb) as usize };
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(()) //no op for now
    }
}

impl Read for VFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bb = ByteBuffer::from_slice(buf);
        let bytes_read = unsafe { _valence_file_read(self.handle, &bb) as usize };
        Ok(bytes_read)
    }
}
