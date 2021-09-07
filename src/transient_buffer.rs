use std::{
    collections::VecDeque,
    fmt::{self, Display},
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    mem::size_of,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, RwLock, RwLockReadGuard,
    },
    thread,
    time::Duration,
};
use tempfile::tempfile;

use crate::{
    error::{Result, TexProError},
    slot_data::{Buffer, ChannelPixel, Size, SlotData},
};

/// A buffer that can be either in memory or in storage, getting it puts it in memory.
#[derive(Debug)]
pub enum TransientBuffer {
    Memory(Box<Buffer>),
    Storage(File, Size, AtomicBool), // Turn the contents of this enum into a struct
}

impl TransientBuffer {
    pub fn new(buffer: Box<Buffer>) -> Self {
        Self::Memory(buffer)
    }

    pub fn buffer(&self) -> &Buffer {
        if let Self::Memory(box_buf) = self {
            box_buf
        } else {
            panic!("This should be unreachable when accessed from the outside")
        }
    }

    pub fn size(&self) -> Size {
        match self {
            Self::Memory(box_buffer) => box_buffer.dimensions().into(),
            Self::Storage(_, size, _) => *size,
        }
    }

    pub fn bytes(&self) -> usize {
        self.size().pixel_count() * size_of::<ChannelPixel>()
    }

    pub fn request(&self) {
        if let Self::Storage(_, _, requested) = self {
            requested.store(true, Ordering::Relaxed);
        }
    }

    pub fn requested(&self) -> bool {
        if let Self::Storage(_, _, requested) = self {
            requested.load(Ordering::Relaxed)
        } else {
            false
        }
    }

    pub fn in_memory(&self) -> bool {
        match self {
            Self::Memory(_) => true,
            Self::Storage(_, _, _) => false,
        }
    }

