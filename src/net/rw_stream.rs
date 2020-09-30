use std::{cell::UnsafeCell, io::Read, io::Write, sync::Arc, sync::Mutex};

struct UnsafeMutator<T> {
    value: UnsafeCell<T>,
}

impl<T> UnsafeMutator<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.value.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}

unsafe impl<T> Sync for UnsafeMutator<T> {}

struct ReadWrapper<R>
where
    R: Read,
{
    inner: Arc<UnsafeMutator<R>>,
}

impl<R: Read> ReadWrapper<R> {
    fn new(inner: Arc<UnsafeMutator<R>>) -> Self {
        Self { inner }
    }
}

impl<R: Read> Read for ReadWrapper<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.get_mut().read(buf)
    }
}

struct WriteWrapper<W>
where
    W: Write,
{
    inner: Arc<UnsafeMutator<W>>,
}

impl<W: Write> WriteWrapper<W> {
    fn new(inner: Arc<UnsafeMutator<W>>) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for WriteWrapper<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.get_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.get_mut().flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner.get_mut().write_all(buf)
    }
}

pub struct RwStream<T>
where
    T: Read + Write + Send + 'static,
{
    mutator: Arc<UnsafeMutator<T>>,
    pub input_stream: Arc<Mutex<dyn Read + Send>>,
    pub output_stream: Arc<Mutex<dyn Write + Send>>,
}

impl<T: Read + Write + Send + 'static> RwStream<T> {
    pub fn new(stream: T) -> Self {
        let mutator = Arc::new(UnsafeMutator::new(stream));
        let input_stream = Arc::new(Mutex::new(ReadWrapper::new(mutator.clone())));
        let output_stream = Arc::new(Mutex::new(WriteWrapper::new(mutator.clone())));

        Self {
            mutator,
            input_stream,
            output_stream,
        }
    }

    pub fn inner(&self) -> &T {
        self.mutator.get()
    }

    #[allow(clippy::mut_from_ref)]
    pub fn inner_mut(&self) -> &mut T {
        self.mutator.get_mut()
    }
}

impl<T: Read + Write + Send + 'static> Clone for RwStream<T> {
    fn clone(&self) -> Self {
        Self {
            mutator: self.mutator.clone(),
            input_stream: self.input_stream.clone(),
            output_stream: self.output_stream.clone(),
        }
    }
}
