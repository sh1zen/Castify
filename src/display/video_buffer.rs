//! Lock-free triple buffer for video frames
//!
//! This module implements a triple-buffering system that allows concurrent
//! access by a writer (decoder) and reader (GUI renderer) without locks.
//!
//! # Design
//!
//! The triple buffer maintains three buffers:
//! - **Write buffer**: Currently being written by the decoder
//! - **Ready buffer**: Most recently completed write, ready to be swapped to read
//! - **Read buffer**: Currently being read by the GUI renderer
//!
//! The writer and reader can operate independently:
//! - Writer writes to write buffer, then atomically swaps it with ready buffer
//! - Reader reads from read buffer, atomically swaps with ready buffer when available
//!
//! # Safety
//!
//! This implementation uses `UnsafeCell` to allow interior mutability without locks.
//! Safety is guaranteed by the following invariants:
//!
//! 1. At any time, each buffer is owned by at most one index (write, ready, or read)
//! 2. The write index is only modified by the writer
//! 3. The read index is only modified by the reader
//! 4. The ready index is modified by both, but only via atomic compare-exchange
//! 5. No two indices ever point to the same buffer simultaneously
//!
//! The atomic operations ensure that swaps are visible across threads and that
//! no data races can occur.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Lock-free triple buffer for passing data between a single writer and single reader
///
/// # Type Parameters
///
/// - `T`: The type of data stored in each buffer. Must be `Send` for cross-thread access.
///
/// # Example
///
/// ```ignore
/// use castify::display::TripleBuffer;
///
/// // Create a triple buffer with default-initialized Vec<u8>
/// let buffer = TripleBuffer::new(|| Vec::new());
///
/// // Writer thread
/// {
///     let mut write_guard = buffer.write();
///     write_guard.resize(1920 * 1080 * 4, 0);
///     // ... fill with decoded frame data ...
///     write_guard.commit(); // Make available to reader
/// }
///
/// // Reader thread (GUI renderer)
/// if buffer.has_new_frame() {
///     let read_guard = buffer.read();
///     // ... upload to GPU texture ...
/// }
/// ```
pub struct TripleBuffer<T> {
    /// Three internal buffers
    buffers: [UnsafeCell<T>; 3],

    /// Index of the buffer currently being written
    write_idx: AtomicUsize,

    /// Index of the buffer ready to be read
    ready_idx: AtomicUsize,

    /// Index of the buffer currently being read
    read_idx: AtomicUsize,

    /// Flag indicating a new frame is available
    has_new: AtomicBool,
}

// Safety: TripleBuffer can be sent between threads if T can be sent
unsafe impl<T: Send> Send for TripleBuffer<T> {}
// Safety: TripleBuffer can be shared between threads if T can be sent
unsafe impl<T: Send> Sync for TripleBuffer<T> {}

impl<T> TripleBuffer<T> {
    /// Create a new triple buffer with buffers initialized by the given function
    ///
    /// # Arguments
    ///
    /// * `init_fn` - Function to initialize each buffer
    ///
    /// # Example
    ///
    /// ```ignore
    /// let buffer = TripleBuffer::new(|| Vec::with_capacity(1920 * 1080 * 4));
    /// ```
    pub fn new<F>(mut init_fn: F) -> Self
    where
        F: FnMut() -> T,
    {
        Self {
            buffers: [
                UnsafeCell::new(init_fn()),
                UnsafeCell::new(init_fn()),
                UnsafeCell::new(init_fn()),
            ],
            write_idx: AtomicUsize::new(0),
            ready_idx: AtomicUsize::new(1),
            read_idx: AtomicUsize::new(2),
            has_new: AtomicBool::new(false),
        }
    }