    /// Ensures the `TransientBuffer` is in storage, returns true if it was moved.
    fn move_to_storage(&mut self) -> Result<bool> {
        if let Self::Memory(box_buffer) = self {
            let mut file = tempfile()?;

            for pixel in box_buffer.iter() {
                file.write_all(&pixel.to_ne_bytes())?;
            }

            *self = Self::Storage(file, box_buffer.dimensions().into(), AtomicBool::new(false));

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Ensures the `TransientBuffer` is in memory, returns true if it was moved.
    fn move_to_memory(&mut self) -> Result<bool> {
        if let Self::Storage(file, size, _) = self {
            let buffer_f32: Vec<f32> = {
                let buffer_int: Vec<u8> = {
                    let mut buffer = Vec::<u8>::new();
                    file.seek(SeekFrom::Start(0))?;
                    file.read_to_end(&mut buffer)?;
                    buffer
                };

                let pixel_count = buffer_int.len() / size_of::<ChannelPixel>();
                let mut buffer = Vec::with_capacity(pixel_count);

                for i in (0..buffer_int.len()).step_by(4) {
                    let bytes: [u8; 4] = [
                        buffer_int[i],
                        buffer_int[i + 1],
                        buffer_int[i + 2],
                        buffer_int[i + 3],
                    ];
                    buffer.push(f32::from_ne_bytes(bytes));
                }

                buffer
            };

            *self = Self::Memory(Box::new(
                Buffer::from_raw(size.width, size.height, buffer_f32)
                    .ok_or(TexProError::Generic)?,
            ));

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// A container for a `TransientBuffer`. Keeps track of if its `TransientBuffer` has been retrieved.
#[derive(Debug)]
pub struct TransientBufferContainer {
    transient_buffer: Arc<RwLock<TransientBuffer>>,
    size: Size,
}

impl TransientBufferContainer {
    pub fn new(transient_buffer: Arc<RwLock<TransientBuffer>>) -> Self {
        let size = transient_buffer.read().unwrap().size();

        Self {
            transient_buffer,
            size,
        }
    }

    pub fn transient_buffer(&self) -> RwLockReadGuard<TransientBuffer> {
        loop {
            if let Ok(transient_buffer) = self.transient_buffer.read() {
                if transient_buffer.in_memory() {
                    return transient_buffer;
                } else {
                    transient_buffer.request();
                }
            } else {
                panic!("Lock poisoned");
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn try_transient_buffer(&self) -> Result<RwLockReadGuard<TransientBuffer>> {
        let transient_buffer = self.transient_buffer.try_read()?;

        if transient_buffer.in_memory() {
            Ok(transient_buffer)
        } else {
            transient_buffer.request();
            Err(TexProError::Generic)
        }
    }

    pub fn from_self(&self) -> Self {
        Self::new(Arc::clone(&self.transient_buffer))
    }

    pub(crate) fn transient_buffer_sneaky(&self) -> &RwLock<TransientBuffer> {
        &self.transient_buffer
    }

    pub fn size(&self) -> Size {
        self.size
    }
}

#[derive(Default)]
pub(crate) struct TransientBufferQueue {
    queue: VecDeque<Arc<TransientBufferContainer>>,
    pub memory_threshold: Arc<AtomicUsize>,
    pub incoming_buffers: Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
    shutdown: Arc<AtomicBool>,
}

impl Display for TransientBufferQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes_memory = self.bytes_memory();
        let bytes_storage = self.bytes_storage();
        let bytes_total = bytes_memory + bytes_storage;

        let top = format!(
            "Thres: {thr}\nTotal: {tot}\nStora: {sto}\nMemor: {mem}",
            thr = self.memory_threshold.load(Ordering::Relaxed),
            tot = bytes_total,
            mem = bytes_memory,
            sto = bytes_storage
        );

        let queue = self
            .queue
            .iter()
            .map(|arc_tbc| {
                let tbc = arc_tbc.transient_buffer.read().unwrap();
                let location = if tbc.in_memory() { "MEM" } else { "STO" };
                let bytes = tbc.bytes();
                format!("{} {:5} {:p}", location, bytes, *arc_tbc)
            })
            .collect::<Vec<String>>()
            .join("\n");

        write!(f, "{}\n{}", top, queue)
    }
}

impl TransientBufferQueue {
    pub fn new(memory_threshold: usize, shutdown: Arc<AtomicBool>) -> Self {
        Self {
            queue: VecDeque::new(),
            memory_threshold: Arc::new(AtomicUsize::new(memory_threshold)),
            incoming_buffers: Arc::new(RwLock::new(Vec::new())),
            shutdown,
        }
    }

    fn handle_incoming(&mut self) {
        if let Ok(mut incoming_buffers) = self.incoming_buffers.write() {
            while let Some(tbuf_container) = incoming_buffers.pop() {
                if self
                    .queue
                    .iter()
                    .any(|tbc| Arc::ptr_eq(tbc, &tbuf_container))
                {
                    continue;
                }

                let mut push_back = false;
                if let Ok(transient_buffer) = tbuf_container.transient_buffer.read() {
                    if transient_buffer.in_memory() {
                        push_back = true;
                    } else {
                        push_back = false;
                    }
                }

                if push_back {
                    self.queue.push_back(tbuf_container);
                } else {
                    self.queue.push_front(tbuf_container);
                }
            }
        }
    }

    pub fn add_buffer(
        incoming_buffers: &Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
        buffer: Arc<TransientBufferContainer>,
    ) {
        if let Ok(mut incoming_buffers) = incoming_buffers.write() {
            incoming_buffers.push(buffer);
        }
    }

    pub fn add_slot_data(
        incoming_buffers: &Arc<RwLock<Vec<Arc<TransientBufferContainer>>>>,
        slot_data: &Arc<SlotData>,
    ) {
        if let Ok(mut incoming_buffers) = incoming_buffers.write() {
            for buf in slot_data.image.bufs() {
                incoming_buffers.push(buf);
            }
        }
    }

    /// Makes sure this queue is not the only one holding a reference to any `Arc`.
    /// Moves any retrieved `TransientBufferContainer`s to the back of the `queue`.
    /// Also makes sure it stays below its `memory_limit` by moving `TransientBufferContainer`s to
    /// storage from the front of the `queue`.
    pub fn thread_loop(&mut self) {
        loop {
            let mut bytes_in_memory = 0;

            if self.shutdown.load(Ordering::Relaxed) {
                return;
            }
            self.handle_incoming();

            for i in (0..self.queue.len()).rev() {
                if Arc::strong_count(&self.queue[i]) == 1 {
                    self.queue.remove(i);
                    continue;
                }

                let mut requested = false;
                if let Ok(transient_buffer) = self.queue[i].transient_buffer.read() {
                    if transient_buffer.in_memory() {
                        bytes_in_memory += transient_buffer.bytes();
                    } else if transient_buffer.requested() {
                        requested = true;
                    }
                }

                if requested {
                    if let Some(removed) = self.queue.remove(i) {
                        if let Ok(mut transient_buffer) = removed.transient_buffer.write() {
                            let _ = transient_buffer.move_to_memory();
                        }
                        self.queue.push_back(removed);
                    }
                }
            }

            let memory_threshold = self.memory_threshold.load(Ordering::Relaxed);
            let mut i: usize = 0;
            while bytes_in_memory > memory_threshold {
                if let Some(tbuf_container) = self.queue.get(i) {
                    let transient_buffer = &tbuf_container.transient_buffer;

                    if let Ok(mut transient_buffer) = transient_buffer.write() {
                        if let Ok(moved) = transient_buffer.move_to_storage() {
                            if moved {
                                bytes_in_memory -= transient_buffer.bytes();
                            }
                        }
                    }
                } else {
                    break;
                }

                i += 1;
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    pub fn bytes_memory(&self) -> usize {
        self.queue
            .iter()
            .map(|tbc| tbc.transient_buffer.read().unwrap())
            .filter(|tb| tb.in_memory())
            .map(|tb| tb.bytes())
            .sum()
    }

    pub fn bytes_storage(&self) -> usize {
        self.queue
            .iter()
            .map(|tbc| tbc.transient_buffer.read().unwrap())
            .filter(|tb| !tb.in_memory())
            .map(|tb| tb.bytes())
            .sum()
    }
}
