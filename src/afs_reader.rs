use std::io::{self, Seek, SeekFrom, Read};

use byteorder::{LE, ReadBytesExt};

#[derive(Clone,Copy,Debug)]
struct AfsFile {
    offset: u32,
    size: usize,
}

impl AfsFile {
    fn read_new<R>(readable: &mut R) -> io::Result<AfsFile>
        where R: Read
    {
        let offset = readable.read_u32::<LE>()?;
        let size = readable.read_u32::<LE>()?;

        Ok(AfsFile {
            offset: offset,
            size: size as usize,
        })
    }
}

#[derive(Clone,Debug)]
pub struct AfsReader<S> {
    inner: S,
    files: Vec<AfsFile>
}

impl<S> AfsReader<S>
    where S: Read + Seek
{
    pub fn new(mut inner: S) -> io::Result<AfsReader<S>> {
        inner.seek(SeekFrom::Start(0))?;

        let mut magic = [0; 4];
        inner.read_exact(&mut magic)?;
        if magic != *b"AFS\x00" {
            panic!("Bad magic");
        }

        let num_entries = inner.read_u32::<LE>()?;
        let mut files = Vec::new();

        for _ in 0..num_entries {
            files.push(AfsFile::read_new(&mut inner)?);
        }

        Ok(AfsReader {
            inner: inner,
            files: files,
        })
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn open<'a>(&'a mut self, element: usize) -> Option<io::Result<AfsEntry<'a, S>>> {
        let inner = &mut self.inner;
        self.files
            .get(element)
            .map(move |file| {
                AfsEntry::new(
                    inner,
                    file.offset as usize,
                    file.size)
            })
    }
}

pub struct AfsEntry<'a, S>
    where S: 'a
{
    file: &'a mut S,
    current: usize,
    end: usize,
}

impl<'a, S> AfsEntry<'a, S>
    where S: Seek + Read
{
    fn new(file: &mut S, start: usize, length: usize) -> io::Result<AfsEntry<S>> {
        file.seek(SeekFrom::Start(start as u64))?;
        Ok(AfsEntry {
            file: file,
            current: start,
            end: start + length,
        })
    }
}

impl<'a, S> Read for AfsEntry<'a, S>
    where S: Seek + Read
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.current >= self.end {
            Ok(0)
        }
        else if self.current + buf.len() <= self.end {
            match self.file.read(buf) {
                Ok(len) => {
                    self.current += len;
                    Ok(len)
                }
                Err(e) => { Err(e) }
            }
        }
        else {
            match self.file.read(&mut buf[..(self.end - self.current)]) {
                Ok(len) => {
                    self.current += len;
                    Ok(len)
                }
                Err(e) => { Err(e) }
            }
        }
    }
}