    /// Get a write guard for writing a new frame
    ///
    /// This returns a guard that allows writing to the write buffer.
    /// When the guard is committed, the write buffer is swapped with the ready buffer.
    pub fn write(&self) -> WriteGuard<'_, T> {
        let idx = self.write_idx.load(Ordering::Relaxed);
        WriteGuard {
            buffer: self,
            buffer_idx: idx,
        }
    }

    /// Get a read guard for reading the latest frame
    ///
    /// This first swaps the ready buffer with the read buffer (if a new frame is available),
    /// then returns a guard that allows reading from the read buffer.
    pub fn read(&self) -> ReadGuard<'_, T> {
        // Swap ready and read if new frame available
        if self.has_new.load(Ordering::Acquire) {
            let read_idx = self.read_idx.load(Ordering::Relaxed);
            let ready_idx = self.ready_idx.swap(read_idx, Ordering::AcqRel);
            self.read_idx.store(ready_idx, Ordering::Release);
            self.has_new.store(false, Ordering::Release);
        }

        let idx = self.read_idx.load(Ordering::Relaxed);
        ReadGuard {
            buffer: self,
            buffer_idx: idx,
        }
    }

    /// Check if a new frame is available
    pub fn has_new_frame(&self) -> bool {
        self.has_new.load(Ordering::Acquire)
    }

    /// Get a buffer by index (unsafe, for internal use)
    ///
    /// # Safety
    ///
    /// Caller must ensure that:
    /// - The buffer at the given index is not aliased
    /// - The reference lifetime does not outlive the guard
    unsafe fn get_buffer(&self, idx: usize) -> &T {
        // SAFETY: Caller guarantees no aliasing
        unsafe { &*self.buffers[idx].get() }
    }

    /// Get a mutable buffer by index (unsafe, for internal use)
    ///
    /// # Safety
    ///
    /// Caller must ensure that:
    /// - The buffer at the given index is not aliased
    /// - Only one mutable reference exists at a time
    /// - The reference lifetime does not outlive the guard
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_buffer_mut(&self, idx: usize) -> &mut T {
        // SAFETY: Caller guarantees exclusive access
        unsafe { &mut *self.buffers[idx].get() }
    }

    /// Commit the write buffer, making it available to the reader
    fn commit_write(&self) {
        // Swap write and ready buffers
        let write_idx = self.write_idx.load(Ordering::Relaxed);
        let ready_idx = self.ready_idx.swap(write_idx, Ordering::AcqRel);
        self.write_idx.store(ready_idx, Ordering::Release);

        // Signal that a new frame is available
        self.has_new.store(true, Ordering::Release);
    }
}

/// Write guard for triple buffer
///
/// Provides mutable access to the write buffer. When dropped or explicitly committed,
/// the write buffer is swapped with the ready buffer, making it available to readers.
pub struct WriteGuard<'a, T> {
    buffer: &'a TripleBuffer<T>,
    buffer_idx: usize,
}

impl<'a, T> WriteGuard<'a, T> {
    /// Get a mutable reference to the write buffer
    pub fn get_mut(&mut self) -> &mut T {
        // Safety: WriteGuard has exclusive access to the write buffer at buffer_idx
        unsafe { self.buffer.get_buffer_mut(self.buffer_idx) }
    }

    /// Commit this write, making it available to readers
    ///
    /// This consumes the guard and swaps the write buffer with the ready buffer.
    pub fn commit(self) {
        // Commit happens in drop
    }

    /// Commit without consuming the guard (allows reuse)
    pub fn commit_mut(&mut self) {
        self.buffer.commit_write();
    }
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        // Auto-commit on drop
        self.buffer.commit_write();
    }
}

impl<'a, T> std::ops::Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: WriteGuard has exclusive access to buffer_idx
        unsafe { self.buffer.get_buffer(self.buffer_idx) }
    }
}

impl<'a, T> std::ops::DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Read guard for triple buffer
///
/// Provides read-only access to the most recent committed frame.
pub struct ReadGuard<'a, T> {
    buffer: &'a TripleBuffer<T>,
    buffer_idx: usize,
}

impl<'a, T> ReadGuard<'a, T> {
    /// Get a reference to the read buffer
    pub fn get(&self) -> &T {
        // Safety: ReadGuard has shared access to the read buffer at buffer_idx
        unsafe { self.buffer.get_buffer(self.buffer_idx) }
    }
}

impl<'a, T> std::ops::Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_write_read() {
        let buffer = TripleBuffer::new(Vec::<u8>::new);

        // Write some data
        {
            let mut write = buffer.write();
            write.extend_from_slice(&[1, 2, 3, 4]);
            write.commit();
        }

        // Should have new frame
        assert!(buffer.has_new_frame());

        // Read the data
        {
            let read = buffer.read();
            assert_eq!(&**read, &[1, 2, 3, 4]);
        }

        // No new frame after reading
        assert!(!buffer.has_new_frame());
    }

    #[test]
    fn test_multiple_writes() {
        let buffer = TripleBuffer::new(Vec::<u8>::new);

        // Write multiple times
        for i in 0..10 {
            let mut write = buffer.write();
            write.clear();
            write.push(i);
            write.commit();
        }

        // Should have new frame
        assert!(buffer.has_new_frame());

        // Read should get the last write
        {
            let read = buffer.read();
            assert_eq!(&**read, &[9]);
        }
    }

    #[test]
    fn test_concurrent_access() {
        let buffer = Arc::new(TripleBuffer::new(Vec::<u8>::new));
        let buffer_clone = buffer.clone();

        // Writer thread
        let writer = thread::spawn(move || {
            for i in 0..100 {
                let mut write = buffer_clone.write();
                write.clear();
                write.extend_from_slice(&[i, i + 1, i + 2]);
                write.commit();
                thread::sleep(Duration::from_micros(100));
            }
        });

        // Reader thread
        let reader = thread::spawn(move || {
            let mut read_count = 0;
            for _ in 0..100 {
                if buffer.has_new_frame() {
                    let read = buffer.read();
                    assert_eq!(read.len(), 3); // Should always have 3 elements
                    read_count += 1;
                }
                thread::sleep(Duration::from_micros(100));
            }
            read_count
        });

        writer.join().unwrap();
        let read_count = reader.join().unwrap();

        // Should have read at least some frames
        assert!(read_count > 0);
    }

    #[test]
    fn test_no_data_race() {
        // This test verifies that concurrent access doesn't cause data races
        // by having the writer write incrementing patterns and the reader verify them
        let buffer = Arc::new(TripleBuffer::new(|| vec![0u8; 1024]));
        let buffer_clone = buffer.clone();

        let writer = thread::spawn(move || {
            for i in 0..1000 {
                let mut write = buffer_clone.write();
                let pattern = (i % 256) as u8;
                write.fill(pattern);
                write.commit();
            }
        });

        let reader = thread::spawn(move || {
            for _ in 0..1000 {
                if buffer.has_new_frame() {
                    let read = buffer.read();
                    // Verify all bytes are the same (no partial writes visible)
                    let first = read[0];
                    for &byte in read.iter() {
                        assert_eq!(
                            byte, first,
                            "Data race detected: inconsistent buffer contents"
                        );
                    }
                }
            }
        });

        writer.join().unwrap();
        reader.join().unwrap();
    }

    #[test]
    fn test_write_guard_deref() {
        let buffer = TripleBuffer::new(Vec::<u8>::new);

        let mut write = buffer.write();
        write.push(42);
        write.push(43);
        assert_eq!(write.len(), 2);
        assert_eq!(write[0], 42);
        write.commit();

        let read = buffer.read();
        assert_eq!(read.len(), 2);
        assert_eq!(read[1], 43);
    }

    #[test]
    fn test_auto_commit_on_drop() {
        let buffer = TripleBuffer::new(Vec::<u8>::new);

        // Write and drop without explicit commit
        {
            let mut write = buffer.write();
            write.push(99);
            // Guard dropped here, should auto-commit
        }

        // Should have new frame
        assert!(buffer.has_new_frame());

        let read = buffer.read();
        assert_eq!(&**read, &[99]);
    }
}
